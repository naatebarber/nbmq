use std::fmt;

#[derive(Debug)]
pub enum NBMQError {
    WouldBlock,
    MessageTooLong,
    MessageTooLarge,
    FrameCorrupt,
    NoPeer,
    UnknownPeer,
    PeerDisconnected,
    HighWaterMark,
    RecvFailed(std::io::Error),
    SendFailed(std::io::Error),
    BindFailed(std::io::Error),
    ConnectFailed(std::io::Error),
    Internal(String),
}

impl fmt::Display for NBMQError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WouldBlock => write!(f, "Operation would block"),
            Self::MessageTooLong => write!(f, "Message multipart length exceeds max of 255"),
            Self::MessageTooLarge => write!(f, "Message exceeds 4GB size limit"),
            Self::FrameCorrupt => write!(f, "Internal frame is malformed"),
            Self::NoPeer => write!(f, "No socket peers available"),
            Self::UnknownPeer => write!(f, "Peer specified is unknown"),
            Self::PeerDisconnected => write!(f, "Peer has been disconnected"),
            Self::HighWaterMark => write!(f, "Queue is over high water mark"),
            Self::RecvFailed(_) => write!(f, "Failed to recv"),
            Self::SendFailed(_) => write!(f, "Failed to send"),
            Self::BindFailed(_) => write!(f, "Failed to bind"),
            Self::ConnectFailed(_) => write!(f, "Failed to connect"),
            Self::Internal(_) => write!(f, "Internal error"),
        }
    }
}

impl std::error::Error for NBMQError {}
