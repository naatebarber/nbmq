use std::error::Error;

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::{RecvQueue, SendQueue},
};

pub struct Radio {
    core: Core,
    opt: SockOpt,

    unique: u64,

    send_queue: SendQueue,
    recv_queue: RecvQueue,
}

impl Radio {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            unique: 0,

            send_queue: SendQueue::new(opt.clone()),
            recv_queue: RecvQueue::new(opt),
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

    fn send_multipart(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.drain_control()?;

        self.send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);

        let mut err: Option<Box<dyn Error>> = None;
        while let Some(frame) = self.send_queue.pull() {
            if let Err(e) = self.core.send_all(&frame) {
                err = Some(e);
            }
        }

        if let Some(err) = err {
            return Err(err);
        }

        Ok(())
    }

    fn recv_multipart(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        return Err("recv_multipart not available on Radio socket".into());
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

    fn peek_send_queue(&mut self) -> &mut SendQueue {
        &mut self.send_queue
    }

    fn peek_recv_queue(&mut self) -> &mut RecvQueue {
        &mut self.recv_queue
    }

    fn peers(&self) -> usize {
        self.core.peers.len()
    }
}
