use std::{net::SocketAddr, str::FromStr};

fn main() {
    let addr = "192.168.1.1:8080";

    let sa = SocketAddr::from_str(addr);

    println!("{:?}", sa);
}
