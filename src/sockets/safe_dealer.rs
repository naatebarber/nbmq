use std::{error::Error, hash::Hasher, net::SocketAddr};

use crate::{
    core::{AsSocket, Core, SockOpt},
    frame::ControlFrame,
    queue::{RecvQueue, SendQueue},
    util::hash::Fnv1a64,
};

pub struct SafeDealer {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<SocketAddr>,

    send_queue: SendQueue,
    recv_queue: RecvQueue,
}

impl SafeDealer {
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

impl AsSocket for SafeDealer {
    type Output = SafeDealer;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(SafeDealer::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(SafeDealer::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send_multipart(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);
        if let Some(peer_update) = self.core.update_peers() {
            self.peers = peer_update;
        }

        let peer = self.select_fair_queue_peer()?.clone();

        let mut err: Option<Box<dyn Error>> = None;
        while let Some(frame) = self.send_queue.pull_safe() {
            if let Err(e) = self.core.send_peer(&frame, &peer) {
                err = Some(e);
            }
        }

        if let Some(e) = err {
            return Err(e);
        }

        Ok(())
    }

    fn recv_multipart(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        loop {
            if let Some((message, ..)) = self.recv_queue.pull_safe() {
                return Ok(message);
            }

            let (frame, peer_addr, control) = self.core.recv()?;

            if let Some(control) = control {
                match control {
                    ControlFrame::Ack(chunk) => {
                        if chunk.len() != 8 {
                            continue;
                        }

                        let mut hash_b = [0u8; 8];
                        hash_b[0..8].copy_from_slice(&chunk[0..8]);
                        let hash = u64::from_be_bytes(hash_b);

                        self.send_queue.confirm_safe(hash);
                    }
                    _ => (),
                };

                continue;
            }

            let mut hasher = Fnv1a64::new();
            hasher.write(&frame);
            let hash = hasher.finish();

            self.core.send_peer(
                &ControlFrame::Ack(hash.to_be_bytes().to_vec()).encode(),
                &peer_addr,
            )?;

            self.recv_queue.push(&frame)?;
        }
    }

    fn drain_control(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok(_) = self.recv_multipart() {
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
