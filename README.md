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

## High Level Architecture

The architecture is split into three major parts:

- `Connection Layer`: manages liveness and peer awareness through control frame kinds 1..4, returns data frames and message-level control frames to the messaging layer above.
- `Queueing`: handles conversion of binary multipart messages into wire frames, and reassembly on the receiving end. includes methods for opt-in delivery guarantee handling.
- `Messaging`: user API level, unites the connection and queuing components in a number of ways to achieve different messaging and delivery behaviors.

## Wire Format

### DataFrame (v0.2.0)

| Field           | Size (bytes) | Description                                                   |
|-----------------|--------------|---------------------------------------------------------------|
| **version**     | 1            | Protocol version (`0x01`)                                     |
| **kind**        | 1            | Frame kind (`0 = DataFrame`)                                  |
| **session_id**  | 8            | Session identifier (u64, big-endian)                          |
| **message_id**  | 8            | Message identifier (u64, big-endian)                          |
| **part_count**  | 1            | Total number of parts in this message                         |
| **part_index**  | 1            | Index of this part (0-based)                                  |
| **message_size**| 4            | Total size of the full message (bytes)                        |
| **part_size**   | 4            | Size of this part (bytes)                                     |
| **chunk_size**  | 2            | Size of this chunk (bytes)                                    |
| **chunk_offset**| 4            | Offset of this chunk within the part                          |
| **data**        | variable     | Payload data (`chunk_size` bytes)                             |

**Header length:** 34 bytes  
**Max frame size:** 500 bytes  
**Max data size per frame:** 466 bytes  

### ControlFrame (v0.2.0)

| Field          | Size (bytes) | Description                                                   |
|----------------|--------------|---------------------------------------------------------------|
| **version**    | 1            | Protocol version (`0x01`)                                     |
| **kind**       | 1            | Control frame type                                            |
| **session_id** | 8            | Session identifier (u64, big-endian)                          |
| **data**       | variable     | Optional payload (depends on kind, e.g. `Ack`)                |

**Header length:** 10 bytes  

**Kinds:**
- `1` → `Connect` (no data) 
- `2` → `Connected(session_id)`  
- `3` → `Disconnected(session_id)`  
- `4` → `Heartbeat(session_id)`  
- `5` → `Ack(session_id, chunk)` where `chunk` is an identifier of the frame sent, created and ingested by messaging layer sockets. 

## Connection Flow

#### Handshake

1. Client socket A connects, and sends `Connect` control frame to bound peer B.
2. B receives `Connect` frame, derives a socket id from the initial socket address of A, and the time of connection. B adds A internally as a peer.
3. B sends a `Connected(session_id)` frame back to A, confirming the connection.
4. A receives this `Connected(session_id)` frame, and adds B as a peer.
5. A sends a `Heartbeat(session_id)` frame to A, signifying the connection is in place.

#### Liveness

- The sockets exchange heartbeats periodically, if one side stops sending heartbeats, the other side removes the peer from its internal cache.
- If the removed peer suddenly sends another message, or heartbeat, it is sent a `Disconnected(session_id)` frame.
- Upon receiving a `Disconnected(session_id)` frame, the slow peer will respond with a `Connect` frame, attempting to reinitiate the connection through a new handshake.
- The heartbeat interval, peer timeout durtion, and reconnect wait duration are all configurable through socket configuration.

```rust
let socket = Socket::<Dealer>::new()
    .set_peer_reconnect_wait(0.5)   // seconds in f64
    .set_peer_heartbeat_ivl(0.1)    // ...
    .set_peer_keepalive(1.)         // ...
    .connect("127.0.0.1:8080")?;
```

## Usage

### Socket Types
- **Dealer** → Fire and forget duplex socket. When a Dealer server has multiple peers, messages sent out are fair-queued.
- **SafeDealer** → Same as Dealer socket, but frames are acknowledged by the receiver, and resent by the sender if not responded to.
The safety levels, including resend-wait, and resend count are configurable through socket options.
- **Radio** → Fire and forget, send-only, socket. Messages sent out from Radio are queued to all peers at once.
- **Dish** → Peer socket to Radio, receive only.

