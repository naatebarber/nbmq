use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, SafeDealer, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
fn safe_socket_resends_until_success() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<SafeDealer>::new().bind("0.0.0.0:4000")?;

    let mut client = Socket::<SafeDealer>::new()
        .set_safe_resend_ivl(0.01)
        .set_safe_resent_limit(20)
        .connect("127.0.0.1:4000")?;

    assert!(server.peers() == 0 && client.peers() == 0);

    sleep(0.01);
    server.tick()?;
    assert!(server.peers() == 1);

    sleep(0.01);
    client.tick()?;
    assert!(client.peers() == 1);

    for i in 10..20 {
        let data = vec![0u8; i];
        client.send(&[data.as_slice()])?;
    }

    sleep(0.02);
    client.tick()?;

    for i in 20..30 {
        let data = vec![0u8; i];
        client.send(&[data.as_slice()])?;
    }

    sleep(0.02);
    client.tick()?;

    for i in 30..40 {
        let data = vec![0u8; i];
        client.send(&[data.as_slice()])?;
    }

    client.tick()?;

    sleep(0.02);
    server.tick()?;

    let mut datas = vec![];
    while let Ok(data) = server.recv() {
        datas.push(data);
    }

    sleep(0.02);
    client.tick()?;

    println!("datas {}", datas.len());
    assert!(datas.len() == 30);

    Ok(())
}

#[test]
pub fn safe_socket_resends_to_correct_peers() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<SafeDealer>::new()
        .set_safe_resend_ivl(0.01)
        .set_safe_resent_limit(20)
        .bind("0.0.0.0:4010")?;

    let mut client = Socket::<SafeDealer>::new().connect("127.0.0.1:4010")?;

    sleep(0.01);
    server.tick()?;
    server.send(&["to client 1".as_bytes()])?;
    println!("1. sent to client 1");

    sleep(0.01);
    let mut client_2 = Socket::<SafeDealer>::new().connect("127.0.0.1:4010")?;

    sleep(0.01);
    server.tick()?;

    server.send(&["to client 2".as_bytes()])?;
    server.send(&["to client 1".as_bytes()])?;
    server.send(&["to client 2".as_bytes()])?;
    server.tick()?;

    sleep(0.01);
    client.tick()?;
    client_2.tick()?;
    let mut ct = 0;
    while let Ok(_) = client.recv() {
        println!("received client");
        ct += 1;
    }
    while let Ok(_) = client_2.recv() {
        println!("received client_2");
        ct += 1;
    }

    println!("{}", ct);
    assert!(ct == 4);

    Ok(())
}
