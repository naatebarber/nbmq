use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, SafeDealer, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
fn safe_socket_resends_until_success() -> Result<(), Box<dyn Error>> {
    let mut client = Socket::<SafeDealer>::new();
    client.opt.safe_resend_ivl = 0.01;
    client.opt.safe_resend_limit = 20;
    let mut client = client.connect("127.0.0.1:4000")?;

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

    let sq = client.peek_send_queue();
    let n_expecting = sq.exp.len();
    let n_resend = sq.sent.len();
    println!("{} exp, {} resend", n_expecting, n_resend);

    sleep(0.02);

    client.drain_control()?;

    let sq = client.peek_send_queue();
    let n_expecting = sq.exp.len();
    let n_resend = sq.sent.len();
    println!("{} exp, {} resend", n_expecting, n_resend);

    println!("datas {}", datas.len());
    assert!(datas.len() == 30);

    // THE ISSUE IS THAT RECONNECT WILL RETURN WITH SUCCESS REGARDLESS OF WHETHER OR NOT IT
    // SUCCEEDED. and what does is causes fucker not to reconnect until after it has failed a
    // message or two. Its completely adequate behavior, its not broken, its just irritating for
    // this test specifically.

    Ok(())
}
