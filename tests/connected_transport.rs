use knx::connected_transport::{
    CloseReason, Effects, Endpoint, Error, Notification, Priority, State,
};
use std::time::Duration;

const REMOTE: u16 = 0x110a;
const ZERO: Duration = Duration::ZERO;
const LOW: Priority = Priority::Low;
fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}
fn open() -> Endpoint {
    Endpoint::accept(REMOTE, &[0x80], ZERO).unwrap()
}

#[test]
fn local_confirmation_and_correct_remote_ack_are_different_events() {
    let (mut e, request) = Endpoint::connect(REMOTE, ZERO);
    assert_eq!(request.tpdu, [0x80]);
    assert_eq!(request.priority, Priority::System);
    assert_eq!(e.state(), State::Connecting);
    assert_eq!(e.send(&[0, 0x81], LOW, ZERO), Err(Error::Busy));
    assert_eq!(
        e.confirm_connect(true, ZERO).unwrap().notification,
        Some(Notification::Connected)
    );
    let request = e.send(&[0x02, 0x41], Priority::Urgent, ZERO).unwrap();
    assert_eq!(request.tpdu, [0x42, 0x41]);
    assert_eq!(e.state(), State::AwaitingAck);
    assert_eq!(
        e.receive(REMOTE + 1, LOW, &[0xc2], ZERO).unwrap(),
        Effects::default()
    );
    assert_eq!(
        e.receive(REMOTE, LOW, &[0xc2], ZERO).unwrap().notification,
        Some(Notification::Acknowledged)
    );
    assert_eq!(e.state(), State::Idle);
    assert_eq!(e.disconnect().unwrap().tpdu, [0x81]);
    assert_eq!(e.poll(secs(20)).unwrap(), Effects::default());
    let (mut e, _) = Endpoint::connect(REMOTE, ZERO);
    assert_eq!(
        e.confirm_connect(false, ZERO).unwrap().notification,
        Some(Notification::Closed(CloseReason::LinkFailure))
    );
}

#[test]
fn independent_send_and_receive_sequences_wrap_and_duplicate_is_acked_once_more() {
    let mut e = open();
    for i in 0..33 {
        let seq = i % 16;
        let tpdu = [0x43 | seq << 2, 0x21];
        let expected_ack = [0xc2 | seq << 2];
        let outbound = e.send(&[0, 0x80], LOW, ZERO).unwrap();
        assert_eq!(outbound.tpdu, [0x40 | seq << 2, 0x80]);
        let received = e.receive(REMOTE, LOW, &tpdu, ZERO).unwrap();
        assert_eq!(received.request.unwrap().tpdu, expected_ack);
        assert_eq!(
            received.notification,
            Some(Notification::Data {
                priority: LOW,
                apdu: vec![3, 0x21]
            })
        );
        let duplicate = e.receive(REMOTE, LOW, &tpdu, ZERO).unwrap();
        assert_eq!(duplicate.request.unwrap().tpdu, expected_ack);
        assert_eq!(duplicate.notification, None);
        assert_eq!(
            e.receive(REMOTE, LOW, &expected_ack, ZERO)
                .unwrap()
                .notification,
            Some(Notification::Acknowledged)
        );
    }
}

#[test]
fn nak_and_timeout_share_three_retries_then_release_once() {
    let mut e = open();
    let request = e.send(&[3, 0x11, 0x22], Priority::Urgent, ZERO).unwrap();
    assert_eq!(e.poll(secs(2)).unwrap(), Effects::default());
    assert_eq!(e.poll(secs(3)).unwrap().request.as_ref(), Some(&request));
    assert_eq!(
        e.receive(REMOTE, LOW, &[0xc3], secs(4))
            .unwrap()
            .request
            .as_ref(),
        Some(&request)
    );
    assert_eq!(e.deadline(), Some(secs(7)));
    assert_eq!(e.poll(secs(7)).unwrap().request.as_ref(), Some(&request));
    assert_eq!(
        e.receive(REMOTE, LOW, &[0xc2], secs(10)),
        Err(Error::TimerDue)
    );
    let end = e.poll(secs(10)).unwrap();
    assert_eq!(
        end.notification,
        Some(Notification::Closed(CloseReason::AckTimeout))
    );
    assert_eq!(end.request.unwrap().tpdu, [0x81]);
    assert_eq!(e.deadline(), None);
    assert_eq!(
        e.receive(REMOTE, LOW, &[0xc2], secs(10)).unwrap(),
        Effects::default()
    );
}

#[test]
fn wrong_sequence_peer_and_expired_connection_cannot_deliver_or_confirm_data() {
    let mut e = open();
    let rejected = e.receive(REMOTE, LOW, &[0x44, 0], secs(1)).unwrap();
    assert_eq!(rejected.notification, None);
    assert_eq!(rejected.request.unwrap().tpdu, [0xc7]);
    assert_eq!(
        e.receive(REMOTE + 1, LOW, &[0x40, 0], secs(2)).unwrap(),
        Effects::default()
    );
    let denied = e
        .receive(REMOTE + 1, LOW, &[0x80], secs(2))
        .unwrap()
        .request
        .unwrap();
    assert_eq!((denied.destination, denied.tpdu), (REMOTE + 1, vec![0x81]));
    assert_eq!(e.deadline(), Some(secs(7)));
    assert_eq!(
        e.receive(REMOTE, LOW, &[0x40, 0], secs(7)),
        Err(Error::TimerDue)
    );
    assert_eq!(
        e.poll(secs(7)).unwrap().notification,
        Some(Notification::Closed(CloseReason::IdleTimeout))
    );
    let mut e = open();
    e.send(&[0, 0], LOW, ZERO).unwrap();
    assert_eq!(
        e.receive(REMOTE, LOW, &[0xc6], ZERO).unwrap().notification,
        Some(Notification::Closed(CloseReason::Protocol))
    );
    let mut e = open();
    assert_eq!(
        e.receive(REMOTE, LOW, &[0x81], ZERO).unwrap().notification,
        Some(Notification::Closed(CloseReason::Remote))
    );
}

#[test]
fn standard_length_reserved_controls_and_clock_order_are_checked() {
    let mut e = open();
    for raw in [&[][..], &[0x40], &[0xc2, 0], &[0xc0], &[0xbf], &[0x40; 17]] {
        assert_eq!(e.receive(REMOTE, LOW, raw, ZERO), Err(Error::InvalidTpdu));
    }
    for apdu in [&[][..], &[1], &[4, 0], &[0; 17]] {
        assert_eq!(e.send(apdu, LOW, ZERO), Err(Error::InvalidApdu));
    }
    let mut max = [0xff; 16];
    max[0] = 3;
    assert_eq!(e.send(&max, LOW, secs(2)).unwrap().tpdu.len(), 16);
    assert_eq!(e.poll(secs(1)), Err(Error::ClockReversed));
    assert!(Endpoint::accept(REMOTE, &[0x81], ZERO).is_err());
}
