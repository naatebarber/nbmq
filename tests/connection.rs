use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dealer, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
pub fn client_will_reconnect_until_success() -> Result<(), Box<dyn Error>> {
    let mut client = Socket::<Dealer>::new()
        .set_reconnect_wait(0.005)
        .set_peer_keepalive(0.03)
        .set_peer_heartbeat_ivl(0.01)
        .connect("127.0.0.1:5100")?;

    sleep(0.01);
    client.tick()?;

    sleep(0.01);
    client.tick()?;

    sleep(0.02);
    let mut server = Socket::<Dealer>::new().bind("0.0.0.0:5100")?;

    sleep(0.01);
    client.tick()?;

    sleep(0.01);
    server.tick()?;

    sleep(0.01);
    client.tick()?;

    assert!(server.peers() == 1);
    assert!(client.peers() == 1);

    Ok(())
}

pub fn unconnected_sockets_will_error_with_no_peer() -> Result<(), Box<dyn Error>> {
    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:5101")?;
    client.tick()?;
    if let Err(e) = client.send(&["test".as_bytes()]) {
        assert!(e.to_string() == "No peer");
    } else {
        panic!("unconnected send successful")
    }

    let mut server = Socket::<Dealer>::new().bind("127.0.0.1:5102")?;
    server.tick()?;
    if let Err(e) = server.send(&["test".as_bytes()]) {
        assert!(e.to_string() == "No peer");
    } else {
        panic!("unconnected send successful")
    }

    Ok(())
}

#[test]
pub fn connection_reconnect_with_tick() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new()
        .set_peer_keepalive(0.03)
        .set_peer_heartbeat_ivl(0.01)
        .bind("0.0.0.0:5000")?;

    let mut client = Socket::<Dealer>::new()
        .set_peer_keepalive(0.03)
        .set_peer_heartbeat_ivl(0.01)
        .connect("127.0.0.1:5000")?;

    println!("a");
    sleep(0.01);
    server.tick()?; // server registers client
    assert!(server.peers() == 1);

    println!("b");
    sleep(0.04);
    server.tick()?; // server drops client
    assert!(server.peers() == 0);

    println!("c");
    sleep(0.01);
    client.tick()?; // client sends heartbeat

    println!("d");
    sleep(0.01);
    server.tick()?; // server receives heartbeat, sends disconnect
    assert!(server.peers() == 0);

    println!("e");
    sleep(0.01);
    client.tick()?; // client receives disconnect and sends connect request to server

    println!("f");
    sleep(0.01);
    server.tick()?; // server re-registers client
    assert!(server.peers() == 1);

    println!("g");
    sleep(0.01);
    client.tick()?;

    return Ok(());
}

#[test]
fn data_send_fails_after_unfinished_handshake() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new()
        .set_peer_keepalive(0.03)
        .set_peer_heartbeat_ivl(0.01)
        .bind("0.0.0.0:5001")?;

    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:5001")?;

    sleep(0.01);

    client.tick()?;
    if let Err(e) = client.send(&["hello".as_bytes()]) {
        assert!(e.to_string().contains("No peer"));
    } else {
        panic!("managed to send to uncompleted connection");
    }

    server.tick()?;
    sleep(0.01);

    // now try again after allowing client to absorb server CONNECT

    client.tick()?;
    if let Err(_) = client.send(&["hello".as_bytes()]) {
        panic!("send should have succeeded");
    }

    Ok(())
}

#[test]
fn disconnected_socket_cant_send_data_messages() -> Result<(), Box<dyn Error>> {
    let mut server = Socket::<Dealer>::new()
        .set_peer_keepalive(0.03)
        .set_peer_heartbeat_ivl(0.01)
        .bind("0.0.0.0:5002")?;

    let mut client = Socket::<Dealer>::new().connect("127.0.0.1:5002")?;

    sleep(0.01);
    server.tick()?;
    assert!(server.peers() == 1);

    sleep(0.03);
    server.tick()?;
    assert!(server.peers() == 0);

    client.tick()?;
    client.send(&["hello".as_bytes()])?;

    sleep(0.01);

    server.tick()?; // server sends client a disconnect
    // server drops the data message

    if let Ok(_) = server.recv() {
        panic!("server shouldnt have received data from disconnected client");
    }

    return Ok(());
}
