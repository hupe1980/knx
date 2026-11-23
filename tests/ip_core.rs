use knx::ip_core::{
    DiscoveryEnd, DiscoveryRequest, Gateway, IpError, IpFrame, decode_endpoint, describe, discover,
};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

// XKNX search_response_test.py, test_search_response, Gira router capture.
fn search_vector() -> Vec<u8> {
    "06 10 02 02 00 50 08 01 C0 A8 2A 0A 0E 57 36 01
     02 00 11 00 00 00 11 22 33 44 55 66 E0 00 17 0C
     01 02 03 04 05 06 47 69 72 61 20 4B 4E 58 2F 49
     50 2D 52 6F 75 74 65 72 00 00 00 00 00 00 00 00
     00 00 00 00 0C 02 02 01 03 02 04 01 05 01 07 01"
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}

#[test]
fn router_fixture_and_unknown_blocks() {
    let mut data = search_vector();
    let gateway = Gateway::search_response(&data).unwrap();
    assert_eq!(gateway.name, "Gira KNX/IP-Router");
    assert_eq!(
        gateway.control_endpoint,
        "192.168.42.10:3671".parse().unwrap()
    );
    assert_eq!(gateway.serial, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    assert_eq!(gateway.individual_address, 0x1100);
    assert_eq!(
        gateway.advertised_version(knx::ip_core::ServiceFamily::DeviceManagement),
        Some(2)
    );
    data.extend_from_slice(&[4, 0xe2, 8, 1]);
    data[5] = 84;
    assert_eq!(Gateway::search_response(&data).unwrap(), gateway);
    data[78] = 0xac;
    assert_eq!(
        Gateway::search_response(&data).unwrap().service_versions[&0xac],
        1
    );
}
#[test]
fn malformed_records_are_rejected() {
    let data = search_vector();
    for n in 0..data.len() {
        assert!(Gateway::search_response(&data[..n]).is_err());
    }
    for (i, val) in [
        (0, 7),
        (1, 0x11),
        (6, 7),
        (7, 2),
        (14, 0),
        (14, 55),
        (15, 2),
        (68, 11),
        (69, 1),
        (72, 2),
        (73, 0),
    ] {
        let mut frame = data.clone();
        frame[i] = val;
        assert!(Gateway::search_response(&frame).is_err());
    }
    let peer = "127.0.0.1:12345".parse().unwrap();
    assert_eq!(
        DiscoveryRequest::Search.encode(peer).unwrap(),
        [6, 16, 2, 1, 0, 14, 8, 1, 127, 0, 0, 1, 48, 57]
    );
    assert_eq!(
        decode_endpoint(&[8, 2, 127, 0, 0, 1, 0, 1]),
        Err(IpError::Endpoint)
    );
    assert_eq!(IpFrame::parse(&[6, 16, 2, 1, 0, 7]), Err(IpError::Length));
}
#[test]
fn description_uses_only_local_responder_and_accepts_after_partial_frame() {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let SocketAddr::V4(peer) = socket.local_addr().unwrap() else {
        panic!()
    };
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut req = [0; 14];
            let (_, from) = socket.recv_from(&mut req).unwrap();
            assert_eq!(&req[..6], &[6, 16, 2, 3, 0, 14]);
            assert_eq!(SocketAddr::V4(decode_endpoint(&req[6..]).unwrap()), from);
            let mut reply = search_vector();
            reply.drain(6..14);
            reply[3] = 4;
            reply[5] = 72;
            socket.send_to(&reply[..12], from).unwrap();
            socket.send_to(&reply, from).unwrap();
        });
        let device = describe(
            Ipv4Addr::LOCALHOST,
            peer,
            Instant::now() + Duration::from_secs(2),
            || false,
        )
        .unwrap();
        assert_eq!(device.control_endpoint, peer);
        assert_eq!(device.name, "Gira KNX/IP-Router");
    })
}
#[test]
fn cancelled_and_expired_queries_send_nothing() {
    let bind = || UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    assert_eq!(
        discover(bind(), Instant::now(), || true).unwrap().end,
        DiscoveryEnd::Cancelled
    );
    assert_eq!(
        discover(bind(), Instant::now(), || false).unwrap().end,
        DiscoveryEnd::Deadline
    );
    let peer = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671);
    assert_eq!(
        describe(Ipv4Addr::LOCALHOST, peer, Instant::now(), || false)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::TimedOut
    );
    assert_eq!(
        describe(Ipv4Addr::LOCALHOST, peer, Instant::now(), || true)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Interrupted
    );
    assert!(
        DiscoveryRequest::Description
            .encode(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 10))
            .is_err()
    );
}

