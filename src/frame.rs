use std::{error::Error, hash::Hasher};

use crate::hash::Fnv1a64;

// v0.2.0 DataFrame
// | version; 1
// | kind; 1
// | session_id; 8
// | message_id; 8
// | part_count; 1
// | part_index; 1
// | message_size; 4
// | part_size; 4
// | chunk_size; 2
// | chunk_offset; 4
// | data
//
// HEADER = 34b

// v0.2.0 ControlFrame
// | version; 1
// | kind; 1
// | session_id: 8
// | data
//
// CONTROL_HEADER = 10b

pub const VERSION: u8 = 1;
pub const DATA_HEADER_SIZE: usize = 34;
pub const CONTROL_HEADER_SIZE: usize = 10;
pub const MAX_FRAME_SIZE: usize = 500;
pub const MAX_DATA_SIZE: usize = MAX_FRAME_SIZE - DATA_HEADER_SIZE;

pub struct DataFrame {
    pub version: u8,
    pub kind: u8,
    pub session_id: u64,
    pub message_id: u64,

    pub part_count: u8,
    pub part_index: u8,
    pub message_size: u32,
    pub part_size: u32,
    pub chunk_size: u16,
    pub chunk_offset: u32,
    pub chunk: Vec<u8>,
}

impl DataFrame {
    pub fn encode(
        kind: u8,
        session_id: u64,
        message_id: u64,
        part_count: u8,
        part_index: u8,
        message_size: u32,
        part_size: u32,
        chunk_size: u16,
        chunk_offset: u32,
        chunk: &[u8],
    ) -> Vec<u8> {
        let mut frame = Vec::with_capacity(DATA_HEADER_SIZE + chunk_size as usize);

        frame.push(VERSION);
        frame.push(kind);
        frame.extend_from_slice(&session_id.to_be_bytes());
        frame.extend_from_slice(&message_id.to_be_bytes());

        frame.extend_from_slice(&[part_count]);
        frame.extend_from_slice(&[part_index]);
        frame.extend_from_slice(&message_size.to_be_bytes());
        frame.extend_from_slice(&part_size.to_be_bytes());
        frame.extend_from_slice(&chunk_size.to_be_bytes());
        frame.extend_from_slice(&chunk_offset.to_be_bytes());
        frame.extend_from_slice(&chunk);

        frame
    }

    pub fn parse(buf: &[u8]) -> Result<Option<DataFrame>, Box<dyn Error>> {
        if buf.len() < DATA_HEADER_SIZE {
            return Ok(None);
        }

        if buf[0] != VERSION {
            return Ok(None);
        }

        Ok(Some(DataFrame {
            version: buf[0],
            kind: buf[1],
            session_id: u64::from_be_bytes((&buf[2..10]).try_into()?),
            message_id: u64::from_be_bytes((&buf[10..18]).try_into()?),
            part_count: buf[18],
            part_index: buf[19],

            message_size: u32::from_be_bytes((&buf[20..24]).try_into()?),
            part_size: u32::from_be_bytes((&buf[24..28]).try_into()?),
            chunk_size: u16::from_be_bytes((&buf[28..30]).try_into()?),
            chunk_offset: u32::from_be_bytes((&buf[30..34]).try_into()?),
            chunk: buf[34..].to_vec(),
        }))
    }

    pub fn hash(&self) -> u64 {
        let buffer = DataFrame::encode(
            self.kind,
            self.session_id,
            self.message_id,
            self.part_count,
            self.part_index,
            self.message_size,
            self.part_size,
            self.chunk_size,
            self.chunk_offset,
            &self.chunk,
        );

        let mut hasher = Fnv1a64::new();
        hasher.write(&buffer);
        hasher.finish()
    }
}

pub enum ControlFrame {
    Connect,
    Connected(u64),
    Disconnected(u64),
    Heartbeat(u64),
    Ack((u64, Vec<u8>)),
}

impl ControlFrame {
    fn _enc(session: u64, kind: u8, data: &[u8]) -> Vec<u8> {
        let mut cframe = Vec::with_capacity(CONTROL_HEADER_SIZE + data.len());

        cframe.push(VERSION);
        cframe.push(kind);
        cframe.extend_from_slice(&session.to_be_bytes());
        cframe.extend_from_slice(data);

        cframe
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Connect => ControlFrame::_enc(0, 1, &[]),
            Self::Connected(session) => ControlFrame::_enc(*session, 2, &[]),
            Self::Disconnected(session) => ControlFrame::_enc(*session, 3, &[]),
            Self::Heartbeat(session) => ControlFrame::_enc(*session, 4, &[]),
            Self::Ack((session, chunk)) => ControlFrame::_enc(*session, 5, chunk),
        }
    }

    pub fn parse(buf: &[u8]) -> Result<Option<ControlFrame>, Box<dyn Error>> {
        if buf.len() < CONTROL_HEADER_SIZE {
            return Ok(None);
        }

        let kind = buf[1];

        Ok(match kind {
            1 => Some(ControlFrame::Connect),
            2 => Some(ControlFrame::Connected(u64::from_be_bytes(
                buf[2..10].try_into()?,
            ))),
            3 => Some(ControlFrame::Disconnected(u64::from_be_bytes(
                buf[2..10].try_into()?,
            ))),
            4 => Some(ControlFrame::Heartbeat(u64::from_be_bytes(
                buf[2..10].try_into()?,
            ))),
            5 => Some(ControlFrame::Ack((
                u64::from_be_bytes(buf[2..10].try_into()?),
                buf[CONTROL_HEADER_SIZE..].to_vec(),
            ))),
            _ => None,
        })
    }
}

pub enum Frame {
    ControlFrame(ControlFrame),
    DataFrame(DataFrame),
}

impl Frame {
    pub fn parse(buf: &[u8]) -> Result<Option<Frame>, Box<dyn Error>> {
        if buf.len() < 2 {
            return Ok(None);
        }

        if buf[0] != VERSION {
            return Ok(None);
        }

        let kind = buf[1];

        match kind {
            0 => match DataFrame::parse(buf)? {
                Some(data_frame) => Ok(Some(Frame::DataFrame(data_frame))),
                None => Ok(None),
            },
            _ => match ControlFrame::parse(buf)? {
                Some(control_frame) => Ok(Some(Frame::ControlFrame(control_frame))),
                None => Ok(None),
            },
        }
    }
}
