use std::{hash::Hasher, thread, time::Duration};

use nbmq::{
    SockOpt,
    hash::Fnv1a64,
    queue::{RecvQueue, SendQueue},
};

fn sleep(n: f64) {
    thread::sleep(Duration::from_secs_f64(n));
}

fn message(bytes: usize) -> Vec<Vec<u8>> {
    let mut m = vec![0u8; bytes];
    m[bytes - 1] = 1;
    vec![m]
}

#[test]
pub fn send_queue_to_recv_queue() {
    let mut opt = SockOpt::default();
    opt.uncompleted_message_ttl = 0.1;

    let mut sq = SendQueue::new(opt.clone());
    let mut rq = RecvQueue::new(opt);

    let a = message(10000);
    let ref_a = a.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    sq.push(ref_a.as_slice(), 0).unwrap();

    while let Some(f) = sq.pull() {
        rq.push(f.as_slice()).unwrap();
    }

    let _a = rq.pull().unwrap();

    assert!(_a == a);
}

#[test]
pub fn recv_queue_will_maint_incomplete() {
    let mut opt = SockOpt::default();
    opt.uncompleted_message_ttl = 0.1;
    opt.queue_maint_ivl = 0.1;

    let mut sq = SendQueue::new(opt.clone());
    let mut rq = RecvQueue::new(opt);

    let a = message(10000);
    let ref_a = a.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    sq.push(ref_a.as_slice(), 0).unwrap();

    if let Some(f) = sq.pull() {
        rq.push(f.as_slice()).unwrap();
    }

    sleep(0.2);

    assert!(rq.incoming.len() == 1);
    assert!(rq.pull().is_none() == true);
    assert!(rq.incoming.len() == 0);
    assert!(rq.complete.len() == 0);
}

#[test]
pub fn safe_send_queue_will_resend_until_limit() {
    let mut opt = SockOpt::default();
    opt.safe_resend_limit = 2;
    opt.safe_resend_ivl = 0.01;

    let mut sq = SendQueue::new(opt.clone());

    let a = ["hello".as_bytes()];

    sq.push(&a, 0).unwrap();

    let frame = sq.pull_safe().unwrap();

    sleep(0.02);

    let frame_2 = sq.pull_safe().unwrap();
    assert!(frame == frame_2);

    // Resend ivl hasn't passed yet, should be none.
    assert!(sq.pull_safe().is_none() == true);

    sleep(0.02);
    let frame_3 = sq.pull_safe().unwrap();
    assert!(frame_2 == frame_3);

    sleep(0.02);

    // Resend interval passsed, but resend limit reached. should drop.
    assert!(sq.pull_safe().is_none() == true);
    assert!(sq.sent.len() == 0);
}

#[test]
pub fn safe_send_queue_will_stop_sending_when_confirmed() {
    let mut opt = SockOpt::default();
    opt.safe_resend_limit = 2;
    opt.safe_resend_ivl = 0.01;

    let mut sq = SendQueue::new(opt.clone());

    let a = ["hello".as_bytes()];

    sq.push(&a, 0).unwrap();

    let frame = sq.pull_safe().unwrap();

    sleep(0.02);

    let frame_2 = sq.pull_safe().unwrap();
    assert!(frame == frame_2);

    let mut hasher = Fnv1a64::new();
    hasher.write(&frame_2);
    let hash = hasher.finish();

    sq.confirm_safe(hash);

    sleep(0.02);

    // Resend interval passed, but message was confirmed and should have been removed from queue.
    assert!(sq.pull_safe().is_none() == true);
    assert!(sq.sent.len() == 0);
    assert!(sq.exp.len() == 0);
}
