use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    time::Instant,
};

use crate::{consts, errors::NBMQError};

#[derive(PartialEq, Eq)]
pub enum SockMode {
    Bind,
    Connect,
}

#[derive(PartialEq, Eq)]
pub enum SockState {
    Bound,
    Connected,
    Disconnected,
}

pub struct Peer {
    pub bytes_sent: usize,
    pub bytes_recv: usize,
    pub last_seen: Option<Instant>,
    pub last_connect: Option<Instant>,
    pub failures: usize,
}

impl Peer {
    pub fn new() -> Self {
        Self {
            bytes_sent: 0,
            bytes_recv: 0,
            last_seen: None,
            last_connect: None,
            failures: 0,
        }
    }
}

pub struct Socket {
    sock: UdpSocket,

    pub mode: SockMode,
    pub state: SockState,
    pub created_at: Instant,
    pub peer_update: bool,

    outbound_peer: Option<(SocketAddr, Peer)>,
    inbound_peers: Option<HashMap<SocketAddr, Peer>>,
}

impl Socket {
    pub fn bind(addr: &str) -> Result<Socket, NBMQError> {
        let socket = UdpSocket::bind(addr).map_err(|e| NBMQError::BindFailed(e))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| NBMQError::BindFailed(e))?;

        Ok(Socket {
            sock: socket,
            mode: SockMode::Bind,
            state: SockState::Bound,
            created_at: Instant::now(),
            peer_update: true,

            outbound_peer: None,
            inbound_peers: Some(HashMap::new()),
        })
    }

    pub fn connect(addr: &str) -> Result<Socket, NBMQError> {
        let sock = UdpSocket::bind("127.0.0.1:0").map_err(|e| NBMQError::BindFailed(e))?;
        sock.set_nonblocking(true)
            .map_err(|e| NBMQError::BindFailed(e))?;

        let state = match sock.connect(addr) {
            Ok(()) => SockState::Connected,
            Err(e) => return Err(NBMQError::ConnectFailed(e)),
        };

        let mut peer = Peer::new();
        peer.last_connect = Some(Instant::now());
        let peer_addr = SocketAddr::from_str(addr)
            .map_err(|_| NBMQError::Internal("Could not parse provided connect addr".into()))?;

        Ok(Socket {
            sock,
            mode: SockMode::Connect,
            state,
            created_at: Instant::now(),
            peer_update: true,

            outbound_peer: Some((peer_addr, peer)),
            inbound_peers: None,
        })
    }

    pub fn reconnect(&mut self) -> Result<(), NBMQError> {
        if let (SockMode::Connect, SockState::Disconnected) = (&self.mode, &self.state) {
            let Some((peer_addr, peer)) = &mut self.outbound_peer else {
                return Err(NBMQError::NoPeer);
            };

            let now = Instant::now();
            let last_connect = peer.last_connect.unwrap_or(now.clone());
            let time_since_last_connect = now.duration_since(last_connect).as_secs_f64();

            if time_since_last_connect < consts::PEER_RECONNECT_WAIT {
                return Ok(());
            }

            peer.last_connect = Some(now);

            match self.sock.connect(*peer_addr) {
                Ok(()) => {
                    self.state = SockState::Connected;
                    return Ok(());
                }
                Err(e) => {
                    return Err(NBMQError::ConnectFailed(e));
                }
            }
        }

        return Ok(());
    }

    pub fn recv(&mut self) -> Result<Vec<u8>, NBMQError> {
        self.reconnect()?;

        let mut buffer = vec![0u8; consts::MAX_FRAME_SIZE];

        match self.mode {
            SockMode::Connect => {
                let bytes_recv = self.sock.recv(&mut buffer).map_err(|e| match e.kind() {
                    io::ErrorKind::WouldBlock => NBMQError::WouldBlock,
                    _ => NBMQError::RecvFailed(e),
                })?;

                if let Some((_, peer)) = &mut self.outbound_peer {
                    peer.bytes_recv += bytes_recv;
                    peer.last_seen = Some(Instant::now());
                }

                buffer.truncate(bytes_recv);
            }
            SockMode::Bind => {
                let (bytes_recv, peer) =
                    self.sock
                        .recv_from(&mut buffer)
                        .map_err(|e| match e.kind() {
                            io::ErrorKind::WouldBlock => NBMQError::WouldBlock,
                            _ => NBMQError::RecvFailed(e),
                        })?;

                if let Some(peers) = &mut self.inbound_peers {
                    let peer = peers.entry(peer).or_insert_with(|| {
                        self.peer_update = true;
                        Peer::new()
                    });

                    peer.bytes_recv += bytes_recv;
                    peer.last_seen = Some(Instant::now());
                }

                buffer.truncate(bytes_recv);
            }
        }

        Ok(buffer)
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), NBMQError> {
        self.reconnect()?;

        match self.mode {
            SockMode::Connect => {
                let Some((_, peer)) = &mut self.outbound_peer else {
                    return Err(NBMQError::NoPeer);
                };

                match self.sock.send(data) {
                    Ok(bytes) => {
                        peer.failures = 0;
                        peer.bytes_sent += bytes;
                        peer.last_seen = Some(Instant::now());

                        return Ok(());
                    }
                    Err(e) => {
                        peer.failures += 1;

                        return Err(match e.kind() {
                            io::ErrorKind::WouldBlock => NBMQError::WouldBlock,
                            _ => NBMQError::SendFailed(e),
                        });
                    }
                }
            }
            SockMode::Bind => {
                let Some(peers) = &mut self.inbound_peers else {
                    return Err(NBMQError::NoPeer);
                };

                for (peer_addr, peer) in peers.iter_mut() {
                    match self.sock.send_to(data, peer_addr) {
                        Ok(bytes) => {
                            peer.failures = 0;
                            peer.bytes_sent += bytes;
                            peer.last_seen = Some(Instant::now());
                        }
                        Err(e) => {
                            peer.failures += 1;

                            return Err(match e.kind() {
                                io::ErrorKind::WouldBlock => NBMQError::WouldBlock,
                                _ => NBMQError::SendFailed(e),
                            });
                        }
                    }
                }

                return Ok(());
            }
        }
    }

    pub fn send_peer(&mut self, peer_addr: &SocketAddr, data: &[u8]) -> Result<(), NBMQError> {
        self.reconnect()?;

        let peer = match self.mode {
            SockMode::Connect => return self.send(data),
            SockMode::Bind => {
                if let Some(peers) = &mut self.inbound_peers {
                    if let Some(peer) = peers.get_mut(peer_addr) {
                        peer
                    } else {
                        return Err(NBMQError::UnknownPeer);
                    }
                } else {
                    return Err(NBMQError::NoPeer);
                }
            }
        };

        match self.sock.send_to(data, peer_addr) {
            Ok(bytes) => {
                peer.failures = 0;
                peer.bytes_sent += bytes;
                peer.last_seen = Some(Instant::now());
            }
            Err(e) => {
                peer.failures += 1;
                eprintln!("failed to send : {:?}", e);
                return Err(match e.kind() {
                    io::ErrorKind::WouldBlock => NBMQError::WouldBlock,
                    _ => NBMQError::SendFailed(e),
                });
            }
        }

        return Ok(());
    }

    pub fn update_peers(&self) -> Option<Vec<SocketAddr>> {
        if !self.peer_update {
            return None;
        }

        let mut peer_addrs = vec![];

        match self.mode {
            SockMode::Connect => {
                if let Some((peer_addr, _)) = &self.outbound_peer {
                    peer_addrs.push(peer_addr.clone())
                }
            }
            SockMode::Bind => {
                if let Some(peers) = &self.inbound_peers {
                    peer_addrs.extend(peers.keys());
                }
            }
        }

        Some(peer_addrs)
    }
}
