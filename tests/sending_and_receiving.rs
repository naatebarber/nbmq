use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dealer, Dish, Radio, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
pub fn basic_send() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2000")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2000")?;

    let msg = "Hello World!";
    client.send_multipart(&[msg.as_bytes()])?;

    sleep(0.01);

    let msgb = server.recv_multipart()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());

    Ok(())
}

#[test]
pub fn ping_pong() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2001")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2001")?;

    let msg = "Ping";
    client.send_multipart(&[msg.as_bytes()])?;

    sleep(0.01);

    let msgb = server.recv_multipart()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());

    let msg = "Pong";
    server.send_multipart(&[msg.as_bytes()])?;

    sleep(0.01);

    let msgb = client.recv_multipart()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());

    Ok(())
}

#[test]
pub fn round_robin() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2002")?;
    let mut a = Socket::<Dealer>::new().connect("127.0.0.1:2002")?;
    let mut b = Socket::<Dealer>::new().connect("127.0.0.1:2002")?;
    let mut c = Socket::<Dealer>::new().connect("127.0.0.1:2002")?;

    sleep(0.01);
    // prime the connnections

    let ping = "ping".as_bytes();

    a.send_multipart(&[ping])?;
    b.send_multipart(&[ping])?;
    c.send_multipart(&[ping])?;

    sleep(0.01);

    let mut server_got = 0;
    while let Ok(msgb) = server.recv_multipart() {
        let msg = String::from_utf8(msgb[0].to_vec())?;
        assert!(msg.as_str() == "ping");
        server_got += 1;
    }

    assert!(server_got == 3);

    for _ in 0..3 {
        server.send_multipart(&["pong".as_bytes()])?;
    }

    sleep(0.01);

    let socks = [&mut a, &mut b, &mut c];

    for client in socks {
        let msgb = client.recv_multipart()?;
        let msg = String::from_utf8(msgb[0].to_vec())?;

        assert!(msg.as_str() == "pong");
    }

    Ok(())
}

#[test]
pub fn large_message() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2003")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2003")?;

    let mut large_bin = vec![];
    for _ in 0..100000 {
        large_bin.push(0);
    }
    large_bin.push(1);

    client.send_multipart(&[&large_bin])?;

    sleep(0.01);

    let msgb = server.recv_multipart()?;
    assert!(msgb.len() == 1);
    assert!(msgb[0].len() == 100001);
    assert!(msgb[0][100000] == 1);

    Ok(())
}

#[test]
pub fn long_message() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2004")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2004")?;

    sleep(0.01);

    let mut long_msg_parts = vec![];
    long_msg_parts.push("hello".as_bytes().to_vec());
    for _ in 0..100 {
        let mut large_bin = vec![];
        for _ in 0..1000 {
            large_bin.push(0);
        }
        large_bin.push(1);
        long_msg_parts.push(large_bin);
    }
    long_msg_parts.push("world".as_bytes().to_vec());

    let long_ref = long_msg_parts
        .iter()
        .map(|x| x.as_slice())
        .collect::<Vec<_>>();

    client.send_multipart(&long_ref)?;

    sleep(0.01);

    let msgb = server.recv_multipart()?;
    assert!(msgb.len() == 102);

    let start = String::from_utf8(msgb[0].to_vec())?;
    let end = String::from_utf8(msgb[101].to_vec())?;

    assert!(start.as_str() == "hello");
    assert!(end.as_str() == "world");

    Ok(())
}

#[test]
fn sockets_register_as_peers_on_connect() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2005")?;
    Socket::<Dealer>::new().connect("127.0.0.1:2005")?;
    Socket::<Dealer>::new().connect("127.0.0.1:2005")?;

    server.drain_control()?;

    assert!(server.peers() == 2);

    Ok(())
}

#[test]
fn long_pause_doesnt_remove_peer() -> Result<(), Box<dyn Error>> {
    let mut radio = Socket::<Radio>::new()
        .set_peer_keepalive(0.01)
        .bind("0.0.0.0:2006")?;
    let mut dish = Socket::<Dish>::new()
        .set_peer_heartbeat_ivl(0.001)
        .connect("127.0.0.1:2006")?;

    let mut ct = 0;

    sleep(0.01);

    radio.drain_control()?;
    radio.send_multipart(&["hello".as_bytes()])?;

    sleep(0.01);

    while let Ok(_) = dish.recv_multipart() {
        ct += 1;
    }
    assert!(ct == 1);

    // Sleep for 10x longer than the peer keepalive.
    sleep(0.1);

    radio.send_multipart(&["world".as_bytes()])?;

    sleep(0.01);

    while let Ok(_) = dish.recv_multipart() {
        ct += 1;
    }
    assert!(ct == 2);

    sleep(0.01);

    radio.send_multipart(&["final".as_bytes()])?;

    assert!(radio.peers() == 1);

    Ok(())
}
