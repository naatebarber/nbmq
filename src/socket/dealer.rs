use std::{error::Error, net::SocketAddr};

use crate::{
    consts,
    errors::NBMQError,
    queue::{RecvQueue, SendQueue},
};

use super::socket::Socket;

pub struct Dealer {
    socket: Socket,
    unique: u64,

    peers: Vec<SocketAddr>,

    send_queue: SendQueue,
    recv_queue: RecvQueue,
}

impl Dealer {
    fn new_from(socket: Socket) -> Self {
        Self {
            socket,
            unique: 0,

            peers: vec![],

            send_queue: SendQueue::new(consts::SOCKET_DEFAULT_HWM),
            recv_queue: RecvQueue::new(consts::SOCKET_DEFAULT_HWM),
        }
    }

    pub fn bind(addr: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Dealer::new_from(Socket::bind(addr)?))
    }

    pub fn connect(addr: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Dealer::new_from(Socket::connect(addr)?))
    }

    fn select_fair_queue_peer(&self) -> Result<&SocketAddr, NBMQError> {
        let peer_ct = self.peers.len();

        if peer_ct < 1 {
            return Err(NBMQError::NoPeer);
        }

        return Ok(&self.peers[self.unique as usize % peer_ct]);
    }

    /// Send a multipart message to one or many peers
    /// - In the case of multiple peers, routing is handled in a round-robin fashion
    pub fn send_multipart(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>> {
        self.send_queue.push(data, self.unique)?;
        self.unique = self.unique.wrapping_add(1);
        if let Some(peer_update) = self.socket.update_peers() {
            self.peers = peer_update;
        }

        let peer = self.select_fair_queue_peer()?.clone();

        while let Some(frame) = self.send_queue.pull() {
            self.socket.send_peer(&frame, &peer)?;
        }

        Ok(())
    }

    /// Receive a multipart message from one or many peers
    /// - In the case of many peers, the order of received messages is first come first serve
    pub fn recv_multipart(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        loop {
            let (frame, _peer) = self.socket.recv()?;
            self.recv_queue.push(&frame)?;

            if let Some(message) = self.recv_queue.pull() {
                return Ok(message);
            }
        }
    }
}
