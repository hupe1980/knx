use knx::iot::{PointError, PointFormat, decode_switch_response};

#[test]
fn versioned_switch_status_read_returns_the_commissioned_boolean() {
    // Point OAS ee2c859, readProperty without metadata, CBOR integer key 1.
    for (format, on, value) in [
        (50, false, b"false".as_slice()),
        (50, true, b"true"),
        (60, false, &[0xa1, 1, 0xf4]),
        (60, true, &[0xa1, 1, 0xf5]),
    ] {
        let selected = if format == 50 {
            PointFormat::Json
        } else {
            PointFormat::Cbor
        };
        assert_eq!(selected.number(), format);
        assert_eq!(selected.switch_body(on), value);
        assert_eq!(decode_switch_response(69, format, value), Ok(on));
    }
    assert_eq!(decode_switch_response(69, 50, b" \ntrue\r\t"), Ok(true));
    for valid in [
        &[0xbf, 1, 0xf5, 0xff][..],
        &[0xb8, 1, 0x19, 0, 1, 0xf5],
        &[0xbb, 0, 0, 0, 0, 0, 0, 0, 1, 0x18, 1, 0xf5],
    ] {
        assert_eq!(decode_switch_response(69, 60, valid), Ok(true));
    }
}

#[test]
fn rejects_other_protocols_metadata_and_unsuccessful_responses() {
    assert_eq!(
        decode_switch_response(129, 60, &[0xa1, 1, 0xf5]),
        Err(PointError::ResponseCode(129))
    );
    assert_eq!(
        decode_switch_response(69, 0, b"true"),
        Err(PointError::ContentFormat)
    );
    for invalid in [
        &[1][..],
        &[0xf5],
        &[0xbf, 1, 0xf5],
        &[0xa1, 1, 0xf5, 0],
        &[0xa2, 1, 0xf5, 1, 0xf4],
        &[0xa1, 1, 0],
        &[0xa1, 0x61, b'1', 0xf5],
        &[0xbf, 1, 0xf5, 1, 0xf4, 0xff],
        &[0xb8, 0],
        &[0xa1, 0x1b, 0],
        &[0xa1, 2, 0xf4],
    ] {
        assert_eq!(
            decode_switch_response(69, 60, invalid),
            Err(PointError::Body)
        );
    }
    for invalid in [
        br#"{"value":true}"#.as_slice(),
        b"1",
        b"true\x0b",
        b"",
        b"  ",
        b"true false",
    ] {
        assert_eq!(
            decode_switch_response(69, 50, invalid),
            Err(PointError::Body)
        );
    }
}

#[test]
fn commissioned_get_binds_peer_token_security_and_negotiated_format() {
    use knx::iot::{PointAccess, PointReply, SwitchPoint};
    use std::net::{Ipv6Addr, SocketAddrV6};
    let ip = Ipv6Addr::LOCALHOST;
    let peer = SocketAddrV6::new(ip, 5683, 0, 0);
    let access = PointAccess::Oscore(27);
    let path = ["west", "p", "room 1", "switch"];
    let point = SwitchPoint::new(peer, &path, access).unwrap();
    let token = *b"get-0011";
    let get = || point.get(PointFormat::Cbor, token);
    let good = || PointReply {
        peer,
        token: &token,
        access,
        code: 69,
        content_format: Some(60),
        body: &[0xa1, 1, 0xf5],
    };
    assert_eq!(get().uri_path(), path);
    assert_eq!(get().token(), token);
    assert_eq!(get().peer(), peer);
    assert_eq!(get().complete(good()), Ok(true));
    for wrong in [
        PointReply {
            peer: SocketAddrV6::new(ip, 5684, 0, 0),
            ..good()
        },
        PointReply {
            peer: SocketAddrV6::new(ip, 5683, 0, 1),
            ..good()
        },
        PointReply {
            peer: "[2001:db8::1]:5683".parse().unwrap(),
            ..good()
        },
        PointReply {
            token: b"x",
            ..good()
        },
        PointReply {
            access: PointAccess::Plain,
            ..good()
        },
        PointReply {
            access: PointAccess::Oscore(101),
            ..good()
        },
    ] {
        assert_eq!(get().complete(wrong), Err(PointError::Exchange));
    }
    for format in [None, Some(50), Some(0)] {
        assert_eq!(
            get().complete(PointReply {
                content_format: format,
                ..good()
            }),
            Err(PointError::ContentFormat)
        );
    }
    for code in [128, 129, 131, 132, 160] {
        assert_eq!(
            get().complete(PointReply {
                code,
                content_format: None,
                ..good()
            }),
            Err(PointError::ResponseCode(code))
        );
    }
    for addr in [
        "[::]:1",
        "[ff02::1]:5683",
        "[fe80::1]:5683",
        "[::1]:0",
        "[::ffff:127.0.0.1]:5683",
    ] {
        assert!(SwitchPoint::new(addr.parse().unwrap(), &path, access).is_err());
    }
    for segments in [
        &["p"][..],
        &["not-a-property", "status"],
        &["p", ".."],
        &["p", ""],
        &["p", "."],
    ] {
        assert!(SwitchPoint::new(peer, segments, access).is_err());
    }
    assert!(SwitchPoint::new(peer, &["p", &"x".repeat(256)], access).is_err());
}

#[test]
fn read_calls_the_selected_transport_once_and_propagates_its_error() {
    use knx::iot::{
        PointAccess, PointReadError, PointReply, PointTransport, SwitchGet, SwitchPoint,
    };
    struct Denied(usize);
    impl PointTransport for Denied {
        type Error = std::io::Error;
        fn get(&mut self, req: &SwitchGet<'_>) -> Result<PointReply<'_>, Self::Error> {
            self.0 += 1;
            assert_eq!(req.access(), PointAccess::Oscore(12));
            assert_eq!(req.accept(), PointFormat::Json);
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        }
    }
    let point = SwitchPoint::new(
        "[::1]:5683".parse().unwrap(),
        &["p", "switch"],
        PointAccess::Oscore(12),
    )
    .unwrap();
    let mut host = Denied(0);
    assert!(
        matches!(point.get(PointFormat::Json,[4;8]).read(&mut host),Err(PointReadError::Transport(e)) if e.kind()==std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(host.0, 1);
}
