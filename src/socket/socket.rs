use crate::{
    consts,
    queue::{recv_queue::RecvQueue, send_queue::SendQueue},
};
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

#[derive(PartialEq, Eq)]
pub enum PeerType {
    Inbound,
    Outbound,
}

pub struct Peer {
    pub peer_type: PeerType,

    pub bytes_sent: usize,
    pub bytes_recv: usize,
    pub last_seen: Option<Instant>,
    pub last_connect: Option<Instant>,
    pub failures: usize,
}

impl Peer {
    pub fn new(peer_type: PeerType) -> Self {
        let last_connect = match peer_type {
            PeerType::Inbound => None,
            PeerType::Outbound => Some(Instant::now()),
        };

        Self {
            peer_type,

            bytes_sent: 0,
            bytes_recv: 0,
            last_seen: None,
            last_connect,
            failures: 0,
        }
    }
}

pub struct Socket {
    sock: UdpSocket,
    pub mode: SockMode,
    pub state: SockState,
    pub created_at: Instant,

    peers: HashMap<SocketAddr, Peer>,
}

impl Socket {
    pub fn bind(addr: &str) -> Result<Socket, Box<dyn Error>> {
        Ok(Socket {
            sock: UdpSocket::bind(addr)?,
            mode: SockMode::Bind,
            state: SockState::Bound,
            created_at: Instant::now(),

            peers: HashMap::new(),
        })
    }

    pub fn connect(addr: &str) -> Result<Socket, Box<dyn Error>> {
        let sock = UdpSocket::bind("127.0.0.1:0")?;
        let state = match sock.connect(addr) {
            Ok(()) => SockState::Connected,
            Err(_) => SockState::Disconnected,
        };

        let mut peers = HashMap::new();
        let peer_addr = SocketAddr::from_str(addr)?;

        peers.insert(peer_addr, Peer::new(PeerType::Outbound));

        Ok(Socket {
            sock,
            mode: SockMode::Connect,
            state,
            created_at: Instant::now(),

            peers,
        })
    }

    pub fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mode == SockMode::Bind {
            return Ok(());
        }

        if self.state == SockState::Connected {
            return Ok(());
        }

        for (peer_addr, peer) in self.peers.iter_mut() {
            if peer.peer_type == PeerType::Outbound {
                if let Some(last_connect) = peer.last_connect {
                    let time_since_last_connect =
                        Instant::now().duration_since(last_connect).as_secs_f64();
                    if time_since_last_connect < consts::PEER_RECONNECT_WAIT {
                        break;
                    }
                }

                match self.sock.connect(peer_addr) {
                    Ok(()) => {
                        self.state = SockState::Connected;
                    }
                    Err(e) => {
                        return Err(Box::new(e));
                    }
                }

                break;
            }
        }

        Ok(())
    }

    pub fn recv(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.reconnect()?;

        let mut buffer = vec![0u8; consts::MAX_FRAME_SIZE];

        let (bytes_recv, peer) = self.sock.recv_from(&mut buffer)?;
        let peer = self
            .peers
            .entry(peer)
            .or_insert(Peer::new(PeerType::Inbound));
        peer.bytes_recv += bytes_recv;
        peer.last_seen = Some(Instant::now());

        buffer.truncate(bytes_recv);

        Ok(buffer)
    }

    pub fn send_peer(&mut self, peer_addr: &SocketAddr, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.reconnect()?;

        let Some(peer) = self.peers.get_mut(peer_addr) else {
            return Err("no such peer".into());
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
