use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::hash::Hasher;
use std::io;
use std::time::Instant;

use crate::frame::Frame;
use crate::util;
use crate::util::hash::Fnv1a64;
use crate::{SockOpt, consts};

pub enum QueueItem {
    Frame(Vec<u8>),
    Marker,
}

pub struct SendQueue {
    opt: SockOpt,

    pub message_count: usize,
    pub frames: VecDeque<QueueItem>,

    pub sent: HashMap<u64, Vec<u8>>,
    pub exp: VecDeque<(u64, Instant, usize)>,
}

impl SendQueue {
    pub fn new(opt: SockOpt) -> Self {
        Self {
            opt,

            message_count: 0,
            frames: VecDeque::new(),

            sent: HashMap::new(),
            exp: VecDeque::new(),
        }
    }

    pub fn hash(data: &[&[u8]], nonce: u64) -> u64 {
        let mut hasher = util::hash::Fnv1a64::new();

        for part in data.iter() {
            hasher.write(part);
        }

        hasher.write(&nonce.to_be_bytes());
        hasher.finish()
    }

    pub fn len(&self) -> usize {
        self.frames.len() + self.sent.len()
    }

    pub fn push(&mut self, data: &[&[u8]], nonce: u64) -> Result<(), Box<dyn Error>> {
        if self.message_count >= self.opt.send_hwm {
            return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
        }

        let message_hash = SendQueue::hash(data, nonce);
        let message_size = data.iter().fold(0, |a, v| a + v.len());
        let parts = data.len();

        if parts > u8::MAX as usize {
            return Err("Message too long, exceeds 256 parts".into());
        }

        if message_size > u32::MAX as usize {
            return Err("Message too large, exceeds 4GB".into());
        }

        for (i, part) in data.iter().enumerate() {
            let part_size = part.len();

            let mut chunk_offset: usize = 0;

            part.chunks(consts::MAX_DATA_SIZE).for_each(|chunk| {
                let chunk_size = chunk.len();

                let frame = Frame::encode(
                    message_hash,
                    parts as u8,
                    i as u8,
                    message_size as u32,
                    part_size as u32,
                    chunk_size as u16,
                    chunk_offset as u32,
                    chunk,
                );

                chunk_offset += chunk_size;
                self.frames.push_back(QueueItem::Frame(frame));
            });
        }

        self.frames.push_back(QueueItem::Marker);
        self.message_count += 1;

        Ok(())
    }

    pub fn pull(&mut self) -> Option<Vec<u8>> {
        loop {
            let Some(m) = self.frames.pop_front() else {
                return None;
            };

            match m {
                QueueItem::Frame(f) => return Some(f),
                QueueItem::Marker => {
                    self.message_count -= 1;
                    return None;
                }
            }
        }
    }

    pub fn pull_safe(&mut self) -> Option<Vec<u8>> {
        let now = Instant::now();

        while self.exp.len() > 0
            && now.duration_since(self.exp[0].1).as_secs_f64() > self.opt.safe_resend_ivl
        {
            if let Some((hash, .., send_ct)) = self.exp.pop_front() {
                if send_ct >= self.opt.safe_resend_limit {
                    self.sent.remove(&hash);
                    continue;
                }

                if let Some(frame) = self.sent.get(&hash) {
                    self.exp.push_back((hash, now, send_ct + 1));
                    return Some(frame.clone());
                }
            } else {
                break;
            }
        }

        loop {
            let Some(m) = self.frames.pop_front() else {
                return None;
            };

            match m {
                QueueItem::Frame(f) => {
                    let mut hasher = Fnv1a64::new();
                    hasher.write(&f);
                    let hash = hasher.finish();

                    self.sent.insert(hash, f.clone());
                    self.exp.push_back((hash, Instant::now(), 0));

                    return Some(f);
                }
                QueueItem::Marker => {
                    self.message_count -= 1;
                    return None;
                }
            }
        }
    }

    pub fn confirm_safe(&mut self, hash: u64) {
        self.sent.remove(&hash);
    }
}
