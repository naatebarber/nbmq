pub use crate::frame::{HEADER_SIZE, MAX_DATA_SIZE, MAX_FRAME_SIZE};

pub const SOCKET_DEFAULT_HWM: usize = 1000;
pub const PEER_RETRY_BACKOFF: f64 = 3.;
pub const PEER_RECONNECT_WAIT: f64 = 1.;
