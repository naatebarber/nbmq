use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io,
    net::SocketAddr,
};

use crate::{
    core::{AsSocket, Core, SockOpt},
    f,
    queue::{RecvQueue, SendQueue},
};

pub struct Dealer {
    core: Core,
    opt: SockOpt,

    unique: u64,
    peers: Vec<SocketAddr>,
    peer_set: HashSet<SocketAddr>,

    send_queues: HashMap<SocketAddr, SendQueue>,
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

        send_queue.push(data, self.unique)?;

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
                let Some(frame) = send_queue.pull() else {
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