### `AsSocket` Trait

All socket types extend the `AsSocket` trait. The primary methods used for communication are:

- `socket.send(data: &[[u8]])`: Intakes a multipart binary message, and populates the socket's internal send queue. This shards the message into many DataFrames. Each peer of a socket has its own send queue.
- `socket.recv()`: Pulls a reassembled message out of the socket's receive queue.
- `socket.tick()`: Pulls a number of received frame buffers from the underlying connection-level UDP socket, and parses them into Frames. Received control frames update the connection and liveness of the sockets peers. Received data frames are fed into the socket's internal receive queue, and gradually reassembled.

Because the design is timerless, to maintain state, `.tick()` needs to be called once per each iteration of the event loop for every active socket.

### Socket Options

- **send_hwm**: `usize` The maximum number of messages a socket's send queue can hold in memory before throwing `WouldBlock`
- **recv_hwm**: `usize` The maximum number of messages a socket's receive queue can hold in memory before throwing `WouldBlock`
- **safe_resend_limit**: `usize` The maximum number of times a DataFrame is allowed to be resent to a peer. Applies to Safe* sockets only.
- **max_tick_send**: `usize` The maximum number of frames that can be pushed from a socket's send queue, into the underlying raw socket, during a call of `.tick()`.
- **max_tick_recv**: `usize` The maximum number of frames that can be pulled from the underlying raw socket, into the socket's recv queue, during a call of `.tick()`.
- **uncompleted_message_ttl**: `f64` The maximum amount of time, in seconds, that a partially complete message will be kept around in memory before being discarded.
- **queue_maint_ivl**: `f64` The interval, in seconds, of queue cleanup.
- **peer_heartbeat_ivl**: `f64` The interval, in seconds, that a socket will send a peer a heartbeat.
- **peer_keepalive**: `f64` The maximum amount of time, in seconds, a peer is retained after going silent.
- **reconnect_wait**: `f64` The amount of time a connecting socket will wait before trying to reconnect to it's bound peer.
- **safe_resend_ivl**: `f64` The amount of time a safe socket will wait before resending an unacknowledged frame.
- **safe_hash_dedup_ttl**: `f64` The amount of time a safe socket will retain a frame hash, for the purpose of deduplicating repeated wire messages.

### Duplex Example

Control frames are silently exchanged during calls of `.send()` and `.recv()`, falling back on `.tick()` during periods of inactivity, for connection telemetry.

```rust
use nbmq::{Socket, Dealer, AsSocket};

// Thread Event Loop 1
let mut server = Socket::<Dealer>::new().bind("0.0.0.0:8000")?;
loop {
    let some_data = ["hello".as_bytes()];

    server.send(&some_data)?;

    while let Ok(data) = server.recv() {
        // ...
    }

    server.tick()?;
}

// Thread Event Loop 2
let mut client = Socket::<Dealer>::new().connect("127.0.0.1:8000")?;
loop {
    let some_data = ["world".as_bytes()];

    client.send(&some_data)?;

    while let Ok(data) = client.recv() {
        // ..
    }

    client.tick()?;
}
```

### Simplex Example

Socket types `Radio` and `Dish` are unidirectional, but control frame flow is still bidirectional. Architecturally, unidirectional sockets share the same communication layer 
as bidirectional sockets. The difference is within the messaging layer. For example, the `Radio` socket has no receive queue and the `Dish` socket has no send queue.

```rust
use nbmq::{Socket, Radio, Dish, AsSocket};

// Thread Event Loop 1
let mut broadcast = Socket::<Radio>::new().bind("0.0.0.0:8000")?;
loop {
    let some_data = ["hello".as_bytes()];

    broadcast.send(&some_data)?;

    broadcast.tick()?;
}

// Thread Event Loop 2
let mut receiver = Socket::<Dish>::new().connect("127.0.0.1:8000")?;
loop {
    while let Ok(data) = receiver.recv() {
        // ..
    }

    receiver.tick()?;
}
```
