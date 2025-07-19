use std::{error::Error, net::SocketAddr};

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::{RecvQueue, SendQueue},
};

pub struct Dealer {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<SocketAddr>,

    send_queue: SendQueue,
    recv_queue: RecvQueue,
}

impl Dealer {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            unique: 0,
            peers: vec![],

            send_queue: SendQueue::new(opt.clone()),
            recv_queue: RecvQueue::new(opt),
        }
    }

    fn select_fair_queue_peer(&self) -> Result<&SocketAddr, Box<dyn Error>> {
        let peer_ct = self.peers.len();

        if peer_ct < 1 {
            return Err("No peers".into());
        }

        return Ok(&self.peers[self.unique as usize % peer_ct]);
    }
}

impl AsSocket for Dealer {
    type Output = Dealer;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Dealer::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Dealer::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send_multipart(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);
        if let Some(peer_update) = self.core.update_peers() {
            self.peers = peer_update;
        }

        let peer = self.select_fair_queue_peer()?.clone();

        let mut err: Option<Box<dyn Error>> = None;
        while let Some(frame) = self.send_queue.pull() {
            if let Err(e) = self.core.send_peer(&frame, &peer) {
                err = Some(e);
            }
        }

        if let Some(err) = err {
            return Err(err);
        }

        Ok(())
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
        self.peers.len()
    }
}
