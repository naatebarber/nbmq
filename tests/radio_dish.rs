use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dish, Radio, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
fn radio_errors_on_recv() -> Result<(), Box<dyn Error>> {
    let mut radio = Socket::<Radio>::new().bind("0.0.0.0:1010")?;

    assert!(radio.recv_multipart().is_err() == true);

    Ok(())
}

#[test]
fn dish_errors_on_send() -> Result<(), Box<dyn Error>> {
    let mut dish = Socket::<Dish>::new().bind("0.0.0.0:1020")?;

    assert!(dish.send_multipart(&["test".as_bytes()]).is_err() == true);

    Ok(())
}

#[test]
fn radio_sends_to_multiple_dish() -> Result<(), Box<dyn Error>> {
    let mut radio = Socket::<Radio>::new().bind("0.0.0.0:1030")?;
    let mut dish_1 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;
    let mut dish_2 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;
    let mut dish_3 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;

    sleep(0.01);

    radio.send_multipart(&["broadcast".as_bytes()])?;

    sleep(0.01);

    let dishes = [&mut dish_1, &mut dish_2, &mut dish_3];

    for dish in dishes {
        let mut datas = vec![];
        while let Ok(data) = dish.recv_multipart() {
            datas.push(data);
        }

        println!("datas: {}", datas.len());

        assert!(datas.len() == 1);

        let message = String::from_utf8(datas[0][0].to_vec())?;

        assert!(message == "broadcast");
    }

    Ok(())
}

#[test]
fn radio_maints_dish_on_disconnect() -> Result<(), Box<dyn Error>> {
    // Bind the radio, with a very short peer keepalive
    // Bind two dishes. these will send out heartbeats immediately

    let mut radio = Socket::<Radio>::new()
        .set_peer_keepalive(0.03)
        .bind("0.0.0.0:1040")?;

    let mut dish_1 = Socket::<Dish>::new()
        .set_peer_heartbeat_ivl(0.01)
        .connect("127.0.0.1:1040")?;

    let dish_2 = Socket::<Dish>::new().connect("127.0.0.1:1040")?;

    // Wait a second for all socket transport to complete
    sleep(0.01);

    // Send multipart from radio. This will internally drain control state, absorbing the connect
    // heartbeats from the connected dishes.
    radio.send_multipart(&["broadcast".as_bytes()])?;

    // At this point we should have two peers
    println!("RADIO PEERS: {}", radio.peers());
    assert!(radio.peers() == 2);

    // We let some time elapse, then call recv on the first dish. When recv is called on this dish,
    // because of our sockopt timings, it will send a heartbeat to the radio.
    //
    // We drop the second dish. We could achieve the same result by doing literally nothing with
    // the second dish, but I want to be verbose here.
    sleep(0.02);
    dish_1.recv_multipart()?;
    drop(dish_2);

    // We let some more time elapse
    sleep(0.02);

    // We send two more things through the radio, this internally absorbs the heartbeat control
    // frame sent by dish 1, updating its keepalive.
    //
    // Since no keepalive for dish 2 was seen, and time since its last keepalive is greater than
    // the peer_keepalive interval we set, dish 2 will be cleaned out from the radio peer list.
    //
    // This logic is handled internally by Core, but radio/dish is the primary candidate where it
    // is needed.
    radio.send_multipart(&["broadcast_2".as_bytes()])?;
    radio.send_multipart(&["broadcast_3".as_bytes()])?;

    // We let some more time elapse, by now dish 2 has long since expired.
    sleep(0.01);

    // We assert here that dish 2 has been removed
    println!("RADIO PEERS: {}", radio.peers());
    assert!(radio.peers() == 1);

    // For kicks we pull from dish 1 and ensure we have received all the data sent by the radio
    let mut datas = vec![];
    while let Ok(msg) = dish_1.recv_multipart() {
        datas.push(msg);
    }

    println!("DATAS: {}", datas.len());
    assert!(datas.len() == 2);

    return Ok(());
}
