use std::{
    collections::HashMap,
    error::Error,
    hash::Hasher,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    time::Instant,
};

use super::sock_opt::SockOpt;
use crate::{
    frame::{self, ControlFrame, Frame},
    hash::Fnv1a64,
    random::XORShift,
    ts::get_ts_u64,
};

#[derive(PartialEq, Eq)]
pub struct ConnectStatus {
    addr: SocketAddr,
    session: u64,
    last_reconnect: Instant,
}

#[derive(PartialEq, Eq)]
pub enum SockMode {
    Bind,
    Connect(ConnectStatus),
}

pub struct Peer {
    pub addr: SocketAddr,
    pub last_seen: Instant,
    pub last_sent: Instant,
}

impl Peer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            last_seen: Instant::now(),
            last_sent: Instant::now(),
        }
    }
}

pub struct Core {
    sock: UdpSocket,
    opt: SockOpt,
    rng: XORShift,

    pub mode: SockMode,
    pub peer_update: bool,
    pub peers: HashMap<u64, Peer>,
}

impl Core {
    pub fn bind(addr: &str, opt: SockOpt) -> Result<Core, Box<dyn Error>> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        Ok(Core {
            sock: socket,
            opt,
            rng: XORShift::new(get_ts_u64()),

            mode: SockMode::Bind,

            peer_update: true,
            peers: HashMap::new(),
        })
    }

    pub fn connect(addr: &str, opt: SockOpt) -> Result<Core, Box<dyn Error>> {
        let mut sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.set_nonblocking(true)?;

        let peer_addr = SocketAddr::from_str(addr)?;
        Core::connect_socket(&mut sock, &peer_addr)?;

        let peers = HashMap::new();

        Ok(Core {
            sock,
            opt,
            rng: XORShift::new(get_ts_u64()),

            mode: SockMode::Connect(ConnectStatus {
                addr: peer_addr,
                session: 0,
                last_reconnect: Instant::now(),
            }),

            peer_update: true,
            peers,
        })
    }

    fn connect_socket(sock: &mut UdpSocket, peer_addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
        sock.connect(peer_addr)?;
        sock.send(&ControlFrame::Connect.encode())?;

        Ok(())
    }

    fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let SockMode::Connect(ConnectStatus {
            addr,
            session,
            last_reconnect,
        }) = &mut self.mode
        {
            if let None = self.peers.get(&session) {
                let now = Instant::now();

                if now.duration_since(*last_reconnect) > self.opt.reconnect_wait {
                    *last_reconnect = now;
                    Core::connect_socket(&mut self.sock, &addr)?;
                }
            }
        }

        return Ok(());
    }

    fn recv_buffer(&mut self) -> Result<(Vec<u8>, SocketAddr), Box<dyn Error>> {
        let mut buffer = vec![0u8; frame::MAX_FRAME_SIZE];

        let recv_addr = match &mut self.mode {
            SockMode::Connect(ConnectStatus { addr, .. }) => {
                let (bytes_recv, recv_addr) = self.sock.recv_from(&mut buffer)?;
                buffer.truncate(bytes_recv);

                if *addr != recv_addr {
                    *addr = recv_addr;
                }

                recv_addr
            }
            SockMode::Bind => {
                let (bytes_recv, recv_addr) = self.sock.recv_from(&mut buffer)?;
                buffer.truncate(bytes_recv);
                recv_addr
            }
        };

        Ok((buffer, recv_addr))
    }

    fn control(
        &mut self,
        control_frame: &ControlFrame,
        peer_addr: &SocketAddr,
    ) -> Result<bool, Box<dyn Error>> {
        match control_frame {
            ControlFrame::Connect => {
                println!("CONNECT");
                let mut hasher = Fnv1a64::new();
                hasher.write(&self.rng.sample().to_be_bytes());
                hasher.write(peer_addr.to_string().as_bytes());
                let session_id = hasher.finish();

                self.peers.insert(
                    session_id,
                    Peer {
                        addr: peer_addr.clone(),
                        last_sent: Instant::now(),
                        last_seen: Instant::now(),
                    },
                );

                self.send_direct(&ControlFrame::Connected(session_id).encode(), peer_addr)?;
                self.peer_update = true;
            }
            ControlFrame::Connected(session_id) => {
                if let SockMode::Connect(ConnectStatus { session, .. }) = &mut self.mode {
                    *session = *session_id;
                    self.peers.drain();
                }

                self.peers.insert(
                    *session_id,
                    Peer {
                        addr: peer_addr.clone(),
                        last_seen: Instant::now(),
                        last_sent: Instant::now(),
                    },
                );

                self.send_direct(&ControlFrame::Heartbeat(*session_id).encode(), peer_addr)?;
                self.peer_update = true;
            }
            ControlFrame::Disconnected(session_id) => {
                if let SockMode::Connect(ConnectStatus { session, .. }) = &mut self.mode {
                    *session = 0;
                    self.peers.drain();
                }

                self.peers.remove(&session_id);

                self.send_direct(&ControlFrame::Connect.encode(), peer_addr)?;
                self.peer_update = true;
            }
            ControlFrame::Heartbeat(session_id) => {
                if let Some(peer) = self.peers.get_mut(&session_id) {
                    peer.last_seen = Instant::now();
                    if peer.addr != *peer_addr {
                        peer.addr = *peer_addr;
                    }
                } else {
                    self.send_direct(&ControlFrame::Disconnected(*session_id).encode(), peer_addr)?;
                };
            }
            // ACKs are control frames, but are handled by the messaging layer.
            // Forward them with return true.
            _ => return Ok(true),
        }

        // All other things handled, don't forward.
        return Ok(false);
    }

    pub fn recv(&mut self) -> Result<Frame, Box<dyn Error>> {
        loop {
            let (buffer, addr) = self.recv_buffer()?;

            let Ok(Some(frame)) = Frame::parse(&buffer) else {
                continue;
            };

            match &frame {
                Frame::DataFrame(data_frame) => {
                    if let Some(peer) = self.peers.get_mut(&data_frame.session_id) {
                        peer.last_seen = Instant::now();
                        if peer.addr != addr {
                            peer.addr = addr;
                        }

                        if Instant::now().duration_since(peer.last_sent)
                            > self.opt.peer_heartbeat_ivl
                        {
                            peer.last_sent = Instant::now();
                            if let Err(_) = self.send_direct(
                                &ControlFrame::Heartbeat(data_frame.session_id).encode(),
                                &addr,
                            ) {
                                self.reconnect()?;
                            }
                        }
                    } else {
                        continue;
                    }
                }
                Frame::ControlFrame(control_frame) => match self.control(control_frame, &addr) {
                    Ok(forward) => {
                        if !forward {
                            continue;
                        }
                    }
                    Err(_) => {
                        self.reconnect()?;
                        continue;
                    }
                },
            }

            return Ok(frame);
        }
    }

    pub fn send_direct(
        &mut self,
        data: &[u8],
        peer_addr: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        match self.mode {
            SockMode::Connect(ConnectStatus { addr, .. }) => {
                debug_assert_eq!(*peer_addr, addr);
                self.sock.send(data)?;
            }
            SockMode::Bind => {
                self.sock.send_to(data, peer_addr)?;
            }
        }

        Ok(())
    }

    pub fn send_peer(&mut self, data: &[u8], session_id: &u64) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();

        let Some(peer) = self.peers.get_mut(session_id) else {
            return Err("No such peer".into());
        };

        peer.last_sent = now;
        let addr = peer.addr;

        if now.duration_since(peer.last_seen) > self.opt.peer_keepalive {
            self.peers.remove(session_id);
            return Ok(());
        }

        match self.mode {
            SockMode::Connect(_) => {
                self.sock.send(data)?;
            }
            SockMode::Bind => {
                self.sock.send_to(data, addr)?;
            }
        }

        Ok(())
    }

    pub fn maint(&mut self) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();

        let mut send_heartbeat = Vec::with_capacity(self.peers.len());
        let mut prune = Vec::with_capacity(self.peers.len());

        self.peers.iter().for_each(|(session_id, peer)| {
            if now.duration_since(peer.last_seen) > self.opt.peer_keepalive {
                prune.push(*session_id);
            }

            if now.duration_since(peer.last_sent) > self.opt.peer_heartbeat_ivl {
                send_heartbeat.push((*session_id, peer.addr));
            }
        });

        send_heartbeat
            .drain(..)
            .for_each(|(session_id, peer_addr)| {
                if let Err(_) =
                    self.send_direct(&ControlFrame::Heartbeat(session_id).encode(), &peer_addr)
                {
                    prune.push(session_id);
                }
            });

        prune.drain(..).for_each(|session_id| {
            self.peer_update = true;
            self.peers.remove(&session_id);
        });

        self.reconnect()?;

        Ok(())
    }

    pub fn update_peers(&mut self) -> Option<Vec<u64>> {
        if self.peer_update {
            self.peer_update = false;
            return Some(self.peers.keys().cloned().collect());
        }

        return None;
    }
}
