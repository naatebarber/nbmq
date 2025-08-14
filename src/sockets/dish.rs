use std::{error::Error, io};

use crate::{
    core::{AsSocket, Core, SockOpt},
    frame::Frame,
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

    fn send(&mut self, _data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        return Err("send not available on Dish".into());
    }

    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        if let Some((message, ..)) = self.recv_queue.pull() {
            return Ok(message);
        }

        return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        for _ in 0..self.opt.max_tick_recv {
            if let Ok(frame) = self.core.recv() {
                let Frame::DataFrame(data_frame) = frame else {
                    continue;
                };

                if let Err(_) = self.recv_queue.push(&data_frame) {
                    break;
                }
            } else {
                break;
            }
        }

        self.core.maint()?;

        return Ok(());
    }

    fn opt(&mut self) -> &mut SockOpt {
        return &mut self.opt;
    }

    fn peers(&self) -> usize {
        self.core.peers.len()
    }
}
