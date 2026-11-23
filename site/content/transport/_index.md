+++
title = "KNX interfaces in Rust"
description = "Discover a KNX/IP gateway, choose tunnelling or routing, and exchange plain or authenticated group telegrams."
template = "guide.html"
+++

## Read a group telegram

Gateways carry link-layer messages in a common format called cEMI. Remove the transport envelope and pass one complete cEMI record to `GroupTelegram::parse`. Match the destination to a commissioned point before decoding. A group read has no value, and a link confirmation reports delivery rather than the physical state of a device.

```rust
use knx::{GroupAddress, decode_switch, telegram::GroupTelegram};

let status: GroupAddress = "4/3/2".parse().unwrap();
let frame = [0x29, 0, 0xbc, 0xe0, 0xff, 0xff, 0x23, 2, 1, 0, 0x81];
let telegram = GroupTelegram::parse(&frame).unwrap();
assert_eq!(telegram.destination, status);
let payload = telegram.indicated_value().unwrap();
assert_eq!(decode_switch(payload.as_bytes()), Ok(true));
```

Group reads, writes and responses support inline values and 1–253 separate application octets. The encoder selects the extended frame format above 14 octets. Before sending, check the smallest supported APDU length along the commissioned path, including the gateway, couplers and destination. [KNX APDU length guidance](https://support.knx.org/hc/en-us/articles/16292302904850-APDU-length-error-message)

For an archived cEMI bus-monitor indication, use `telegram::BusMonitorIndication::parse`. Inspect the clock width, sequence number and link-error flags before using `raw_frame()` for diagnostics. The relative timestamp comes from the capturing interface. A capture can contain a damaged frame.

Select `media::Medium` for TP, PL110 or asynchronous RF host indications. For PL110, supply the commissioned domain number (1–255). The decoder checks both wire octets and retains the optional relative timestamp. RF checks the six-byte domain and retains the frame number.

For coupler diagnostics, read the observed routing counter from a media indication. `media::GroupFilterTable::entry` inspects a commissioned filter bitmap in filter mode. A legacy table returns `None` outside main groups 0–13. Confirm delivery from an egress capture. PL110 supports main groups 0–15.

## Exchange individual connected TPDUs

Use `connected_transport::Endpoint::connect` for one selected individual address. Send the returned request through the network layer, then pass its local confirmation to `confirm_connect`. An incoming connect can instead be admitted with `Endpoint::accept`.

Submit a 2–16 byte application PDU with its six transport bits clear. Dispatch each returned request with the indicated priority. Only `Notification::Acknowledged` confirms the peer accepted the numbered PDU. Deliver individual indications addressed to your device through `receive` and call `poll` at `deadline` using elapsed monotonic time. A closed connection requires a fresh endpoint.

## Select a KNXnet/IP gateway

Use `ip_core::describe` for a known UDP/IPv4 control endpoint. For discovery, give `ip_core::discover` a socket configured for the selected local IPv4 interface, a deadline and a cancellation callback. Inspect the gateway’s advertised service version before connecting. Discovery replies are unauthenticated.

Use `ip_core::ExtendedSearch` for Core v2 gateways. Set `service` to the required family and minimum version, or select a particular MAC address or a gateway in programming mode. The collector checks those criteria against each reply. Request `manufacturer_data` for vendor-defined records. On Weinzierl gateways, `baos()` reads the advertised ObjectServer version and any required IP security version.

## Send and receive encrypted group traffic

Enable the `data-secure` Cargo feature. Configure `data_secure::GroupReceiver` with an authorized sender, commissioned group address and 16-byte group key. Pass each cEMI indication to `receive`. A successful result provides the application service and group-layer protection evidence for `DeliveryPolicy::check`. IP-layer protection is evaluated separately.

Implement `SequenceStore::advance` as an atomic compare-and-update in your durable store. Scope normal-mode sender sequences to the installation and individual address, across groups and keys. Seed the baseline during commissioning or trusted synchronization. Return success only after the higher sequence commits. Restoring old counter state can admit already processed frames.

To send a group read, response or write, configure `GroupSender` with the local individual address, group and key. Its `request` method returns a sealed cEMI request and accepts up to 240 external application octets. Implement `OutgoingSequenceStore::reserve` as a durable atomic increment shared by every group using that source. Return an error if the commissioned counter is missing and `Ok(None)` when its 48-bit range is exhausted. Keep used reservations across restarts, dropped requests and key rotations.

For tunnelling, use the gateway-assigned source when constructing the sender and pass the resulting request to `Tunnel::secure_request`. An acknowledgement retry resends the staged bytes.

### Synchronize a sender’s group sequences

Use `data_secure::sync::SyncPeer` with the local individual address, a commissioned peer’s individual address and their point-to-point key. Send the result of `request` through your link and give the returned `PendingSync` the answering indication within six seconds. Finish or discard pending requests before calling `replace_key`. For incoming requests, share one `SyncResponder` across the local device’s peers.

Implement `SyncStore::reconcile` in the same durable store as group sends and receives. Atomically raise both **next** counters and preserve higher values. A matched response can recover missing incoming state. An unsolicited request requires a trusted incoming baseline, and every new request requires a trusted local sending counter.

## Open a KNXnet/IP Secure TCP session

Enable `ip-secure` and connect a TCP stream to the commissioned gateway. Supply `ip_secure::Handshake::start` with the client's unique six-byte serial number, user ID (1–127), and the device and user 16-byte credential hashes. Load these hashes from your commissioned secret store.

1. Write `handshake.request()`. Read the six-byte KNXnet/IP header and the remaining bytes indicated by its big-endian total length.
2. Pass the complete response to `handshake.authenticate`, write the returned authentication frame, and pass the protected status to `pending.finish`. Each handshake phase expires after 10 seconds.
3. Use `session.send` and `session.receive` for core description, connection management and tunnelling frames. A link-layer tunnel uses TCP route-back HPAIs. On `Received::Frame`, use `as_bytes()` and its per-message `ip_protection()` evidence.
4. Send `keepalive()` after 15 seconds without outgoing traffic. The client expires after 30 idle seconds. Write `close()` during teardown and discard the session when the stream closes.

One secure wrapper holds up to 4,080 inner octets (4,118 on the wire). Treat a short TCP read as part of the current frame. The checkout includes a working TCP client in `tools/interop/ip_secure_client.rs`.

## Drive a plain link-layer tunnel

1. Send `tunnel::connect_request` to the selected control endpoint. Pass the actual UDP source and response to `Tunnel::accept`.
2. Send each `Tunnel::request` to the negotiated data endpoint. Arm `ACK_TIMEOUT` after sending. On timeout, send the bytes returned by `ack_timeout` once. If it returns `Indeterminate`, disconnect and reconcile the physical state before issuing another command.
3. Feed incoming data datagrams to `receive`. Send the returned acknowledgement for both a new indication and a duplicate. `Unhandled` carries a complete cEMI message outside the group decoder. `Acknowledged` confirms gateway receipt.
4. Schedule a heartbeat every 60 seconds, with a 10-second response timeout. Follow the returned retry or disconnect action. Handle server disconnects on the control endpoint and establish a fresh session after closure.

The application owns socket I/O, cancellation and timers. `DeliveryPolicy` can check IP and group protection evidence from a trusted external verifier. The plain tunnel provides neither authentication nor encryption.

## Send and receive routing events

Join the commissioned multicast address with `routing::RoutingMembership`. Pass each datagram, its source and actual ingress interface to the matching `RoutingReceiver`. Handle indications, lost-message counts and busy intervals separately. Dropping the membership leaves the group.

To send, bind a UDP socket to a concrete local IPv4 address and set its outgoing multicast interface. Pass it, the routing group and the application’s commissioned individual address to `RoutingSender::new`. Feed incoming routing datagrams to its `receive` method so busy messages take effect. `send` either submits one group service to UDP or returns `RetryAt`. Keep the message in your application and retry after that deadline, processing new busy reports in the meantime. UDP submission carries no bus acknowledgement.

On Linux, enable the `linux-routing` feature and pass your configured receive socket to `LinuxRoutingSocket::new`. Each complete datagram carries its IP destination, ingress address and interface index. Check the destination against the routing group before calling `receive` on the sender or receiver. The optional `socket_drop_total` is the kernel’s cumulative socket loss. Compute wrapping differences on samples from the same socket and report them separately from a router’s lost-message count.

## Exchange authenticated multicast

Enable `secure-routing` and select a trusted group clock. Create one `secure_routing::SecureRoutingReceiver` per commissioned sender with the ingress interface index, multicast address, sender serial, backbone key, latency tolerance and current trusted group time in milliseconds.

Pass the actual interface index, destination socket address (multicast IP and UDP port 3671), current group time and datagram to `receive`. Use `event()` to handle the verified indication, loss or busy report. `ip_protection()` supplies the IP-layer evidence for this delivery. The shared key authenticates backbone membership. Check group Data Secure separately.

Preserve trusted group time across restarts and any restoration of a previously used backbone key. Each receiver accepts messages timestamped after its startup sample. Pass `None` while synchronization is unavailable. The replay window holds up to 512 timestamp/tag pairs per sender and rejects additional messages until older entries expire. An external KNX timer or the built-in `GroupClock` can supply synchronized time. [KNXnet/IP standard](https://www.iso.org/standard/73364.html)

To originate a group service, select the outgoing multicast interface on a bound IPv4 socket and give it to `SecureRoutingSender::new`. Reserve a dedicated serial for that sender across processes and key changes. Pass each peer's datagrams through `sender.receive` with its commissioned receiver so authenticated busy reports update the pacing deadline. On `RetryAt`, keep the request and sample group time again when retrying. A socket error consumes the attempted timestamp.

### Synchronize the multicast timer

Use `GroupClock` when the application participates in Timer Notify. Give it the commissioned backbone key, membership, local notification source, dedicated timer serial and ETS latency settings. Implement `TimerStore` to atomically persist the reserved upper bound for each previously used key. Use `TimerOrigin::NewKey` only on its first commissioning.

Drive `poll` at `deadline()` and submit each due notification on the configured socket. Pass incoming datagrams to `observe` with their actual interface, destination and source. While `sample` returns `None`, hold application traffic. Once acquired, create peer receivers with `clock.receiver`, pass the time returned by `observe` to their receive path, and call `wrapper_sent` after a successful routing send. A persistence failure requires reopening the durable store and clock.

## Bind a sensor or actuator

For an LSAB switching actuator, `lighting::SwitchActuator` binds the command input and optional feedback output. `observed` accepts feedback only from the configured actuator and destination.

For an FB 418 dimmer, give `lighting::DimmingActuator::new` the commissioned switch, relative and absolute command groups. Use `dim` for a relative step and `set_level` with the byte returned by `encode_scaling_percent` for an absolute level. Bind the optional actual-level group with `with_level_feedback` to read observed percentages.

For a basic sunblind actuator (FB 800), bind the DPT 1.008 travel group and DPT 1.007 stop/step group with `shading::SunblindActuator::new`. Use `travel` for continuous motion. Use `stop_or_step` for a short operation whose effect depends on whether the actuator is moving.

For a room (FB 321) or outside (FB 320) temperature point, bind `temperature::TemperatureSensor` to its commissioned individual address and output group. `observed_celsius` reads a matching DPT 9.001 write or response. Set `read_enabled` from the object’s ETS read flag. A sensor host can call `publish` for a sample and pass incoming reads to `respond_to_read`. A disabled read flag yields no reply.

## Read a KNX IoT switch

Bind `iot::SwitchPoint::new` to the commissioned IPv6 endpoint, decoded Uri-Path segments (for example, `&["p", "lsab", "0", "ioo"]`) and access context. Call `point.get(PointFormat::Cbor, token).read(&mut transport)` with a fresh eight-byte token from your CoAP stack. Implement `PointTransport` to send the value-only GET. An asynchronous client can await its exchange and pass a `PointReply` to `complete`.

For a protected read, the host transport applies the selected OSCORE context and verifies the inner response before supplying its nonsecret context handle. The remote resource authorizes the authenticated reader. The checkout’s `tools/interop/switch_client.rs` shows a Rust host adapter handling both modes. Check `PointError::ResponseCode(131)` for CoAP 4.03 Forbidden. The reader matches peer, token, context and negotiated format before exposing a switch value. [KNX Point API](https://buildwithknxiot.knx.org/public-projects/knx-iot-docs/pointapi/introduction/)
