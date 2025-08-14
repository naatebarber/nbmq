use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dealer, Dish, Radio, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
pub fn basic_send() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2000")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2000")?;

    sleep(0.01);
    server.tick()?;

    sleep(0.01);
    client.tick()?;
    let msg = "Hello World!";
    client.send(&[msg.as_bytes()])?;
    client.tick()?;

    sleep(0.01);
    server.tick()?;
    let msgb = server.recv()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());

    Ok(())
}

#[test]
pub fn ping_pong() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2001")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2001")?;

    sleep(0.01);
    server.tick()?;

    sleep(0.01);
    client.tick()?;
    let msg = "Ping";
    client.send(&[msg.as_bytes()])?;
    client.tick()?;

    sleep(0.01);
    server.tick()?;
    let msgb = server.recv()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());
    let msg = "Pong";
    server.send(&[msg.as_bytes()])?;
    server.tick()?;

    sleep(0.01);
    client.tick()?;
    let msgb = client.recv()?;
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
    server.tick()?;

    sleep(0.01);
    a.tick()?;
    b.tick()?;
    c.tick()?;
    a.send(&["ping1".as_bytes()])?;
    b.send(&["ping2".as_bytes()])?;
    c.send(&["ping3".as_bytes()])?;
    a.tick()?;
    b.tick()?;
    c.tick()?;

    sleep(0.01);
    server.tick()?;
    let mut server_got = 0;
    while let Ok(msgb) = server.recv() {
        let msg = String::from_utf8(msgb[0].to_vec())?;
        assert!(msg.as_str().contains("ping"));
        server_got += 1;
    }
    println!("server got {}", server_got);
    assert!(server_got == 3);

    for _ in 0..3 {
        server.send(&["pong".as_bytes()])?;
    }
    server.tick()?;

    sleep(0.01);
    let socks = [&mut a, &mut b, &mut c];
    for client in socks {
        client.tick()?;
        let msgb = client.recv()?;
        let msg = String::from_utf8(msgb[0].to_vec())?;

        assert!(msg.as_str() == "pong");
    }

    Ok(())
}

#[test]
pub fn large_message() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2003")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2003")?;

    sleep(0.01);
    server.tick()?;

    sleep(0.01);
    client.tick()?;
    let mut large_bin = vec![];
    for _ in 0..100000 {
        large_bin.push(0);
    }
    large_bin.push(1);
    client.send(&[&large_bin])?;
    client.tick()?;

    sleep(0.01);
    server.tick()?;
    let msgb = server.recv()?;
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
    server.tick()?;

    sleep(0.01);
    client.tick()?;
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

    client.send(&long_ref)?;
    client.tick()?;

    sleep(0.01);
    server.tick()?;
    let msgb = server.recv()?;
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

    sleep(0.01);
    server.tick()?;

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
    radio.tick()?;
    radio.send(&["hello".as_bytes()])?;
    radio.tick()?;

    sleep(0.01);
    dish.tick()?;
    while let Ok(_) = dish.recv() {
        ct += 1;
    }
    assert!(ct == 1);

    // Sleep for 10x longer than the peer keepalive.
    sleep(0.1);
    radio.send(&["world".as_bytes()])?;
    radio.tick()?;

    sleep(0.01);
    dish.tick()?;
    while let Ok(_) = dish.recv() {
        ct += 1;
    }
    assert!(ct == 2);

    sleep(0.01);
    radio.send(&["final".as_bytes()])?;
    radio.tick()?;

    assert!(radio.peers() == 1);

    Ok(())
}

#[test]
pub fn disconnected_client_fails_to_send() -> Result<(), Box<dyn Error>> {
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2007")?;

    client.tick()?;
    match client.send(&["Hello".as_bytes()]) {
        Ok(_) => panic!("disconnected client sent successfully"),
        Err(e) => assert!(e.to_string() == "No peer"),
    }

    Ok(())
}
