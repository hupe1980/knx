#![cfg(feature = "secure-routing")]

use knx::{
    ProtectionEvidence,
    routing::RoutingEvent,
    secure_routing::{SecureRoutingError as E, SecureRoutingReceiver},
};
use std::net::{Ipv4Addr, SocketAddrV4};

const GROUP: Ipv4Addr = Ipv4Addr::new(239, 1, 2, 3);
const ENDPOINT: SocketAddrV4 = SocketAddrV4::new(GROUP, 3671);
const TIME: u64 = 0xc0c1c2c3c4c5;
const SERIAL: [u8; 6] = [0, 0xfa, 0x12, 0x34, 0x56, 0x78];

// AN159v06 routing example followed by Calimero-generated variations.
const IND: &str = "0610095000370000c0c1c2c3c4c500fa12345678affeb7ee7e8a1c2f7bbabec775fd6e10d0bc4b7212a03aaae49da85689774c1d2b4da4";
const LOST: &str = "0610095000300000c0c1c2c3c4c500fa12345678afff4364d4a02aa344763c45cbb9e1a8a12872741eb387c55fde4c35";
const BUSY: &str = "0610095000320000c0c1c2c3c4c400fa1234567800016f508364be49b97578823294fd322d7f89f294be0c348a8ce9b7db8e";
const UNSUPPORTED: &str =
    "06100950002c0000c0c1c2c3c4c600fa123456780002010ffae7c409d63713da98f537b22225d9849a4ab15e";
const NEXT: &str = "0610095000370000c0c1c2c3c4c600fa123456780002010ffdd6c41e913055eb370a92a67f277c2f434c2abdf3aed645958966870b7b93";
const ROTATED: &str = "0610095000370000c0c1c2c3c4c700fa123456780002ad4eb8aa37d28f98d9e7349fc896c5624397c32b60772113f1fe2ff149f4a1f48e";
const RESTORED: &str = "0610095000370000c0c1c2c3c4c800fa123456780003d814c52d5f314b5def438a8e6a9ab310df04afd8eccec206f5762bd831980777b6";
fn bytes(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect()
}

fn receiver(start: u64, key: u8) -> SecureRoutingReceiver {
    SecureRoutingReceiver::new(
        2,
        GROUP,
        SERIAL,
        std::array::from_fn(|i| key + i as u8),
        100,
        start,
    )
    .unwrap()
}

#[test]
fn an159_group_write_and_independent_calimero_controls() {
    let mut rx = receiver(TIME - 2, 0);
    let value = rx.receive(2, ENDPOINT, Some(TIME), &bytes(IND)).unwrap();
    assert_eq!(
        value.ip_protection(),
        ProtectionEvidence::AuthenticatedFresh
    );
    let RoutingEvent::Indication(telegram) = value.event() else {
        panic!()
    };
    assert_eq!(telegram.source, 0x1159);
    assert_eq!(u16::from(telegram.destination), 0x0ade);
    assert_eq!(telegram.indicated_value().unwrap().as_bytes(), &[1]);
    assert!(matches!(
        rx.receive(2, ENDPOINT, Some(TIME), &bytes(LOST))
            .unwrap()
            .event(),
        RoutingEvent::Lost { messages: 3, .. }
    ));
    assert!(matches!(
        rx.receive(2, ENDPOINT, Some(TIME), &bytes(BUSY))
            .unwrap()
            .event(),
        RoutingEvent::Busy { control: 1, .. }
    ));
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME), &bytes(IND))
            .unwrap_err(),
        E::Replay
    );
}

#[test]
fn authenticates_envelope_and_inner_before_committing_replay() {
    let mut rx = receiver(TIME - 2, 0);
    let p = bytes(IND);
    for offset in [3, 5, 8, 20, 22, p.len() - 1] {
        let mut modified = p.clone();
        modified[offset] ^= 1;
        assert_eq!(
            rx.receive(2, ENDPOINT, Some(TIME), &modified).unwrap_err(),
            E::Authentication
        );
    }
    for offset in [7, 15] {
        let mut modified = p.clone();
        modified[offset] ^= 1;
        assert_eq!(
            rx.receive(2, ENDPOINT, Some(TIME), &modified).unwrap_err(),
            E::Binding
        );
    }
    assert_eq!(
        rx.receive(1, ENDPOINT, Some(TIME), &p).unwrap_err(),
        E::Binding
    );
    assert_eq!(
        rx.receive(
            2,
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671),
            Some(TIME),
            &p
        )
        .unwrap_err(),
        E::Binding
    );
    assert_eq!(
        rx.receive(2, SocketAddrV4::new(GROUP, 1234), Some(TIME), &p)
            .unwrap_err(),
        E::Binding
    );
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME), &[0; 1024]).unwrap_err(),
        E::Authentication
    );
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME), &p[..20]).unwrap_err(),
        E::Authentication
    );
    rx.receive(2, ENDPOINT, Some(TIME), &p).unwrap();
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME + 1), &bytes(UNSUPPORTED))
            .unwrap_err(),
        E::InnerService
    );
    rx.receive(2, ENDPOINT, Some(TIME + 1), &bytes(NEXT))
        .unwrap();
}

