use std::error::Error;

// v0.1.0
// | message_hash; 8
// | part_count; 1
// | part_index; 1
// | message_size; 4
// | part_size; 4
// | chunk_size; 2
// | chunk_offset; 4
// | data
//
// HEADER = 24b

// v0.2.0
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

pub const VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 34;
pub const MAX_FRAME_SIZE: usize = 500;
pub const MAX_DATA_SIZE: usize = MAX_FRAME_SIZE - HEADER_SIZE;

pub struct Frame<'a> {
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
    pub chunk: &'a [u8],
}

impl Frame<'_> {
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
        let mut frame = Vec::with_capacity(HEADER_SIZE + chunk_size as usize);

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

    pub fn parse<'a>(b_frame: &'a [u8]) -> Result<Frame<'a>, Box<dyn Error>> {
        Ok(Frame {
            version: b_frame[0],
            kind: b_frame[1],
            session_id: u64::from_be_bytes((&b_frame[2..10]).try_into()?),
            message_id: u64::from_be_bytes((&b_frame[10..18]).try_into()?),
            part_count: b_frame[18],
            part_index: b_frame[19],

            message_size: u32::from_be_bytes((&b_frame[20..24]).try_into()?),
            part_size: u32::from_be_bytes((&b_frame[24..28]).try_into()?),
            chunk_size: u16::from_be_bytes((&b_frame[28..30]).try_into()?),
            chunk_offset: u32::from_be_bytes((&b_frame[30..34]).try_into()?),
            chunk: &b_frame[34..],
        })
    }
}

pub enum ControlFrame {
    Connect,
    Connected(u64),
    Disconnected(u64),
    Reconnect(u64),
    Heartbeat(u64),
    Ack((u64, Vec<u8>)),
}

impl ControlFrame {
    fn _encode_one(session: u64, kind: u8, data: &[u8]) -> Vec<u8> {
        Frame::encode(kind, session, 0, 0, 0, 0, 0, 0, 0, &data)
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Connect => ControlFrame::_encode_one(0, 1, &[]),
            Self::Connected(session) => ControlFrame::_encode_one(*session, 2, &[]),
            Self::Disconnected(session) => ControlFrame::_encode_one(*session, 3, &[]),
            Self::Reconnect(session) => ControlFrame::_encode_one(*session, 4, &[]),
            Self::Heartbeat(session) => ControlFrame::_encode_one(*session, 5, &[]),
            Self::Ack((session, chunk)) => ControlFrame::_encode_one(*session, 6, chunk),
        }
    }

    pub fn parse(frame: &[u8]) -> Result<Option<ControlFrame>, Box<dyn Error>> {
        let kind = frame[1];

        Ok(match kind {
            1 => Some(ControlFrame::Connect),
            2 => Some(ControlFrame::Connected(u64::from_be_bytes(
                frame[2..10].try_into()?,
            ))),
            3 => Some(ControlFrame::Disconnected(u64::from_be_bytes(
                frame[2..10].try_into()?,
            ))),
            4 => Some(ControlFrame::Reconnect(u64::from_be_bytes(
                frame[2..10].try_into()?,
            ))),
            5 => Some(ControlFrame::Heartbeat(u64::from_be_bytes(
                frame[2..10].try_into()?,
            ))),
            6 => Some(ControlFrame::Ack((
                u64::from_be_bytes(frame[2..10].try_into()?),
                frame[HEADER_SIZE..].to_vec(),
            ))),
            _ => None,
        })
    }
}
