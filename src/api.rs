use std::{error::Error, time::Duration};

use crate::{AsSocket, SockOpt};

pub struct Socket<T> {
    pub opt: SockOpt,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: AsSocket> Socket<T> {
    pub fn new() -> Socket<T> {
        Socket {
            opt: SockOpt::default(),
            _phantom: std::marker::PhantomData::default(),
        }
    }

    pub fn set_send_hwm(mut self, send_hwm: usize) -> Self {
        self.opt.send_hwm = send_hwm;
        self
    }

    pub fn set_recv_hwm(mut self, recv_hwm: usize) -> Self {
        self.opt.recv_hwm = recv_hwm;
        self
    }

    pub fn set_safe_resent_limit(mut self, safe_resend_limit: usize) -> Self {
        self.opt.safe_resend_limit = safe_resend_limit;
        self
    }

    pub fn set_max_tick_send(mut self, max_tick_send: usize) -> Self {
        self.opt.max_tick_send = max_tick_send;
        self
    }

    pub fn set_uncompleted_message_ttl(mut self, uncompleted_message_ttl: f64) -> Self {
        self.opt.uncompleted_message_ttl = Duration::from_secs_f64(uncompleted_message_ttl);
        self
    }

    pub fn set_queue_maint_ivl(mut self, queue_maint_ivl: f64) -> Self {
        self.opt.queue_maint_ivl = Duration::from_secs_f64(queue_maint_ivl);
        self
    }

    pub fn set_peer_heartbeat_ivl(mut self, peer_heartbeat_ivl: f64) -> Self {
        self.opt.peer_heartbeat_ivl = Duration::from_secs_f64(peer_heartbeat_ivl);
        self
    }

    pub fn set_peer_keepalive(mut self, peer_keepalive: f64) -> Self {
        self.opt.peer_keepalive = Duration::from_secs_f64(peer_keepalive);
        self
    }

    pub fn set_reconnect_wait(mut self, reconnect_wait: f64) -> Self {
        self.opt.reconnect_wait = Duration::from_secs_f64(reconnect_wait);
        self
    }

    pub fn set_safe_resend_ivl(mut self, safe_resend_ivl: f64) -> Self {
        self.opt.safe_resend_ivl = Duration::from_secs_f64(safe_resend_ivl);
        self
    }

    pub fn set_safe_hash_dedup_ttl(mut self, safe_hash_dedup_ttl: f64) -> Self {
        self.opt.safe_hash_dedup_ttl = Duration::from_secs_f64(safe_hash_dedup_ttl);
        self
    }

    pub fn bind(self, addr: &str) -> Result<T::Output, Box<dyn Error>> {
        T::bind(addr, self.opt)
    }

    pub fn connect(self, addr: &str) -> Result<T::Output, Box<dyn Error>> {
        T::connect(addr, self.opt)
    }
}
