use std::collections::VecDeque;
use std::hash::Hasher;

use crate::frame::Frame;
use crate::util;
use crate::{consts, errors::NBMQError};

enum QueueItem {
    Frame(Vec<u8>),
    Marker,
}

pub struct SendQueue {
    high_water_mark: usize,

    message_count: usize,
    frames: VecDeque<QueueItem>,
}

impl SendQueue {
    pub fn new(high_water_mark: usize) -> Self {
        Self {
            high_water_mark,

            message_count: 0,
            frames: VecDeque::new(),
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

    pub fn push(&mut self, data: &[&[u8]], nonce: u64) -> Result<(), NBMQError> {
        if self.message_count >= self.high_water_mark {
            return Err(NBMQError::HighWaterMark);
        }

        let message_hash = SendQueue::hash(data, nonce);
        let message_size = data.iter().fold(0, |a, v| a + v.len());
        let parts = data.len();

        if parts > u8::MAX as usize {
            return Err(NBMQError::MessageTooLong);
        }

        if message_size > u32::MAX as usize {
            return Err(NBMQError::MessageTooLarge);
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
}
