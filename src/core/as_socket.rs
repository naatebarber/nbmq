use std::error::Error;

use super::sock_opt::SockOpt;

pub trait AsSocket {
    type Output: AsSocket;

    /// Create a bound socket at a specified address
    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>>;
    /// Create a bound socket at a random high port and connect it to a remote address
    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>>;

    // Send a multipart message
    fn send(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>>;

    // Receive a multipart message
    fn recv(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>>;

    // Step the system, call this once per iteration of your event loop
    fn tick(&mut self) -> Result<(), Box<dyn Error>>;

    /// Get a mutable set of socket options
    fn opt(&mut self) -> &mut SockOpt;

    /// Get the current number of connected peers
    fn peers(&self) -> usize;
}
