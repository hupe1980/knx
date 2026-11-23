use knx::telegram::{GroupService, GroupTelegram};
use knx::tunnel::*;
use std::net::{Ipv4Addr, SocketAddrV4};

const PEER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671);
const RESPONSE: [u8; 20] = [
    6, 16, 2, 6, 0, 20, 21, 0, 8, 1, 127, 0, 0, 1, 14, 87, 4, 4, 17, 42,
];
const ACK: [u8; 10] = [6, 16, 4, 33, 0, 10, 4, 21, 0, 0];
const IND: [u8; 21] = [
    6, 16, 4, 32, 0, 21, 4, 21, 0, 0, 0x29, 0, 0xbc, 0xe0, 0x11, 2, 0x23, 2, 1, 0, 0x81,
];

#[test]
fn session_negotiation_and_terminal_outcomes() {
    assert_eq!(&connect_request(PEER, PEER).unwrap()[22..], &[4, 4, 2, 0]);
    let mut channel = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    assert_eq!(channel.assigned_address(), 0x112a);
    assert!(!knx::ProtectionRequirement::SecureRequired.accepts(channel.ip_protection()));
    assert_eq!(channel.data_endpoint(), PEER);
    assert!(matches!(
        Tunnel::accept(PEER, PEER, &[6, 16, 2, 6, 0, 8, 0, 0x24]),
        Err(TunnelError::Refused(0x24))
    ));
    let wrong = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 999);
    assert!(Tunnel::accept(PEER, wrong, &RESPONSE).is_err());
    assert_eq!(channel.heartbeat(PEER).unwrap()[3], 7);
    channel
        .heartbeat_response(PEER, &[6, 16, 2, 8, 0, 8, 21, 0])
        .unwrap();
    let req = GroupTelegram::request(0, "1/1/1".parse().unwrap(), GroupService::Read);
    let packet = channel.request(&req).unwrap().to_vec();
    assert_eq!(channel.request(&req), Err(TunnelError::Busy));
    assert_eq!(channel.receive(wrong, &ACK), Err(TunnelError::Unexpected));
    assert_eq!(channel.ack_timeout(), Ok(Event::Retry(&packet)));
    assert_eq!(channel.ack_timeout(), Ok(Event::Indeterminate));
    assert_eq!(channel.request(&req), Err(TunnelError::Closed));
    assert_eq!(channel.disconnect(PEER).unwrap()[3], 9);
}

#[test]
fn ordered_indications_duplicates_and_wrap_are_independent_of_ack() {
    let mut channel = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    let req = GroupTelegram::request(0, "1/1/1".parse().unwrap(), GroupService::Read);
    for seq in (0..=255).chain([0]) {
        let mut incoming = IND;
        incoming[8] = seq;
        let mut future = incoming;
        future[8] = seq.wrapping_add(1);
        assert_eq!(channel.receive(PEER, &future), Ok(Event::Discard));
        let mut ack = ACK;
        ack[8] = seq;
        assert_eq!(channel.request(&req).unwrap()[8], seq);
        assert!(
            matches!(channel.receive(PEER,&incoming),Ok(Event::Incoming{ack:a,telegram:Some(t)}) if a==ack && t.indicated_value().unwrap().as_bytes()==[1])
        );
        assert_eq!(
            channel.receive(PEER, &incoming),
            Ok(Event::Incoming {
                ack,
                telegram: None
            })
        );
        assert!(channel.has_pending_request());
        assert_eq!(channel.receive(PEER, &ack), Ok(Event::Acknowledged));
    }
}

#[test]
fn unrelated_cemi_does_not_stall_the_channel() {
    let mut channel = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    let mut unrelated = IND;
    unrelated[13] &= 0x7f;
    assert_eq!(
        channel.receive(PEER, &unrelated),
        Ok(Event::Unhandled {
            ack: ACK,
            cemi: &unrelated[10..]
        })
    );
    assert_eq!(
        channel.receive(PEER, &unrelated),
        Ok(Event::Incoming {
            ack: ACK,
            telegram: None
        })
    );
    let mut next = IND;
    next[8] = 1;
    assert!(matches!(
        channel.receive(PEER, &next),
        Ok(Event::Incoming {
            telegram: Some(_),
            ..
        })
    ));
}

#[test]
fn heartbeat_retries_three_times_then_disconnects_with_uncertain_delivery() {
    let mut c = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    let negative = [6, 16, 2, 8, 0, 8, 21, 0x21];
    assert_eq!(
        c.heartbeat_response(PEER, &negative),
        Err(TunnelError::Unexpected)
    );
    c.request(&GroupTelegram::request(
        0,
        "1/1/1".parse().unwrap(),
        GroupService::Read,
    ))
    .unwrap();
    let beat = c.heartbeat(PEER).unwrap();
    assert_eq!(
        c.heartbeat_timeout(),
        Ok(ControlEvent::RetryHeartbeat(beat))
    );
    for _ in 0..2 {
        assert_eq!(
            c.heartbeat_response(PEER, &negative),
            Ok(ControlEvent::RetryHeartbeat(beat))
        );
    }
    assert!(
        matches!(c.heartbeat_timeout(),Ok(ControlEvent::Disconnect{request,delivery_uncertain:true}) if request[3]==9)
    );
    assert!(!c.has_pending_request());
    assert_eq!(c.heartbeat(PEER), Err(TunnelError::Closed));
}

#[test]
fn server_disconnect_reports_the_pending_command() {
    let mut c = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    c.request(&GroupTelegram::request(
        0,
        "1/1/1".parse().unwrap(),
        GroupService::Read,
    ))
    .unwrap();
    let packet = [6, 16, 2, 9, 0, 16, 21, 0, 8, 1, 127, 0, 0, 1, 14, 87];
    assert_eq!(
        c.remote_disconnect(PEER, &packet),
        Ok(ControlEvent::Disconnected {
            response: [6, 16, 2, 10, 0, 8, 21, 0],
            delivery_uncertain: true
        })
    );
    assert!(!c.has_pending_request());
}

#[test]
fn maximum_group_record_preserves_both_ip_length_octets_across_retry_and_receive() {
    use knx::telegram::GroupData;
    let mut channel = Tunnel::accept(PEER, PEER, &RESPONSE).unwrap();
    let mut request = GroupTelegram::request(
        0x112a,
        "1/2/1".parse().unwrap(),
        GroupService::Response(GroupData::Bytes(&[9; 253])),
    );
    request.relative_timestamp = Some(0x1234);
    let mut datagram = channel.request(&request).unwrap().to_vec();
    assert_eq!(datagram.len(), 278);
    assert_eq!(&datagram[4..6], &[1, 0x16]);
    assert_eq!(channel.ack_timeout(), Ok(Event::Retry(&datagram)));
    datagram[10] = 0x29;
    assert!(
        matches!(channel.receive(PEER,&datagram),Ok(Event::Incoming{telegram:Some(t),..})
        if t.indicated_value().unwrap().as_bytes()==[9;253] && t.relative_timestamp == Some(0x1234))
    );
}
