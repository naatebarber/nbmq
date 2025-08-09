# TODO

- Track connections by a library-generated session ID instead of SocketAddr. SocketAddr is subject to change via NAT reassignment and stuff.
- Split session_id, kind (control/userdata), message_id (hash) into separate fields. stop overloading the message_hash field. update to proto 0.2.0
- Dont use softmax for queueing/sending. very temperature sensitive. use deficit round robin.
