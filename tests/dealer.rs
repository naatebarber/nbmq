use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dealer, Socket};

fn sleep() {
    thread::sleep(Duration::from_millis(100));
}

#[test]
pub fn basic_send() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:2000")?;
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:2000")?;

    let msg = "Hello World!";
    client.send_multipart(&[msg.as_bytes()])?;

    sleep();

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

    sleep();

    let msgb = server.recv_multipart()?;
    let recv_msg = String::from_utf8(msgb[0].to_vec())?;
    assert!(msg == recv_msg.as_str());

    let msg = "Pong";
    server.send_multipart(&[msg.as_bytes()])?;

    sleep();

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

    sleep();
    // prime the connnections

    let ping = "ping".as_bytes();

    a.send_multipart(&[ping])?;
    b.send_multipart(&[ping])?;
    c.send_multipart(&[ping])?;

    sleep();

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

    sleep();

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

    sleep();

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

    sleep();

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

    sleep();

    let msgb = server.recv_multipart()?;
    assert!(msgb.len() == 102);

    let start = String::from_utf8(msgb[0].to_vec())?;
    let end = String::from_utf8(msgb[101].to_vec())?;

    assert!(start.as_str() == "hello");
    assert!(end.as_str() == "world");

    Ok(())
}
