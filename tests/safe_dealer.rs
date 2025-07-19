use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, SafeDealer, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
fn safe_socket_resends_until_success() -> Result<(), Box<dyn Error>> {
    let mut client = Socket::<SafeDealer>::new()
        .set_safe_resend_ivl(0.01)
        .set_safe_resent_limit(20)
        .connect("127.0.0.1:4000")?;

    for i in 10..20 {
        let data = vec![0u8; i];
        let _ = client.send_multipart(&[data.as_slice()]);
    }

    let mut server = Socket::<SafeDealer>::new().bind("0.0.0.0:4000")?;

    sleep(0.02);

    for i in 20..30 {
        let data = vec![0u8; i];
        let _ = client.send_multipart(&[data.as_slice()]);
    }

    sleep(0.02);

    for i in 30..40 {
        let data = vec![0u8; i];
        let _ = client.send_multipart(&[data.as_slice()]);
    }

    sleep(0.02);

    let mut datas = vec![];
    while let Ok(data) = server.recv_multipart() {
        datas.push(data);
    }

    sleep(0.02);

    client.drain_control()?;

    println!("datas {}", datas.len());
    assert!(datas.len() == 30);

    // THE ISSUE IS THAT RECONNECT WILL RETURN WITH SUCCESS REGARDLESS OF WHETHER OR NOT IT
    // SUCCEEDED. and what does is causes fucker not to reconnect until after it has failed a
    // message or two. Its completely adequate behavior, its not broken, its just irritating for
    // this test specifically.

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

    server.drain_control()?;
    server.send_multipart(&["to client 1".as_bytes()])?;
    println!("1. sent to client 1");

    sleep(0.01);

    let mut client_2 = Socket::<SafeDealer>::new().connect("127.0.0.1:4010")?;

    sleep(0.01);

    server.drain_control()?;
    server.send_multipart(&["to client 2".as_bytes()])?;
    println!("2. sent to client 2");

    sleep(0.02);

    server.send_multipart(&["to client 1".as_bytes()])?;

    sleep(0.02);

    server.send_multipart(&["to client 2".as_bytes()])?;

    sleep(0.01);

    let mut ct = 0;
    while let Ok(_) = client.recv_multipart() {
        ct += 1;
    }
    while let Ok(_) = client_2.recv_multipart() {
        ct += 1;
    }

    assert!(ct == 4);

    Ok(())
}
