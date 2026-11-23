use knx::routing::{RoutingError, RoutingEvent, RoutingReceiver};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

// XKNX routing_lost_message_test.py and routing_busy_test.py.
const LOST: [u8; 10] = [6, 16, 5, 0x31, 0, 10, 4, 0, 0, 5];
const BUSY: [u8; 12] = [6, 16, 5, 0x32, 0, 12, 6, 0, 0, 100, 0, 0];
const IND: [u8; 17] = [
    6, 16, 5, 0x30, 0, 17, 0x29, 0, 0xbc, 0xe0, 0xff, 0xff, 0x23, 2, 1, 0, 0x81,
];
#[test]
fn independent_vectors_expose_loss_and_busy_separately() {
    assert_eq!(
        RoutingEvent::parse(&LOST),
        Ok(RoutingEvent::Lost {
            device_state: 0,
            messages: 5
        })
    );
    assert_eq!(
        RoutingEvent::parse(&BUSY),
        Ok(RoutingEvent::Busy {
            device_state: 0,
            wait: Duration::from_millis(100),
            control: 0
        })
    );
    let RoutingEvent::Indication(telegram) = RoutingEvent::parse(&IND).unwrap() else {
        panic!()
    };
    assert_eq!(telegram.indicated_value().unwrap().as_bytes(), &[1]);
}
#[test]
fn streams_are_isolated_and_local_echo_is_explicit() {
    let a = Ipv4Addr::new(192, 0, 2, 1);
    let b = Ipv4Addr::new(192, 0, 2, 2);
    let echo = SocketAddrV4::new(a, 3671);
    let other = SocketAddrV4::new(b, 3671);
    let receive = RoutingReceiver::new(a, Some(echo)).unwrap();
    assert!(receive.receive(a, echo, &IND).unwrap().is_none());
    assert!(receive.receive(b, other, &IND).unwrap().is_none());
    for data in [&BUSY[..], &LOST, &IND] {
        assert_eq!(
            receive.receive(a, other, data).unwrap(),
            Some(RoutingEvent::parse(data).unwrap())
        );
    }
    assert!(RoutingReceiver::new(Ipv4Addr::UNSPECIFIED, None).is_err());
}
#[test]
fn unsupported_primitive_wrong_width_and_other_services_are_rejected() {
    for n in 0..BUSY.len() {
        assert!(RoutingEvent::parse(&BUSY[..n]).is_err());
    }
    let mut loss = LOST;
    loss[6] = 6;
    assert_eq!(RoutingEvent::parse(&loss), Err(RoutingError::Body));
    let mut busy = BUSY;
    busy[6] = 4;
    assert_eq!(RoutingEvent::parse(&busy), Err(RoutingError::Body));
    let mut frame = IND;
    frame[6] = 0x2e;
    assert_eq!(RoutingEvent::parse(&frame), Err(RoutingError::Body));
    frame[3] = 0x33;
    assert_eq!(
        RoutingEvent::parse(&frame),
        Err(RoutingError::UnsupportedService)
    );
}

#[test]
fn group_membership_lifetime_is_bound_to_a_local_socket() {
    use knx::routing::RoutingMembership;
    let local = Ipv4Addr::LOCALHOST;
    let socket = std::net::UdpSocket::bind((local, 0)).unwrap();
    let group = Ipv4Addr::new(239, 255, 0, 42);
    let membership = RoutingMembership::join(&socket, group, local).unwrap();
    assert!(RoutingMembership::join(&socket, group, local).is_err());
    drop(membership);
    let _membership = RoutingMembership::join(&socket, group, local).unwrap();
    assert!(RoutingMembership::join(&socket, local, local).is_err());
}

#[test]
fn sender_filters_echo_and_interface_before_applying_busy() {
    use knx::routing::RoutingSender;
    use std::{net::UdpSocket, time::Instant};

    let local = Ipv4Addr::LOCALHOST;
    let group = Ipv4Addr::new(239, 255, 42, 12);
    let socket = UdpSocket::bind((local, 0)).unwrap();
    let local_port = socket.local_addr().unwrap().port();
    let own = SocketAddrV4::new(local, local_port);
    let peer = SocketAddrV4::new(local, if local_port == 3671 { 3672 } else { 3671 });
    let mut sender = RoutingSender::new(socket, group, 0x1201).unwrap();
    assert_eq!(sender.receive(local, own, &BUSY), Ok(None));
    assert_eq!(sender.receive(Ipv4Addr::UNSPECIFIED, peer, &BUSY), Ok(None));

    for ms in [0u16, 19, 101, 0xffff] {
        let mut invalid = BUSY;
        invalid[8..10].copy_from_slice(&ms.to_be_bytes());
        assert_eq!(
            sender.receive(local, peer, &invalid),
            Err(RoutingError::Body)
        );
    }
    let mut unknown_control = BUSY;
    unknown_control[11] = 1;
    let before = Instant::now();
    assert!(
        sender
            .receive(local, peer, &unknown_control)
            .unwrap()
            .is_some()
    );
    let ready = sender.ready_at();
    assert!(ready >= before + Duration::from_millis(100));
    assert!(ready <= Instant::now() + Duration::from_millis(150));
}

