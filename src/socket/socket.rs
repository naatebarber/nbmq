use std::{
    collections::HashMap,
    error::Error,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    time::Instant,
};

use crate::{consts, frame::ControlFrame};

#[derive(PartialEq, Eq)]
pub enum SockMode {
    Bind,
    Connect(SocketAddr),
}

pub struct Peer {
    pub last_seen: Instant,
    pub last_sent: Instant,
}

impl Peer {
    pub fn new() -> Self {
        Self {
            last_seen: Instant::now(),
            last_sent: Instant::now(),
        }
    }
}

pub struct Socket {
    sock: UdpSocket,

    pub mode: SockMode,
    pub peer_update: bool,
    pub peers: HashMap<SocketAddr, Peer>,
}

impl Socket {
    pub fn bind(addr: &str) -> Result<Socket, Box<dyn Error>> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        Ok(Socket {
            sock: socket,

            mode: SockMode::Bind,
            peer_update: true,
            peers: HashMap::new(),
        })
    }

    pub fn connect(addr: &str) -> Result<Socket, Box<dyn Error>> {
        let sock = UdpSocket::bind("127.0.0.1:0")?;
        sock.set_nonblocking(true)?;

        sock.connect(addr)?;

        let peer = Peer::new();
        let peer_addr = SocketAddr::from_str(addr)?;
        let mut peers = HashMap::new();
        peers.insert(peer_addr.clone(), peer);

        Ok(Socket {
            sock,

            mode: SockMode::Connect(peer_addr),
            peer_update: true,
            peers,
        })
    }

    fn recv_frame(&mut self) -> Result<(Vec<u8>, SocketAddr), Box<dyn Error>> {
        let mut buffer = vec![0u8; consts::MAX_FRAME_SIZE];

        let recv_addr = match self.mode {
            SockMode::Connect(peer_addr) => {
                let bytes_recv = self.sock.recv(&mut buffer)?;
                buffer.truncate(bytes_recv);
                peer_addr
            }
            SockMode::Bind => {
                let (bytes_recv, recv_addr) = self.sock.recv_from(&mut buffer)?;
                buffer.truncate(bytes_recv);
                recv_addr
            }
        };

        let peer = self.peers.entry(recv_addr.clone()).or_insert_with(|| {
            self.peer_update = true;
            Peer::new()
        });

        peer.last_seen = Instant::now();

        if Instant::now().duration_since(peer.last_sent).as_secs_f64() > consts::PEER_HEARTBEAT_IVL
        {
            peer.last_sent = Instant::now();
            self.send_peer(&ControlFrame::Heartbeat.encode(), &recv_addr)?;
        }

        Ok((buffer, recv_addr))
    }

    fn recv_control(&mut self, data: &[u8], peer_addr: &SocketAddr) -> bool {
        let Ok(Some(control_frame)) = ControlFrame::parse(data) else {
            return false;
        };

        match control_frame {
            ControlFrame::Heartbeat => {
                let peer = self.peers.entry(peer_addr.clone()).or_insert_with(|| {
                    self.peer_update = true;
                    Peer::new()
                });

                peer.last_seen = Instant::now();

                return true;
            }
        }
    }

    pub fn recv(&mut self) -> Result<(Vec<u8>, SocketAddr), Box<dyn Error>> {
        loop {
            let (frame, peer_addr) = self.recv_frame()?;
            if !self.recv_control(&frame, &peer_addr) {
                return Ok((frame, peer_addr));
            }
        }
    }

    pub fn send_all(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match self.mode {
            SockMode::Connect(peer_addr) => {
                self.sock.send(data)?;

                let peer = self.peers.entry(peer_addr).or_insert_with(|| {
                    self.peer_update = true;
                    Peer::new()
                });

                peer.last_sent = Instant::now();
            }
            SockMode::Bind => {
                let mut drop_stale_peers = vec![];

                for (peer_addr, peer) in self.peers.iter_mut() {
                    self.sock.send_to(data, peer_addr)?;
                    peer.last_sent = Instant::now();

                    if Instant::now().duration_since(peer.last_seen).as_secs_f64()
                        > consts::PEER_KEEPALIVE
                    {
                        drop_stale_peers.push(peer_addr.clone());
                        self.peer_update = true;
                    }
                }

                drop_stale_peers.drain(..).for_each(|peer_addr| {
                    self.peers.remove(&peer_addr);
                });
            }
        }

        return Ok(());
    }

    pub fn send_peer(&mut self, data: &[u8], peer_addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
        let peer_addr = match self.mode {
            SockMode::Connect(peer_addr) => {
                self.send_all(data)?;
                peer_addr.clone()
            }
            SockMode::Bind => {
                self.sock.send_to(data, peer_addr)?;
                peer_addr.clone()
            }
        };

        let peer = self.peers.entry(peer_addr).or_insert_with(|| {
            self.peer_update = true;
            Peer::new()
        });

        peer.last_sent = Instant::now();

        if Instant::now().duration_since(peer.last_seen).as_secs_f64() > consts::PEER_KEEPALIVE {
            self.peers.remove(&peer_addr);
            self.peer_update = true;
        }

        Ok(())
    }

    pub fn update_peers(&mut self) -> Option<Vec<SocketAddr>> {
        if self.peer_update {
            self.peer_update = false;
            return Some(self.peers.keys().cloned().collect());
        }

        return None;
    }
}
