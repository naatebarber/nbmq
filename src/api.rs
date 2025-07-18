use std::error::Error;

use crate::{AsSocket, Dealer, SockOpt};

pub enum Socket {
    Dealer,
}

impl Socket {
    pub fn new(self) -> SocketConfig {
        SocketConfig {
            sock_type: self,
            opt: SockOpt::default(),
        }
    }
}

pub struct SocketConfig {
    sock_type: Socket,
    pub opt: SockOpt,
}

impl SocketConfig {
    pub fn bind(self, addr: &str) -> Result<impl AsSocket, Box<dyn Error>> {
        match self.sock_type {
            Socket::Dealer => Dealer::bind(addr, self.opt),
        }
    }

    pub fn connect(self, addr: &str) -> Result<impl AsSocket, Box<dyn Error>> {
        match self.sock_type {
            Socket::Dealer => Dealer::connect(addr, self.opt),
        }
    }
}
