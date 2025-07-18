use crate::consts;

#[derive(Clone, Debug)]
pub struct SockOpt {
    pub send_hwm: usize,
    pub recv_hwm: usize,
    pub safe_resend_ivl: f64,
    pub safe_resend_limit: usize,
    pub uncompleted_message_ttl: f64,
    pub queue_maint_ivl: f64,
    pub peer_keepalive: f64,
    pub peer_heartbeat_ivl: f64,
}

impl Default for SockOpt {
    fn default() -> Self {
        Self {
            send_hwm: consts::SOCKET_DEFAULT_HWM,
            recv_hwm: consts::SOCKET_DEFAULT_HWM,
            safe_resend_ivl: consts::SAFE_RESEND_IVL,
            safe_resend_limit: consts::SAFE_RESEND_LIMIT,
            uncompleted_message_ttl: consts::UNCOMPLETED_MESSAGE_TTL,
            queue_maint_ivl: consts::QUEUE_MAINT_IVL,
            peer_keepalive: consts::PEER_KEEPALIVE,
            peer_heartbeat_ivl: consts::PEER_HEARTBEAT_IVL,
        }
    }
}
