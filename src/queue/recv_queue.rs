use std::{
    collections::{HashMap, VecDeque},
    error::Error,
};

use crate::{consts, frame::Frame};

#[derive(Clone)]
pub struct MessagePart {
    pub size: u32,

    pub assigned: u32,
    pub data: Vec<u8>,
}

impl MessagePart {
    pub fn new(size: u32) -> Self {
        let mut data = Vec::with_capacity(size as usize);
        data.resize(size as usize, 0);

        Self {
            size,

            assigned: 0,
            data,
        }
    }

    pub fn add_frame(&mut self, frame: &Frame) -> Result<bool, Box<dyn Error>> {
        if (frame.chunk_offset + frame.chunk_size) as u32 > self.size {
            return Err("invalid chunk bounds".into());
        }

        let range =
            (frame.chunk_offset as usize)..((frame.chunk_offset + frame.chunk_size) as usize);
        self.data[range].copy_from_slice(&frame.chunk);
        self.assigned += frame.chunk_size as u32;

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
        }
    }

    pub fn add_frame(&mut self, frame: &Frame) -> Result<bool, Box<dyn Error>> {
        let part = &mut self.parts[frame.part_index as usize]
            .get_or_insert(MessagePart::new(frame.part_size));
        if part.add_frame(&frame)? {
            self.completed_parts += 1;
        }

        self.assigned += frame.chunk_size as u32;

        if self.completed_parts == self.part_count && self.assigned == self.size {
            return Ok(true);
        }

        return Ok(false);
    }
}

pub struct RecvQueue {
    pub high_water_mark: usize,

    pub incoming: HashMap<u64, IncomingMessage>,
    pub complete: VecDeque<Vec<Vec<u8>>>,
}

impl RecvQueue {
    pub fn new(high_water_mark: usize) -> Self {
        Self {
            high_water_mark,

            incoming: HashMap::new(),
            complete: VecDeque::new(),
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if data.len() < consts::HEADER_SIZE {
            return Err("message too short".into());
        }

        let frame = Frame::parse(data)?;

        let message = match self.incoming.get_mut(&frame.message_hash) {
            Some(m) => m,
            None => {
                if self.incoming.len() >= self.high_water_mark {
                    return Err("incoming message high water mark reached".into());
                }

                self.incoming
                    .entry(frame.message_hash)
                    .or_insert(IncomingMessage::new(frame.message_size, frame.part_count))
            }
        };

        if message.add_frame(&frame)? {
            if let Some(message) = self.incoming.remove(&frame.message_hash) {
                if self.complete.len() < self.high_water_mark {
                    let reassembly = message
                        .parts
                        .into_iter()
                        .filter_map(|x| Some(x?.data))
                        .collect::<Vec<Vec<u8>>>();
                    self.complete.push_back(reassembly);
                } else {
                    return Err("completed message high water mark reached".into());
                }
            }
        }

        Ok(())
    }

    pub fn pull(&mut self) -> Option<Vec<Vec<u8>>> {
        self.complete.pop_front()
    }
}
