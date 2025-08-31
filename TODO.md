# TODO

- ✅ Track connections by a library-generated session ID instead of SocketAddr. SocketAddr is subject to change via NAT reassignment and stuff.
    - Session IDs can be a hash of SocketAddr and current timestamp. meaning ill need a get_ts function.
    - ended up as a hash of a random number seeded by timestamp + socketaddr
- ✅ Split session_id, kind (control/userdata), message_id (hash) into separate fields. stop overloading the message_hash field. update to proto 0.2.0
- ✅ Dont use softmax for queueing/sending. very temperature sensitive. split evenly
    - eventually use deficit round robin.
- ✅ Make use random (XORShift) in session IDs. 
- ✅ Flesh out maint() so liveness can be driven solely by tick, in liu of any send or recv operations. should be a backup mechanism, user io driving should be preferred.
- ✅ Control frames dont need to be the same size/structure as frame. if kind is different I can pretty easily parse it differently, or create another struct.
- ✅ Send disconnect for ANY frame that comes in, where the peer addr is not recognized. not just heartbeats.
- ✅ hwm test - send side and recv side per each socket type
- ✅ If tick fails on recv with high water mark reached, it doesnt continue calling core.recv. i should drain core, and then return an error afterwards
    - This also has an effect on Control/Liveness tracking. if i exit the recv loop early after a HWM reached error, i neglect to pull control frames from the kernel.
    - OPTIONS:
        - a) ✅ Drain recv queue fully each time per tick, backpressure is mainly handled by send_hwm and max_tick_send. I can't safeguard against bad topologies.
        - b) ✅ Remove max_tick_recv.
        - c) ✅ If the recv_hwm is hit, drain the kernel buffer before raising the error.
- Add an Encryptor trait that users can extend to add p2p encryption using whatever scheme they want.
- Test every state change path of core.rs, use coverage to confirm
- Add a STUN server socket
- Add socket names, so a STUN socket can provide an index of connected peers

## Notes

 - HANDSHAKE: Client sends control frame (connect) to bound socket, socket creates session id, sends back with control frame (accepted), client receives and sends heartbeat
 - maint()
