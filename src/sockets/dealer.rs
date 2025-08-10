use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io,
};

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::{RecvQueue, SendQueue},
};

pub struct Dealer {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<u64>,
    peer_set: HashSet<u64>,

    send_queues: HashMap<u64, SendQueue>,
    recv_queue: RecvQueue,
}

impl Dealer {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            unique: 0,
            peers: vec![],
            peer_set: HashSet::new(),

            send_queues: HashMap::new(),
            recv_queue: RecvQueue::new(opt),
        }
    }

    fn select_fair_queue_peer(&self) -> Result<&u64, Box<dyn Error>> {
        let peer_ct = self.peers.len();

        if peer_ct < 1 {
            return Err("No peers".into());
        }

        return Ok(&self.peers[self.unique as usize % peer_ct]);
    }

    fn check_peer_update(&mut self) {
        if let Some(peer_update) = self.core.update_peers() {
            self.peers = peer_update;
            self.peer_set = HashSet::new();
            self.peer_set.extend(self.peers.iter());

            self.send_queues.retain(|k, _| self.peer_set.contains(k));
        }
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

    fn send(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.check_peer_update();
        self.unique = self.unique.wrapping_add(1);

        let peer = self.select_fair_queue_peer()?.clone();
        let send_queue = self
            .send_queues
            .entry(peer)
            .or_insert(SendQueue::new(self.opt.clone()));

        send_queue.push(peer, data, self.unique)?;

        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        if let Some((message, ..)) = self.recv_queue.pull() {
            return Ok(message);
        }

        return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        self.check_peer_update();

        for _ in 0..self.opt.max_tick_recv {
            if let Ok((frame, _, control)) = self.core.recv() {
                if let Some(_) = &control {
                    continue;
                }

                if let Err(_) = self.recv_queue.push(&frame) {
                    break;
                }
            } else {
                break;
            }
        }

        let n_per = if self.send_queues.len() > 0 {
            self.opt.max_tick_send / self.send_queues.len()
        } else {
            0
        };

        for (session_id, send_queue) in self.send_queues.iter_mut() {
            let mut ct = 0;

            while let Some(frame) = send_queue.pull() {
                if let Err(_) = self.core.send_peer(&frame, session_id) {
                    break;
                } else {
                    ct += 1;
                    if ct > n_per {
                        break;
                    }
                }
            }
        }

        self.check_peer_update();

        Ok(())
    }

    fn opt(&mut self) -> &mut SockOpt {
        return &mut self.opt;
    }

    fn peers(&self) -> usize {
        self.peers.len()
    }
}