#[test]
fn trusted_time_boundaries_restart_and_key_restoration() {
    let p = bytes(IND);
    let mut rx = receiver(TIME - 2, 0);
    assert_eq!(rx.receive(2, ENDPOINT, None, &p).unwrap_err(), E::Clock);
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(1 << 48), &p).unwrap_err(),
        E::Clock
    );
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME - 1), &p).unwrap_err(),
        E::Freshness
    );
    rx.receive(2, ENDPOINT, Some(TIME + 100), &p).unwrap();
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME + 99), &p).unwrap_err(),
        E::Clock
    );
    assert_eq!(
        rx.receive(2, ENDPOINT, Some(TIME + 101), &p).unwrap_err(),
        E::Freshness
    );
    let mut restarted = receiver(TIME + 101, 0);
    assert_eq!(
        restarted
            .receive(2, ENDPOINT, Some(TIME + 101), &p)
            .unwrap_err(),
        E::Freshness
    );

    let mut a = receiver(TIME - 1, 0);
    a.receive(2, ENDPOINT, Some(TIME), &p).unwrap();
    let mut b = receiver(TIME, 16);
    assert_eq!(
        b.receive(2, ENDPOINT, Some(TIME + 1), &bytes(NEXT))
            .unwrap_err(),
        E::Authentication
    );
    b.receive(2, ENDPOINT, Some(TIME + 2), &bytes(ROTATED))
        .unwrap();
    let mut restored = receiver(TIME + 2, 0);
    assert_eq!(
        restored
            .receive(2, ENDPOINT, Some(TIME + 2), &p)
            .unwrap_err(),
        E::Freshness
    );
    restored
        .receive(2, ENDPOINT, Some(TIME + 3), &bytes(RESTORED))
        .unwrap();
}

#[test]
fn requires_concrete_membership_and_supported_clock_configuration() {
    for (interface, group, latency, time, error) in [
        (0, GROUP, 100, 0, E::Configuration),
        (1, Ipv4Addr::LOCALHOST, 100, 0, E::Configuration),
        (1, GROUP, 0, 0, E::Configuration),
        (1, GROUP, 8001, 0, E::Configuration),
        (1, GROUP, 1, 1 << 48, E::Clock),
    ] {
        assert_eq!(
            SecureRoutingReceiver::new(interface, group, SERIAL, [0; 16], latency, time).err(),
            Some(error)
        );
    }
}

#[test]
fn sender_requires_authenticated_busy_from_its_backbone() {
    use knx::{routing::SendOutcome, secure_routing::SecureRoutingSender, telegram::GroupService};
    use std::net::UdpSocket;
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let mut sender = SecureRoutingSender::new(
        socket,
        2,
        GROUP,
        0x7102,
        [1; 6],
        std::array::from_fn(|i| i as u8),
        TIME - 2,
    )
    .unwrap();
    let p = bytes(BUSY);
    let mut peer = receiver(TIME - 2, 0);
    let mut wrong_key = receiver(TIME - 2, 16);
    let before = sender.ready_at();
    for (index, group) in [(1, GROUP), (2, Ipv4Addr::new(239, 2, 3, 4))] {
        let mut other = SecureRoutingReceiver::new(
            index,
            group,
            SERIAL,
            std::array::from_fn(|i| i as u8),
            100,
            TIME - 2,
        )
        .unwrap();
        assert_eq!(
            sender
                .receive(
                    &mut other,
                    index,
                    SocketAddrV4::new(group, 3671),
                    Some(TIME),
                    &p
                )
                .unwrap_err(),
            E::Binding
        );
        other
            .receive(index, SocketAddrV4::new(group, 3671), Some(TIME), &p)
            .unwrap();
    }
    assert_eq!(
        sender
            .receive(&mut wrong_key, 2, ENDPOINT, Some(TIME), &p)
            .unwrap_err(),
        E::Binding
    );
    let mut altered = p.clone();
    *altered.last_mut().unwrap() ^= 1;
    assert_eq!(
        sender
            .receive(&mut peer, 2, ENDPOINT, Some(TIME), &altered)
            .unwrap_err(),
        E::Authentication
    );
    assert_eq!(sender.ready_at(), before);
    sender
        .receive(&mut peer, 2, ENDPOINT, Some(TIME), &p)
        .unwrap();
    let ready = sender.ready_at();
    assert!(ready > before);
    assert_eq!(
        sender
            .receive(&mut peer, 2, ENDPOINT, Some(TIME), &p)
            .unwrap_err(),
        E::Replay
    );
    assert_eq!(sender.ready_at(), ready);
    assert_eq!(
        sender
            .send(Some(TIME + 1), "3/4/5".parse().unwrap(), GroupService::Read)
            .unwrap(),
        SendOutcome::RetryAt(ready)
    );
}
