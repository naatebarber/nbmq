use crate::consts;
use std::{
    collections::HashMap,
    error::Error,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    time::Instant,
};

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

    outbound_peer: Option<(SocketAddr, Peer)>,
    inbound_peers: Option<HashMap<SocketAddr, Peer>>,
}

impl Socket {
    pub fn bind(addr: &str) -> Result<Socket, Box<dyn Error>> {
        Ok(Socket {
            sock: UdpSocket::bind(addr)?,
            mode: SockMode::Bind,
            state: SockState::Bound,
            created_at: Instant::now(),

            outbound_peer: None,
            inbound_peers: Some(HashMap::new()),
        })
    }

    pub fn connect(addr: &str) -> Result<Socket, Box<dyn Error>> {
        let sock = UdpSocket::bind("127.0.0.1:0")?;
        let state = match sock.connect(addr) {
            Ok(()) => SockState::Connected,
            Err(_) => SockState::Disconnected,
        };

        let mut peer = Peer::new();
        peer.last_connect = Some(Instant::now());
        let peer_addr = SocketAddr::from_str(addr)?;

        Ok(Socket {
            sock,
            mode: SockMode::Connect,
            state,
            created_at: Instant::now(),

            outbound_peer: Some((peer_addr, peer)),
            inbound_peers: None,
        })
    }

    pub fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let (SockMode::Connect, SockState::Disconnected) = (&self.mode, &self.state) {
            let Some((peer_addr, peer)) = &mut self.outbound_peer else {
                return Err("connect mode socket has no peer".into());
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
                    return Err(Box::new(e));
                }
            }
        }

        return Ok(());
    }

    pub fn recv(&mut self) -> Result<(SocketAddr, Vec<u8>), Box<dyn Error>> {
        self.reconnect()?;

        let mut buffer = vec![0u8; consts::MAX_FRAME_SIZE];
        let (bytes_recv, peer) = self.sock.recv_from(&mut buffer)?;

        match self.mode {
            SockMode::Connect => {
                if let Some((_, peer)) = &mut self.outbound_peer {
                    peer.bytes_recv += bytes_recv;
                    peer.last_seen = Some(Instant::now());
                }
            }
            SockMode::Bind => {
                if let Some(peers) = &mut self.inbound_peers {
                    let peer = peers.entry(peer).or_insert(Peer::new());
                    peer.bytes_recv += bytes_recv;
                    peer.last_seen = Some(Instant::now());
                }
            }
        }

        buffer.truncate(bytes_recv);

        Ok((peer, buffer))
    }

    pub fn send_peer(&mut self, peer_addr: &SocketAddr, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.reconnect()?;

        let peer = match self.mode {
            SockMode::Connect => {
                if let Some((outbound_peer_addr, peer)) = &mut self.outbound_peer {
                    if outbound_peer_addr == peer_addr {
                        peer
                    } else {
                        return Err("no such peer".into());
                    }
                } else {
                    return Err("no peer".into());
                }
            }
            SockMode::Bind => {
                if let Some(peers) = &mut self.inbound_peers {
                    if let Some(peer) = peers.get_mut(peer_addr) {
                        peer
                    } else {
                        return Err("no such peer".into());
                    }
                } else {
                    return Err("no peers".into());
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
                return Err(Box::new(e));
            }
        }

        return Ok(());
    }
}
