use std::{error::Error, io};

use super::sock_opt::SockOpt;

pub trait AsSocket {
    type Output: AsSocket;

    /// Create a bound socket at a specified address
    fn bind(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>>;
    /// Create a bound socket at a random high port and connect it to a remote address
    fn connect(addr: &str, opt: SockOpt) -> Result<Self::Output, Box<dyn Error>>;

    /// Send a multipart message
    fn send_multipart(&mut self, data: &[&[u8]]) -> Result<(), Box<dyn Error>>;
    /// Receive a multipart message
    fn recv_multipart(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn Error>>;
    /// Receive control frames only on the socket, discarding all user data frames
    fn drain_control(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.recv_multipart() {
                Ok(_) => continue,
                Err(e) => {
                    if let Some(io_err) = e.downcast_ref::<io::Error>() {
                        if io_err.kind() == io::ErrorKind::WouldBlock {
                            break;
                        }
                    }

                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Get a mutable set of socket options
    fn opt(&mut self) -> &mut SockOpt;

    /// Get the current number of connected peers
    fn peers(&self) -> usize;
}
