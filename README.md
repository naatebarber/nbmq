# NBMQ

UDP first, timerless messaging queue for a fun, lightning fast, fire-and-forget messaging style with event-stepped connection awareness.

This library was inspired by ZeroMQ, specifically ZeroMQ using the `send_multipart() / recv_multipart()` paradigm along with the `ZMQ::DONTWAIT` flag.
I really enjoy ZeroMQ under this paradigm - never having to worry about blocking when a HWM is reached, failing loudly and explicitly when pressure is too high, etc.
This library is just meant to take this paradigm further, relinquishing more control of delivery, and providing more transparency to internal state. It's 
intended to be more like a toolkit than a plug and play solution, but if you're anything like me, that's a positive.

The entirety of NBMQ is state-stepped when a user performs an operation on a socket. Peer health is managed this way. Backpressure control and message insurance will
follow. No background threads, no hidden state, no bullshit.

### Zero Dependencies By Design

No dependencies, or at least as few as possible. Absolutely NO externally linked C libraries. 
Everything for NBMQ will be implemented fresh in Rust. I end up building for edge compute systems a lot of the time, 
and want the footprint of this library to be as small as possible.

### Network Protocol Versatility

For now, I'm starting with exclusively UDP. It would be interesting to follow in the steps of ZeroMQ and create `mmap` / `TCP` / `etc` style transport backends,
but I'm not trying to drink the ocean on day one.

I chose UDP initially because it serves as a completely unbiased base transport protocol. There aren't many frameworks that offer an unopinionated 
gradient between the speed/danger of UDP and safety/overhead of TCP. I want the user to be able to configure, with granularity, the exact tradeoff 
between safety/speed for their sockets. UDP allows for opinionated transport paradigms to be added or peeled away at a whim.

## Usage

#### Socket Types
- `Dealer`: Fire and forget duplex socket. When a Dealer server has multiple peers, messages sent out are fair-queued.
- `SafeDealer`: Same as Dealer socket, but frames are acknowledged by the receiver, and resent by the sender if not responded to.
The safety levels, including resend-wait, and resend count are configurable through socket options.
- `Radio`: Fire and forget, send-only, socket. Messages sent out from Radio are queued to all peers at once.
- `Dish`: Peer socket to Radio, receive only.

#### Duplex Communication

To create a duplex communication regime, we call `.send_multipart()` and `.recv_multipart()` on both sides.
Control frames are silently exchanged when these methods are called, keeping telemetry current.

```rust
use nbmq::{Socket, Dealer, AsSocket};

// Thread Event Loop 1
let mut server = Socket::<Dealer>::new().bind("0.0.0.0:8000")?;
loop {
    let some_data = ["hello".as_bytes()];

    server.send_multipart(&some_data)?;

    while let Ok(data) = server.recv_multipart() {
        // ...
    }
}

// Thread Event Loop 2
let mut client = Socket::<Dealer>::new().connect("127.0.0.1:8000")?;
loop {
    let some_data = ["world".as_bytes()];

    client.send_multipart(&some_data)?;

    while let Ok(data) = client.recv_multipart() {
        // ..
    }
}
```

#### Simplex Communication

In situations where one side of a socket pair does all the sending, and the other side
does all the receiving, we have to call `.drain_control()` on the sending socket. This
allows for ambient control frames to flow between both sides, keeping system
telemetry up to date without timers or background threads.

```rust
use nbmq::{Socket, Radio, Dish, AsSocket};

// Thread Event Loop 1
let mut server = Socket::<Radio>::new().bind("0.0.0.0:8000")?;
loop {
    let some_data = ["hello".as_bytes()];

    server.send_multipart(&some_data)?;
    
    // Since NBMQ is timerless, in order to maintain socket telemetry
    // we manually drain socket control frames when recv_multipart() is not used.
    server.drain_control()?;
}

// Thread Event Loop 2
let mut client = Socket::<Dish>::new().connect("127.0.0.1:8000")?;
loop {
    while let Ok(data) = client.recv_multipart() {
        // ..
    }
}

// Thread Event Loop N
let mut client = Socket::<Dish>::new().connect("127.0.0.1:8000")?;
loop {
    while let Ok(data) = client.recv_multipart() {
        // ..
    }
}
```
