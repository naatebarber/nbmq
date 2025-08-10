use std::{hash::Hasher, net::SocketAddr};

use crate::{hash::Fnv1a64, ts::get_ts_u64};

pub fn softmax(x: &[f64]) -> Vec<f64> {
    let max = x.iter().cloned().fold(f64::MIN, f64::max);
    let mut exps = x.iter().map(|xi| (*xi - max).exp()).collect::<Vec<f64>>();
    let s = exps.iter().sum::<f64>();
    exps.iter_mut().for_each(|x| *x /= s);
    exps
}

/// Create a session ID for a connection
/// Uses the peer's address at time of connect, and the
/// current u64 timestamp hashed with fnv1a
pub fn session_id(addr: &SocketAddr) -> u64 {
    let mut hasher = Fnv1a64::new();
    hasher.write(&get_ts_u64().to_be_bytes());
    hasher.write(addr.to_string().as_bytes());
    hasher.finish()
}

/// Create a message ID for framing
/// Uses the message data and a nonce hashed with fnv1a
pub fn message_id(data: &[&[u8]], nonce: u64) -> u64 {
    let mut hasher = Fnv1a64::new();

    for part in data.iter() {
        hasher.write(part);
    }

    hasher.write(&nonce.to_be_bytes());
    hasher.finish()
}
