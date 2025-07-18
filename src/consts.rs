pub use crate::frame::{HEADER_SIZE, MAX_DATA_SIZE, MAX_FRAME_SIZE};

pub const SOCKET_DEFAULT_HWM: usize = 1000;
pub const SAFE_RESEND_IVL: f64 = 0.2;
pub const SAFE_RESEND_LIMIT: usize = 10;
pub const UNCOMPLETED_MESSAGE_TTL: f64 = 10.;
pub const QUEUE_MAINT_IVL: f64 = 1.;
pub const PEER_HEARTBEAT_IVL: f64 = 1.;
pub const PEER_KEEPALIVE: f64 = 10.;
