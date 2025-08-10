use std::{
    collections::HashMap,
    error::Error,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    time::Instant,
};

use super::sock_opt::SockOpt;
use crate::{
    consts,
    frame::{ControlFrame, Frame},
};

#[derive(PartialEq, Eq)]
pub enum SockMode {
    Bind,
    Connect((SocketAddr, u64)),
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

    pub mode: SockMode,
    pub peer_update: bool,
    pub peers: HashMap<u64, Peer>,
}

impl Core {
    pub fn bind(addr: &str, opt: SockOpt) -> Result<Core, Box<dyn Error>> {
        let mut socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        Core::drain_socket(&mut socket);

        Ok(Core {
            sock: socket,
            opt,

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

            mode: SockMode::Connect((peer_addr, 0)),
            peer_update: true,
            peers,
        })
    }

    fn drain_socket(sock: &mut UdpSocket) {
        let mut buf = [0u8; 2048];
        loop {
            if let Err(_) = sock.recv_from(&mut buf) {
                return;
            }
        }
    }

    fn connect_socket(sock: &mut UdpSocket, peer_addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
        sock.connect(peer_addr)?;
        sock.send(&ControlFrame::Connect.encode())?;

        Ok(())
    }

    fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        match self.mode {
            SockMode::Connect((peer_addr, session_id)) => {
                if let None = self.peers.get(&session_id) {
                    Core::connect_socket(&mut self.sock, &peer_addr)?;
                }
            }
            _ => (),
        }

        return Ok(());
    }

    fn recv_frame(&mut self) -> Result<(Vec<u8>, SocketAddr), Box<dyn Error>> {
        self.reconnect()?;

        let mut buffer = vec![0u8; consts::MAX_FRAME_SIZE];

        let recv_addr = match self.mode {
            SockMode::Connect((peer_addr, _)) => {
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

        Ok((buffer, recv_addr))
    }

    fn control(
        &mut self,
        data: &[u8],
        peer_addr: &SocketAddr,
    ) -> Result<Option<ControlFrame>, Box<dyn Error>> {
        let Ok(Some(control_frame)) = ControlFrame::parse(&data) else {
            let Ok(data_frame) = Frame::parse(&data) else {
                return Ok(None);
            };

            if let Some(peer) = self.peers.get_mut(&data_frame.session_id) {
                peer.last_seen = Instant::now();

                if peer.last_seen.duration_since(peer.last_sent).as_secs_f64()
                    > self.opt.peer_heartbeat_ivl
                {
                    peer.last_sent = Instant::now();
                    self.send_direct(
                        &ControlFrame::Heartbeat(data_frame.session_id).encode(),
                        peer_addr,
                    )?;
                }
            }

            return Ok(None);
        };

        match control_frame {
            ControlFrame::Connect => {
                let session_id = super::f::session_id(peer_addr);

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
                if let SockMode::Connect((_, session)) = &mut self.mode {
                    *session = session_id;
                }

                self.peers.insert(
                    session_id,
                    Peer {
                        addr: peer_addr.clone(),
                        last_seen: Instant::now(),
                        last_sent: Instant::now(),
                    },
                );

                self.send_direct(&ControlFrame::Heartbeat(session_id).encode(), peer_addr)?;
                self.peer_update = true;
            }
            ControlFrame::Disconnected(session_id) => {
                if let SockMode::Connect((_, session)) = &mut self.mode {
                    *session = 0;
                }

                self.peers.remove(&session_id);
            }
            ControlFrame::Heartbeat(session_id) => {
                if let Some(peer) = self.peers.get_mut(&session_id) {
                    peer.last_seen = Instant::now();
                } else {
                    self.send_direct(&ControlFrame::Disconnected(session_id).encode(), peer_addr)?;
                };
            }
            _ => (),
        }

        return Ok(Some(control_frame));
    }

    pub fn recv(&mut self) -> Result<(Vec<u8>, u64, Option<ControlFrame>), Box<dyn Error>> {
        let (frame, peer_addr) = self.recv_frame()?;
        let control = match self.control(&frame, &peer_addr) {
            Ok(control) => control,
            Err(_) => None,
        };

        let session = u64::from_be_bytes(frame[2..10].try_into()?);

        return Ok((frame, session, control));
    }

    pub fn send_direct(
        &mut self,
        data: &[u8],
        peer_addr: &SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        match self.mode {
            SockMode::Connect(_) => {
                self.sock.send(data)?;
            }
            SockMode::Bind => {
                self.sock.send_to(data, peer_addr)?;
            }
        }

        Ok(())
    }

    pub fn send_all(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match &mut self.mode {
            SockMode::Connect((_, session_id)) => {
                let Some(peer) = self.peers.get_mut(&session_id) else {
                    *session_id = 0;
                    return Err("No peer".into());
                };

                self.sock.send(data)?;

                peer.last_sent = Instant::now();
            }
            SockMode::Bind => {
                let mut drop_stale_peers = vec![];

                for (session_id, peer) in self.peers.iter_mut() {
                    if let Ok(_) = self.sock.send_to(data, peer.addr) {
                        peer.last_sent = Instant::now();
                    }

                    if Instant::now().duration_since(peer.last_seen).as_secs_f64()
                        > self.opt.peer_keepalive
                    {
                        drop_stale_peers.push(session_id.clone());
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

    pub fn send_peer(&mut self, data: &[u8], session_id: &u64) -> Result<(), Box<dyn Error>> {
        let Some(peer) = self.peers.get_mut(session_id) else {
            return Err("No such peer".into());
        };

        peer.last_sent = Instant::now();
        let addr = peer.addr;

        if Instant::now().duration_since(peer.last_seen).as_secs_f64() > self.opt.peer_keepalive {
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

    pub fn maint(&mut self) {}

    pub fn update_peers(&mut self) -> Option<Vec<u64>> {
        if self.peer_update {
            self.peer_update = false;
            return Some(self.peers.keys().cloned().collect());
        }

        return None;
    }
}
