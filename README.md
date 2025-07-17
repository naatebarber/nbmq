# NBMQ

UDP first, timerless messaging queue for a fun, lightning fast, fire-and-forget messaging style with event-stepped connection awareness.

This library was inspired by ZeroMQ, specifically ZeroMQ using the `send_multipart() / recv_multipart()` paradigm along with the `ZMQ::DONTWAIT` flag.
I really enjoy ZeroMQ under this paradigm - never having to worry about blocking when a HWM is reached, failing hard when pressure is too high, etc.
This library is just meant to take this paradigm further, relinquishing more control of delivery, and providing more transparency to internal state. It's 
intended to be more like a toolkit than a plug and play solution, but if you're anything like me, that's a positive.

The entirety of NBMQ is state-stepped when a user performs an operation on a socket. Peer health is managed this way. Backpressure control and message insurance will
follow. No background threads, no hidden state, no bullshit.
