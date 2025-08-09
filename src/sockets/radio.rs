use std::error::Error;

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::SendQueue,
};

pub struct Radio {
    core: Core,
    opt: SockOpt,

    unique: u64,

    send_queue: SendQueue,
}

impl Radio {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            unique: 0,

            send_queue: SendQueue::new(opt.clone()),
        }
    }
}

impl AsSocket for Radio {
    type Output = Radio;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Radio::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Radio::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);

        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        return Err("recv_multipart not available on Radio socket".into());
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        for _ in 0..self.opt.max_tick_recv {
            if let Err(_) = self.core.recv() {
                break;
            }
        }

        for _ in 0..self.opt.max_tick_send {
            let Some(frame) = self.send_queue.pull() else {
                break;
            };

            if let Err(_) = self.core.send_all(&frame) {
                break;
            }
        }

        Ok(())
    }

    fn opt(&mut self) -> &mut SockOpt {
        return &mut self.opt;
    }

    fn peers(&self) -> usize {
        self.core.peers.len()
    }
}
