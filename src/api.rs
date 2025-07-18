use std::error::Error;

use crate::{AsSocket, SockOpt};

pub struct Socket<T> {
    pub opt: SockOpt,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: AsSocket> Socket<T> {
    pub fn new() -> Socket<T> {
        Socket {
            opt: SockOpt::default(),
            _phantom: std::marker::PhantomData::default(),
        }
    }

    pub fn bind(self, addr: &str) -> Result<T::Output, Box<dyn Error>> {
        T::bind(addr, self.opt)
    }

    pub fn connect(self, addr: &str) -> Result<T::Output, Box<dyn Error>> {
        T::connect(addr, self.opt)
    }
}
