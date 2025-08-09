use std::{
    collections::{HashMap, HashSet},
    error::Error,
    hash::Hasher,
    io,
    net::SocketAddr,
};

use crate::{
    core::{AsSocket, Core, SockOpt},
    f,
    frame::ControlFrame,
    queue::{RecvQueue, SendQueue},
    util::hash::Fnv1a64,
};

pub struct SafeDealer {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<SocketAddr>,
    peer_set: HashSet<SocketAddr>,

    send_queues: HashMap<SocketAddr, SendQueue>,
    recv_queue: RecvQueue,
}

impl SafeDealer {
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

    fn select_fair_queue_peer(&self) -> Result<&SocketAddr, Box<dyn Error>> {
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

impl AsSocket for SafeDealer {
    type Output = SafeDealer;

    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(SafeDealer::new_from(Core::bind(addr, opt.clone())?, opt))
    }

    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>> {
        Ok(SafeDealer::new_from(Core::connect(addr, opt.clone())?, opt))
    }

    fn send(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.check_peer_update();

        let peer = self.select_fair_queue_peer()?.clone();
        let send_queue = self
            .send_queues
            .entry(peer)
            .or_insert(SendQueue::new(self.opt.clone()));

        send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);

        let mut err: Option<Box<dyn Error>> = None;
        while let Some(frame) = send_queue.pull_safe() {
            if let Err(e) = self.core.send_peer(&frame, &peer) {
                err = Some(e);
            }
        }

        if let Some(e) = err {
            return Err(e);
        }

        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        if let Some((message, ..)) = self.recv_queue.pull_safe() {
            return Ok(message);
        }

        return Err(Box::new(io::Error::from(io::ErrorKind::WouldBlock)));
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        for _ in 0..self.opt.max_tick_recv {
            let Ok((frame, peer, control)) = self.core.recv() else {
                break;
            };

            if let Some(control) = control {
                match control {
                    ControlFrame::Ack(chunk) => {
                        if chunk.len() != 8 {
                            continue;
                        }

                        let mut hash_b = [0u8; 8];
                        hash_b[0..8].copy_from_slice(&chunk[0..8]);
                        let hash = u64::from_be_bytes(hash_b);

                        let send_queue = self
                            .send_queues
                            .entry(peer)
                            .or_insert(SendQueue::new(self.opt.clone()));

                        send_queue.confirm_safe(hash);
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
                &peer,
            )?;

            self.recv_queue.push(&frame)?;
        }

        let queue_sizes = self
            .send_queues
            .iter()
            .map(|(p, q)| (p.clone(), q.len()))
            .collect::<Vec<(SocketAddr, usize)>>();
        let distribution =
            f::softmax(&queue_sizes.iter().map(|x| x.1 as f64).collect::<Vec<f64>>());
        let frames_per_queue = distribution
            .iter()
            .map(|x| f64::floor(x * self.opt.max_tick_send as f64) as usize)
            .collect::<Vec<usize>>();

        for i in 0..frames_per_queue.len() {
            let peer = queue_sizes[i].0;
            let limit = frames_per_queue[i];
            let Some(send_queue) = self.send_queues.get_mut(&peer) else {
                continue;
            };

            for _ in 0..limit {
                let Some(frame) = send_queue.pull_safe() else {
                    break;
                };

                if let Err(_) = self.core.send_peer(&frame, &peer) {
                    break;
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