#[test]
fn pending_description_polls_cancellation_while_peer_is_silent() {
    let silent = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let SocketAddr::V4(peer) = silent.local_addr().unwrap() else {
        panic!()
    };
    let mut polls = 0;
    let err = describe(
        Ipv4Addr::LOCALHOST,
        peer,
        Instant::now() + Duration::from_secs(2),
        || {
            polls += 1;
            polls == 2
        },
    )
    .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Interrupted);
}

#[test]
fn core_v2_request_and_selection_use_the_same_advertised_fields() {
    use knx::ip_core::{ExtendedSearch, ServiceFamily};
    let local = "127.0.0.1:3671".parse().unwrap();
    let criteria = ExtendedSearch {
        programming_mode: true,
        mac: Some([1, 2, 3, 4, 5, 6]),
        service: Some((ServiceFamily::Routing, 1.try_into().unwrap())),
        manufacturer_data: true,
    };
    assert_eq!(
        criteria.encode(local).unwrap(),
        [
            6, 16, 2, 11, 0, 34, 8, 1, 127, 0, 0, 1, 14, 87, 2, 129, 8, 130, 1, 2, 3, 4, 5, 6, 4,
            131, 5, 1, 6, 132, 1, 2, 254, 0,
        ]
    );
    let mut data = search_vector();
    assert_eq!(criteria.response(&data), Err(IpError::Service));
    data[3] = 12;
    assert_eq!(criteria.response(&data), Err(IpError::Selection));
    data[17] = 1;
    let gateway = criteria.response(&data).unwrap();
    assert_eq!(gateway.mac, [1, 2, 3, 4, 5, 6]);
    assert!(gateway.manufacturer_data.is_none());
    for invalid in [
        ExtendedSearch {
            mac: Some([0; 6]),
            ..criteria
        },
        ExtendedSearch {
            service: Some((ServiceFamily::Routing, 2.try_into().unwrap())),
            ..criteria
        },
    ] {
        assert_eq!(invalid.response(&data), Err(IpError::Selection));
    }
    data.extend([6, 254, 0x12, 0x34, 7, 9]);
    data[5] = 86;
    let vendor = criteria.response(&data).unwrap().manufacturer_data.unwrap();
    assert_eq!((vendor.manufacturer, vendor.data), (0x1234, vec![7, 9]));
    data.extend([4, 254, 0, 1]);
    data[5] = 90;
    assert_eq!(criteria.response(&data), Err(IpError::Description));
    data.truncate(83);
    data[5] = 83;
    data[80] = 3;
    assert_eq!(criteria.response(&data), Err(IpError::Description));
    assert_eq!(Gateway::search_response(&data), Err(IpError::Service));
}

#[test]
fn weinzierl_baos_advertisement_preserves_required_security_version() {
    use knx::ip_core::{BaosAdvertisement, ManufacturerData};
    let decode = |manufacturer, records: &[u8]| {
        ManufacturerData {
            manufacturer,
            data: records.to_vec(),
        }
        .baos()
    };
    // Weinzierl BAOS binary protocol section 6.4, figure 21.
    let fields = [1, 4, 0xf0, 0x22, 2, 4, 0xf0, 1];
    assert_eq!(
        decode(0xc5, &fields),
        Ok(Some(BaosAdvertisement {
            protocol_version: 0x22,
            required_security_version: Some(1)
        }))
    );
    assert_eq!(
        decode(0xc5, &[1, 4, 0xf0, 0x20]),
        Ok(Some(BaosAdvertisement {
            protocol_version: 0x20,
            required_security_version: None
        }))
    );
    assert_eq!(decode(0, &fields), Ok(None));
    for malformed in [
        &fields[..3],
        &fields[4..],
        &[1, 4, 0xf0, 0][..],
        &[1, 4, 0xf0, 0x22, 1, 4, 0xf0, 0x20],
        &[3, 4, 0xf0, 1],
    ] {
        assert_eq!(decode(0xc5, malformed), Err(IpError::Description));
    }
}
