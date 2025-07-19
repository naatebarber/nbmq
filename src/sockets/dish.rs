use std::error::Error;

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::RecvQueue,
};

pub struct Dish {
    core: Core,
    opt: SockOpt,

    recv_queue: RecvQueue,
}

impl Dish {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            recv_queue: RecvQueue::new(opt),
        }
    }
}

impl AsSocket for Dish {
    type Output = Dish;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Dish::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Dish::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send_multipart(&mut self, _data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        return Err("send_multipart not available on Dish".into());
    }

    fn recv_multipart(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        loop {
            if let Some((message, ..)) = self.recv_queue.pull() {
                return Ok(message);
            }

            let (frame, .., control) = self.core.recv()?;

            if let Some(_) = control {
                continue;
            }

            self.recv_queue.push(&frame)?;
        }
    }

    fn drain_control(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(_) = self.core.recv() {
            continue;
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
