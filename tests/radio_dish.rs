use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dish, Radio, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

#[test]
fn radio_errors_on_recv() -> Result<(), Box<dyn Error>> {
    let mut radio = Socket::<Radio>::new().bind("0.0.0.0:1010")?;

    assert!(radio.recv().is_err() == true);

    Ok(())
}

#[test]
fn dish_errors_on_send() -> Result<(), Box<dyn Error>> {
    let mut dish = Socket::<Dish>::new().bind("0.0.0.0:1020")?;

    assert!(dish.send(&["test".as_bytes()]).is_err() == true);

    Ok(())
}

#[test]
fn radio_sends_to_multiple_dish() -> Result<(), Box<dyn Error>> {
    let mut radio = Socket::<Radio>::new().bind("0.0.0.0:1030")?;
    let mut dish_1 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;
    let mut dish_2 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;
    let mut dish_3 = Socket::<Dish>::new().connect("127.0.0.1:1030")?;

    let dishes = [&mut dish_1, &mut dish_2, &mut dish_3];

    sleep(0.01);
    radio.tick()?;
    radio.send(&["broadcast".as_bytes()])?;
    radio.tick()?;

    sleep(0.01);

    for dish in dishes {
        dish.tick()?;

        let mut datas = vec![];
        while let Ok(data) = dish.recv() {
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

    let _dish_2 = Socket::<Dish>::new().connect("127.0.0.1:1040")?;

    sleep(0.01);
    println!("radio");
    radio.tick()?; // <- radio receives connects / heartbeats from dishes here
    radio.send(&["broadcast".as_bytes()])?;
    assert!(radio.peers() == 2);

    sleep(0.02);
    println!("dish1");
    dish_1.tick()?; // dish 1 receives broadcast, and resends heartbeat here

    sleep(0.02);
    println!("radio");
    radio.tick()?; // its been 0.04 sec since dish 2 sent any heartbeats, should be gone by now

    println!("radio peers: {}", radio.peers());
    assert!(radio.peers() == 1);

    Ok(())

    //
    //
    // sleep(0.01)
    //
    // // Wait a second for all socket transport to complete
    // sleep(0.01);
    // dish_1.tick()?;
    // dish_2.tick()?;
    //
    //
    //
    // // Send multipart from radio. This will internally drain control state, absorbing the connect
    // // heartbeats from the connected dishes.
    // radio.tick()?;
    // radio.send(&["broadcast".as_bytes()])?;
    // radio.tick()?;
    //
    // // At this point we should have two peers
    // println!("RADIO PEERS: {}", radio.peers());
    // assert!(radio.peers() == 2);
    //
    // // We let some time elapse, then call recv on the first dish. When recv is called on this dish,
    // // because of our sockopt timings, it will send a heartbeat to the radio.
    // //
    // // We drop the second dish. We could achieve the same result by doing literally nothing with
    // // the second dish, but I want to be verbose here.
    // sleep(0.01);
    // dish_1.tick()?;
    // drop(dish_2);
    //
    // // We let some more time elapse
    // sleep(0.02);
    // dish_1.tick()?;
    //
    // // We send two more things through the radio, this internally absorbs the heartbeat control
    // // frame sent by dish 1, updating its keepalive.
    // //
    // // Since no keepalive for dish 2 was seen, and time since its last keepalive is greater than
    // // the peer_keepalive interval we set, dish 2 will be cleaned out from the radio peer list.
    // //
    // // This logic is handled internally by Core, but radio/dish is the primary candidate where it
    // // is needed.
    // radio.tick()?;
    //
    // // We let some more time elapse, by now dish 2 has long since expired.
    // sleep(0.01);
    // radio.tick()?;
    //
    // // We assert here that dish 2 has been removed
    // println!("RADIO PEERS: {}", radio.peers());
    // assert!(radio.peers() == 1);
    //
    // // For kicks we pull from dish 1 and ensure we have received all the data sent by the radio
    // let mut datas = vec![];
    //
    // dish_1.tick()?;
    // while let Ok(msg) = dish_1.recv() {
    //     datas.push(msg);
    // }
    //
    // println!("DATAS: {}", datas.len());
    // assert!(datas.len() == 2);
    //
    // return Ok(());
}
