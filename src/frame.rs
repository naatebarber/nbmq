use std::error::Error;

// v0.1.0
// message_hash; 8 | part_count; 1 | part_index; 1 | message_size; 4 | part_size; 4 | chunk_size; 2 | chunk_offset; 4 | data

pub const HEADER_SIZE: usize = 24;
pub const MAX_FRAME_SIZE: usize = 500;
pub const MAX_DATA_SIZE: usize = MAX_FRAME_SIZE - HEADER_SIZE;

pub struct Frame<'a> {
    pub message_hash: u64,
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
        message_hash: u64,
        part_count: u8,
        part_index: u8,
        message_size: u32,
        part_size: u32,
        chunk_size: u16,
        chunk_offset: u32,
        chunk: &[u8],
    ) -> Vec<u8> {
        let mut frame = Vec::with_capacity(HEADER_SIZE + chunk_size as usize);

        frame.extend_from_slice(&message_hash.to_be_bytes());
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
            message_hash: u64::from_be_bytes((&b_frame[0..8]).try_into()?),
            part_count: b_frame[8],
            part_index: b_frame[9],

            message_size: u32::from_be_bytes((&b_frame[10..14]).try_into()?),
            part_size: u32::from_be_bytes((&b_frame[14..18]).try_into()?),
            chunk_size: u16::from_be_bytes((&b_frame[18..20]).try_into()?),
            chunk_offset: u32::from_be_bytes((&b_frame[20..24]).try_into()?),
            chunk: &b_frame[24..],
        })
    }
}
