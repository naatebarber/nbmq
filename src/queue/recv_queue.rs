use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    io,
    time::Instant,
};

use crate::{
    SockOpt,
    frame::{self, Frame},
};

#[derive(Clone)]
pub struct MessagePart {
    pub size: u32,

    pub assigned: u32,
    pub assigned_ranges: HashSet<(u32, u32)>,
    pub data: Vec<u8>,
}

impl MessagePart {
    pub fn new(size: u32) -> Self {
        let mut data = Vec::with_capacity(size as usize);
        data.resize(size as usize, 0);

        Self {
            size,

            assigned: 0,
            assigned_ranges: HashSet::new(),
            data,
        }
    }

    pub fn add_frame(&mut self, frame: &Frame) -> Result<bool, Box<dyn Error>> {
        if self.assigned == self.size {
            return Ok(false);
        }

        if (frame.chunk_offset + frame.chunk_size as u32) > self.size {
            return Err(Box::new(io::Error::from(io::ErrorKind::InvalidData)));
        }

        let start = frame.chunk_offset;
        let end = frame.chunk_offset + frame.chunk_size as u32;
        let range = (start as usize)..(end as usize);

        if self.assigned_ranges.insert((start, end)) {
            self.data[range].copy_from_slice(&frame.chunk);
            self.assigned += frame.chunk_size as u32;
        }

        if self.assigned == self.size {
            return Ok(true);
        }

        return Ok(false);
    }
}

pub struct IncomingMessage {
    pub size: u32,
    pub part_count: u8,
    pub completed_parts: u8,

    pub assigned: u32,
    pub parts: Vec<Option<MessagePart>>,

    pub last_modify: Instant,
}

impl IncomingMessage {
    pub fn new(size: u32, part_count: u8) -> Self {
        let mut parts = Vec::with_capacity(part_count as usize);
        parts.resize(part_count as usize, None);

        Self {
            size,
            part_count,

            assigned: 0,
            completed_parts: 0,
            parts,

            last_modify: Instant::now(),
        }
    }

    pub fn add_frame(&mut self, frame: &Frame) -> Result<bool, Box<dyn Error>> {
        let part = &mut self.parts[frame.part_index as usize]
            .get_or_insert(MessagePart::new(frame.part_size));

        if part.add_frame(&frame)? {
            self.completed_parts += 1;
            self.assigned += part.assigned;
        }

        self.last_modify = Instant::now();

        if self.completed_parts == self.part_count && self.assigned == self.size {
            return Ok(true);
        }

        return Ok(false);
    }
}

pub struct RecvQueue {
    pub opt: SockOpt,

    pub incoming: HashMap<(u64, u64), IncomingMessage>,
    pub complete: HashMap<(u64, u64), Vec<Vec<u8>>>,
    pub complete_deque: VecDeque<(u64, u64)>,

    pub last_maint: Instant,

    dedup: HashSet<(u64, u64)>,
    dedup_deque: VecDeque<((u64, u64), Instant)>,
}

impl RecvQueue {
    pub fn new(opt: SockOpt) -> Self {
        Self {
            opt,

            incoming: HashMap::new(),
            complete: HashMap::new(),
            complete_deque: VecDeque::new(),

            last_maint: Instant::now(),

            dedup: HashSet::new(),
            dedup_deque: VecDeque::new(),
        }
    }

    fn maint(&mut self) {
        let now = Instant::now();

        if now.duration_since(self.last_maint) < self.opt.queue_maint_ivl {
            return;
        }

        self.incoming
            .retain(|_, v| now.duration_since(v.last_modify) < self.opt.uncompleted_message_ttl);

        self.last_maint = now;
    }

    pub fn push(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if data.len() < frame::HEADER_SIZE {
            return Err(Box::new(io::Error::from(io::ErrorKind::InvalidData)));
        }

        let frame = Frame::parse(data)?;
        let key = (frame.session_id, frame.message_id);

        let message = match self.incoming.get_mut(&key) {
            Some(m) => m,
            None => {
                if self.incoming.len() >= self.opt.recv_hwm {
                    return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
                }

                self.incoming
                    .entry(key)
                    .or_insert(IncomingMessage::new(frame.message_size, frame.part_count))
            }
        };

        if message.add_frame(&frame)? {
            if let Some(message) = self.incoming.remove(&key) {
                if self.complete.len() < self.opt.recv_hwm {
                    let reassembly = message
                        .parts
                        .into_iter()
                        .filter_map(|x| Some(x?.data))
                        .collect::<Vec<Vec<u8>>>();

                    self.complete.entry(key).or_insert_with(|| {
                        self.complete_deque.push_back(key);
                        reassembly
                    });
                } else {
                    return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
                }
            }
        }

        Ok(())
    }

    pub fn pull(&mut self) -> Option<(Vec<Vec<u8>>, (u64, u64))> {
        self.maint();

        loop {
            let Some(key) = self.complete_deque.pop_front() else {
                return None;
            };

            if let Some(message) = self.complete.remove(&key) {
                return Some((message, key));
            }
        }
    }

    pub fn pull_safe(&mut self) -> Option<(Vec<Vec<u8>>, (u64, u64))> {
        let Some((message, key)) = self.pull() else {
            return None;
        };

        let now = Instant::now();

        while self.dedup_deque.len() > 0
            && self.dedup_deque[0].1.duration_since(now) > self.opt.safe_hash_dedup_ttl
        {
            let Some((key, ..)) = self.dedup_deque.pop_front() else {
                break;
            };
            self.dedup.remove(&key);
        }

        if self.dedup.insert(key) {
            return Some((message, key));
        }

        return None;
    }
}
