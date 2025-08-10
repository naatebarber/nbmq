use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::{
    core::{AsSocket, Core, SockOpt},
    queue::SendQueue,
};

pub struct Radio {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<u64>,
    peer_set: HashSet<u64>,

    send_queues: HashMap<u64, SendQueue>,
}

impl Radio {
    fn new_from(core: Core, opt: SockOpt) -> Self {
        Self {
            core,
            opt: opt.clone(),

            unique: 0,
            peers: Vec::new(),
            peer_set: HashSet::new(),

            send_queues: HashMap::new(),
        }
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

impl AsSocket for Radio {
    type Output = Radio;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Radio::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(Radio::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        for session_id in self.peers.iter() {
            let send_queue = self
                .send_queues
                .entry(*session_id)
                .or_insert(SendQueue::new(self.opt.clone()));

            send_queue.push(*session_id, data, self.unique)?;
            self.unique = self.unique.wrapping_add(1);
        }

        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        return Err("recv not available on Radio socket".into());
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        self.check_peer_update();

        for _ in 0..self.opt.max_tick_recv {
            if let Err(_) = self.core.recv() {
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
        self.core.peers.len()
    }
}
