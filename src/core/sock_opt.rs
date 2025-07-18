use crate::consts;

#[derive(Clone, Debug)]
pub struct SockOpt {
    pub peer_keepalive: f64,
    pub peer_heartbeat_ivl: f64,
}

impl Default for SockOpt {
    fn default() -> Self {
        Self {
            peer_keepalive: consts::PEER_KEEPALIVE,
            peer_heartbeat_ivl: consts::PEER_HEARTBEAT_IVL,
        }
    }
}
