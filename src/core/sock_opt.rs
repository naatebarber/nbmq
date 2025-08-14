use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SockOpt {
    pub send_hwm: usize,
    pub recv_hwm: usize,
    pub safe_resend_limit: usize,
    pub max_tick_send: usize,
    pub max_tick_recv: usize,
    pub uncompleted_message_ttl: Duration,
    pub queue_maint_ivl: Duration,
    pub peer_heartbeat_ivl: Duration,
    pub peer_keepalive: Duration,
    pub reconnect_wait: Duration,
    pub safe_resend_ivl: Duration,
    pub safe_hash_dedup_ttl: Duration,
}

impl Default for SockOpt {
    fn default() -> Self {
        Self {
            send_hwm: 1000,
            recv_hwm: 1000,
            safe_resend_limit: 10,
            max_tick_send: 1000,
            max_tick_recv: 1000,
            uncompleted_message_ttl: Duration::from_secs_f64(10.),
            queue_maint_ivl: Duration::from_secs_f64(1.),
            peer_keepalive: Duration::from_secs_f64(10.),
            peer_heartbeat_ivl: Duration::from_secs_f64(1.),
            reconnect_wait: Duration::from_secs_f64(5.),
            safe_resend_ivl: Duration::from_secs_f64(0.2),
            safe_hash_dedup_ttl: Duration::from_secs_f64(1.0),
        }
    }
}