#[test]
fn sender_requires_a_concrete_bound_socket_and_multicast_destination() {
    use knx::routing::RoutingSender;
    use std::net::UdpSocket;
    let local = Ipv4Addr::LOCALHOST;
    let group = Ipv4Addr::new(239, 255, 42, 12);
    assert!(RoutingSender::new(UdpSocket::bind((local, 0)).unwrap(), local, 1).is_err());
    assert!(
        RoutingSender::new(
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap(),
            group,
            1
        )
        .is_err()
    );
    let socket = UdpSocket::bind((local, 0)).unwrap();
    socket.connect((local, 1234)).unwrap();
    assert!(RoutingSender::new(socket, group, 1).is_err());
}

#[cfg(all(target_os = "linux", feature = "linux-routing"))]
#[test]
fn linux_socket_reports_actual_ingress_truncation_and_local_queue_loss() {
    use knx::routing::LinuxRoutingSocket;
    use nix::sys::socket::{setsockopt, sockopt};
    use std::{io::ErrorKind, net::UdpSocket};
    let local = Ipv4Addr::LOCALHOST;
    let socket = UdpSocket::bind((local, 0)).unwrap();
    socket.set_nonblocking(true).unwrap();
    setsockopt(&socket, sockopt::RcvBuf, &2048).unwrap();
    let target = socket.local_addr().unwrap();
    let control = socket.try_clone().unwrap();
    let mut receiver = LinuxRoutingSocket::new(socket).unwrap();
    let tx = UdpSocket::bind((local, 0)).unwrap();
    let mut storage = [0; 128];

    tx.send_to(&IND, target).unwrap();
    let datagram = receiver.receive(&mut storage).unwrap();
    assert_eq!(datagram.bytes, IND);
    assert_eq!(datagram.destination, local);
    assert_eq!(datagram.local_address, local);
    assert_eq!(datagram.source.port(), tx.local_addr().unwrap().port());
    assert!(datagram.interface_index > 0);
    tx.send_to(&IND, target).unwrap();
    assert_eq!(
        receiver.receive(&mut storage[..4]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );

    // Overflow only this dedicated loopback queue. A later packet carries its cumulative counter.
    for _ in 0..64 {
        tx.send_to(&LOST, target).unwrap();
    }
    let mut delivered = 0;
    loop {
        match receiver.receive(&mut storage) {
            Ok(_) => delivered += 1,
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => panic!("{e}"),
        }
    }
    assert!((1..64).contains(&delivered));
    tx.send_to(&IND, target).unwrap();
    let marker = receiver.receive(&mut storage).unwrap();
    assert_eq!(marker.socket_drop_total, Some(64 - delivered));
    assert!(matches!(
        RoutingEvent::parse(marker.bytes),
        Ok(RoutingEvent::Indication(_))
    ));

    setsockopt(&control, sockopt::ReceiveTimestampns, &true).unwrap();
    tx.send_to(&IND, target).unwrap();
    assert_eq!(
        receiver.receive(&mut storage).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    setsockopt(&control, sockopt::ReceiveTimestampns, &false).unwrap();
    setsockopt(&control, sockopt::Ipv4PacketInfo, &false).unwrap();
    tx.send_to(&IND, target).unwrap();
    assert_eq!(
        receiver.receive(&mut storage).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

#[cfg(all(target_os = "linux", feature = "linux-routing"))]
#[test]
fn failed_local_send_returns_the_kernel_error_without_advancing_pacing() {
    use knx::{routing::RoutingSender, telegram::GroupService};
    use nix::sys::socket::{Shutdown, shutdown};
    use std::{net::UdpSocket, os::fd::AsRawFd};

    let local = Ipv4Addr::LOCALHOST;
    let socket = UdpSocket::bind((local, 0)).unwrap();
    socket.set_multicast_ttl_v4(0).unwrap();
    let control = socket.try_clone().unwrap();
    let group = Ipv4Addr::new(239, 255, 42, 12);
    let mut sender = RoutingSender::new(socket, group, 0x7102).unwrap();
    let initial = sender.ready_at();

    // Disable transmission on this local socket to exercise the real UDP failure path.
    control.connect(control.local_addr().unwrap()).unwrap();
    shutdown(control.as_raw_fd(), Shutdown::Write).unwrap();
    let expected = control
        .send_to(&[], (group, 3671))
        .unwrap_err()
        .raw_os_error()
        .unwrap();
    let error = sender
        .send("3/4/5".parse().unwrap(), GroupService::Read)
        .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(expected));
    assert_eq!(sender.ready_at(), initial);
}
