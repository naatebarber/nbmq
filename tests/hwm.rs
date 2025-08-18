use std::{error::Error, thread, time::Duration};

use nbmq::{AsSocket, Dealer, Dish, Radio, SafeDealer, Socket};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

pub fn fails_on_send_hwm_reached<S, R>(p: usize) -> Result<(), Box<dyn Error>>
where
    S: AsSocket,
    R: AsSocket,
{
    let addr = format!("127.0.0.1:{}", p);

    let mut sender = Socket::<S>::new().set_send_hwm(10).bind(&addr)?;

    Socket::<R>::new().connect(&addr)?;

    sleep(0.01);
    sender.tick()?; // allow sender to facilitate connection

    // queue 5 messages
    for _ in 0..5 {
        sender.send(&["abc".as_bytes()])?;
    }

    // drain the 5 messages from queue
    // internal message ct should be back at zero
    sender.tick()?;

    assert!(sender.peers() == 1);

    // we queue 10 messages
    for _ in 0..10 {
        sender.send(&["abc".as_bytes()])?;
    }

    // we queue another message, it should fail here with wouldblock.
    match sender.send(&["abc".as_bytes()]) {
        Ok(_) => panic!("no error on send hwm reach!"),
        Err(e) => {
            if e.to_string().to_lowercase().contains("block") {
                return Ok(());
            } else {
                panic!("random other error, not wouldblock, hit {}", e.to_string());
            }
        }
    };
}

pub fn fails_on_recv_hwm_reached<S, R>(p: usize) -> Result<(), Box<dyn Error>>
where
    S: AsSocket,
    R: AsSocket,
{
    let addr = format!("127.0.0.1:{}", p);

    let mut sender = Socket::<S>::new().bind(&addr)?;

    let mut receiver = Socket::<R>::new().set_recv_hwm(10).connect(&addr)?;

    sleep(0.01);
    // sender facitates connection
    sender.tick()?;
    // sender queues 5 messages
    for _ in 0..5 {
        sender.send(&["hello".as_bytes()])?;
    }
    // sender drains 5 messages to kernel
    sender.tick()?;

    sleep(0.01);
    // receiver accepts connection and receives 5 messages
    receiver.tick()?;

    // drain 5 messages out of recv queue
    let mut ct = 0;
    while let Ok(_) = receiver.recv() {
        ct += 1;
    }
    assert!(ct == 5);

    // send 11 more messages, should overwhelm recv queue on receiver
    for _ in 0..11 {
        sender.send(&["hello".as_bytes()])?;
    }
    sender.tick()?;

    sleep(0.01);

    match receiver.tick() {
        Ok(_) => panic!("no wouldblock error thrown from overwhelmed recv queue!"),
        Err(e) => {
            if e.to_string().to_lowercase().contains("block") {
                return Ok(());
            } else {
                panic!("other error, not wouldblock, returned: {}", e.to_string());
            }
        }
    }
}

#[test]
pub fn dealer_respects_sendhwm() -> Result<(), Box<dyn Error>> {
    fails_on_send_hwm_reached::<Dealer, Dealer>(6100)
}

#[test]
pub fn radio_respects_sendhwm() -> Result<(), Box<dyn Error>> {
    fails_on_send_hwm_reached::<Radio, Dish>(6101)
}

#[test]
pub fn safedealer_respects_sendhwm() -> Result<(), Box<dyn Error>> {
    fails_on_send_hwm_reached::<SafeDealer, SafeDealer>(6102)
}

#[test]
pub fn dealer_respects_recvhwm() -> Result<(), Box<dyn Error>> {
    fails_on_recv_hwm_reached::<Dealer, Dealer>(6103)
}

#[test]
pub fn radio_respects_recvhwm() -> Result<(), Box<dyn Error>> {
    fails_on_recv_hwm_reached::<Radio, Dish>(6104)
}

#[test]
pub fn safedealer_respects_recvhwm() -> Result<(), Box<dyn Error>> {
    fails_on_recv_hwm_reached::<SafeDealer, SafeDealer>(6105)
}
