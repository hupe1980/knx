//! Loopback-only CoAP adapter for the KNX Point API interoperability check.
use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType};
use knx::iot::{
    PointAccess, PointError, PointFormat, PointReadError, PointReply, PointTransport, SwitchGet,
    SwitchPoint,
};
use std::{
    io::{self, Read},
    net::{Ipv6Addr, SocketAddr, SocketAddrV6, UdpSocket},
    time::Duration,
};

// A fresh random test key is used for exactly one process. Production storage
// must persist the reservation before using a sequence under a long-lived key.
#[derive(Default)]
struct Sequence(Option<oscore::SenderSequenceState>);

impl oscore::SenderStateStore for Sequence {
    type Error = core::convert::Infallible;
    fn load(
        &mut self,
        _: &oscore::ContextId,
    ) -> Result<Option<oscore::SenderSequenceState>, Self::Error> {
        Ok(self.0)
    }
    fn compare_exchange(
        &mut self,
        _: &oscore::ContextId,
        expected: Option<oscore::SenderSequenceState>,
        next: oscore::SenderSequenceState,
    ) -> Result<bool, Self::Error> {
        if self.0 != expected {
            return Ok(false);
        }
        self.0 = Some(next);
        Ok(true)
    }
}

struct Peer {
    socket: UdpSocket,
    packet: Packet,
    message_id: u16,
    security: Option<(oscore::Context, Sequence)>,
}

impl PointTransport for Peer {
    type Error = io::Error;

    fn get(&mut self, req: &SwitchGet<'_>) -> io::Result<PointReply<'_>> {
        let access = if self.security.is_some() {
            PointAccess::Oscore(1)
        } else {
            PointAccess::Plain
        };
        if !req.peer().ip().is_loopback() || req.access() != access {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        self.message_id += 1;
        let mut packet = Packet::new();
        packet.header.set_type(MessageType::Confirmable);
        packet.header.code = MessageClass::Request(RequestType::Get);
        packet.header.message_id = self.message_id;

        for segment in req.uri_path() {
            packet.add_option(CoapOption::UriPath, segment.as_bytes().to_vec());
        }
        packet.add_option(CoapOption::Accept, vec![req.accept().number() as u8]);
        let piv = if let Some((context, sequence)) = &mut self.security {
            let inner = packet.to_bytes().map_err(io::Error::other)?;
            let (ciphertext, option) = context
                .reserve_sender(sequence)
                .map_err(|_| io::ErrorKind::InvalidData)?
                .protect_request(1, &inner[4..], &[])
                .map_err(io::Error::other)?;
            let id = oscore::request_identifiers(&option).map_err(io::Error::other)?;
            packet = Packet::new();
            packet.header.set_type(MessageType::Confirmable);
            packet.header.code = MessageClass::Request(RequestType::Post);
            packet.header.message_id = self.message_id;
            packet.add_option(CoapOption::Oscore, option.to_vec());
            packet.payload = ciphertext.to_vec();
            Some(id)
        } else {
            None
        };
        packet.set_token(req.token().to_vec());
        self.socket
            .send_to(&packet.to_bytes().map_err(io::Error::other)?, req.peer())?;
        let mut bytes = [0; 512];
        let (len, source) = self.socket.recv_from(&mut bytes)?;
        let SocketAddr::V6(peer) = source else {
            return Err(io::ErrorKind::InvalidData.into());
        };
        self.packet = Packet::from_bytes(&bytes[..len]).map_err(io::Error::other)?;
        // This fixture expects piggybacked responses from the local reference server.
        if self.packet.header.get_type() != MessageType::Acknowledgement
            || self.packet.header.message_id != self.message_id
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if peer != req.peer() || self.packet.get_token() != req.token() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if let (Some((context, _)), Some(id)) = (&mut self.security, piv) {
            let option = self
                .packet
                .get_option(CoapOption::Oscore)
                .filter(|v| v.len() == 1)
                .and_then(|v| v.front())
                .ok_or_else(|| {
                    if u8::from(self.packet.header.code) == 129
                        && self.packet.get_option(CoapOption::Oscore).is_none()
                    {
                        io::ErrorKind::PermissionDenied
                    } else {
                        io::ErrorKind::InvalidData
                    }
                })?;
            let mut damaged = self.packet.payload.clone();
            *damaged.last_mut().ok_or(io::ErrorKind::InvalidData)? ^= 1;
            assert!(
                context
                    .unprotect_response(option, &damaged, id.piv())
                    .is_err()
            );
            let (code, options, body) = context
                .unprotect_response(option, &self.packet.payload, id.piv())
                .map_err(io::Error::other)?;
            assert!(
                context
                    .unprotect_response(option, &self.packet.payload, id.piv())
                    .is_err()
            );
            let token = self.packet.get_token().to_owned();
            let mut decoded = vec![0x40, code, 0, 0];
            decoded.extend_from_slice(&options);
            self.packet = Packet::from_bytes(&decoded).map_err(io::Error::other)?;
            self.packet.payload = body.to_vec();
            self.packet.set_token(token);
        }
        let content_format = match self.packet.get_option(CoapOption::ContentFormat) {
            None => None,
            Some(options) if options.len() == 1 => match options.front().unwrap().as_slice() {
                [] => Some(0),
                [a] => Some(u16::from(*a)),
                [a, b] => Some(u16::from_be_bytes([*a, *b])),
                _ => return Err(io::ErrorKind::InvalidData.into()),
            },
            _ => return Err(io::ErrorKind::InvalidData.into()),
        };
        Ok(PointReply {
            peer,
            token: self.packet.get_token(),
            access,
            code: self.packet.header.code.into(),
            content_format,
            body: &self.packet.payload,
        })
    }
}

// aiocoap 0.4.17 capture: GET /p/secure/0/ioo, Accept 60, CBOR true.
// Public known-answer material is used only offline for this exact exchange.
fn verify_wire_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let plain_request = b"\x43\x01\x00\x01\x6b\x6e\x78\xb1\x70\x06\x73\x65\x63\x75\x72\x65\x01\x30\x03\x69\x6f\x6f\x61\x3c";
    let plain_response = b"\x63\x45\x00\x01\x6b\x6e\x78\xc1\x3c\xff\xa1\x01\xf5";
    let request = Packet::from_bytes(b"\x43\x02\x00\x01\x6b\x6e\x78\x93\x09\x00\x01\xff\xbd\x69\x4d\xf8\xf4\x0f\xe2\xc3\xe0\x22\x71\xfb\x92\x37\x4c\x5f\x80\xaf\x2d\x57\x41\x09\xd6\x97\xde\x29")?;
    let response = Packet::from_bytes(b"\x63\x44\x00\x01\x6b\x6e\x78\x90\xff\x67\xb6\x5e\xf1\x3e\x02\xa9\xfa\xc2\xac\x91\xb4\xba\xde\xbd")?;
    let mut seq = Sequence::default();
    let key = core::array::from_fn::<_, 16, _>(|i| i as u8);
    let mut context = oscore::Context::new_fresh(&key, None, None, &[1], &[])?
        .register_fresh(&mut seq)
        .map_err(|_| "fixture sequence registration failed")?;
    let (body, option) = context
        .reserve_sender(&mut seq)
        .map_err(|_| "fixture sequence")?
        .protect_request(1, &plain_request[7..], &[])?;
    assert_eq!(
        request
            .get_option(CoapOption::Oscore)
            .unwrap()
            .front()
            .unwrap()
            .as_slice(),
        &option[..]
    );
    assert_eq!(request.payload, &body[..]);
    let id = oscore::request_identifiers(&option)?;
    let (code, options, body) = context.unprotect_response(
        response
            .get_option(CoapOption::Oscore)
            .unwrap()
            .front()
            .unwrap()
            .as_slice(),
        &response.payload,
        id.piv(),
    )?;
    assert_eq!(code, plain_response[1]);
    assert_eq!(&options[..], &plain_response[7..9]);
    assert_eq!(&body[..], &plain_response[10..]);
    assert!(knx::iot::decode_switch_response(code, 60, &body)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(port) = std::env::args().nth(1) else {
        verify_wire_fixture()?;
        for fmt in [PointFormat::Json, PointFormat::Cbor] {
            for value in [false, true] {
                for byte in fmt.switch_body(value) {
                    print!("{byte:02x}");
                }
                println!();
            }
        }
        return Ok(());
    };
    let peer = SocketAddrV6::new(Ipv6Addr::LOCALHOST, port.parse()?, 0, 0);
    let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mode = std::env::args().nth(2).ok_or("missing fixture mode")?;
    let protected = matches!(mode.as_str(), "oscore" | "unknown" | "guest");
    if !protected && mode != "plain" {
        return Err("unknown fixture mode".into());
    }
    let security = if protected {
        let mut key = [0; 16];
        io::stdin().read_exact(&mut key)?;
        let mut sequence = Sequence::default();
        let sender = match mode.as_str() {
            "unknown" => 2,
            "guest" => 3,
            _ => 1,
        };
        let context = oscore::Context::new_fresh(&key, None, None, &[sender], &[])?
            .register_fresh(&mut sequence)
            .map_err(|_| "fresh context registration failed")?;
        Some((context, sequence))
    } else {
        None
    };
    let access = if protected {
        PointAccess::Oscore(1)
    } else {
        PointAccess::Plain
    };
    let mut host = Peer {
        socket,
        packet: Packet::new(),
        message_id: 0,
        security,
    };
    let path = if protected { "secure" } else { "lsab" };
    let segments = ["p", path, "0", "ioo"];
    let point = SwitchPoint::new(peer, &segments, access)?;
    if mode == "guest" {
        assert!(matches!(
            point.get(PointFormat::Cbor, [0; 8]).read(&mut host),
            Err(PointReadError::Response(PointError::ResponseCode(131)))
        ));
        return Ok(());
    }
    if mode == "unknown" {
        assert!(matches!(
            point.get(PointFormat::Cbor, [0; 8]).read(&mut host),
            Err(PointReadError::Transport(e)) if e.kind() == io::ErrorKind::PermissionDenied
        ));
        return Ok(());
    }
    for (i, fmt) in [PointFormat::Json, PointFormat::Cbor]
        .into_iter()
        .enumerate()
    {
        for (j, expected) in [false, true].into_iter().enumerate() {
            let token = ((2 * i + j) as u64).to_be_bytes();
            assert_eq!(point.get(fmt, token).read(&mut host)?, expected);
        }
    }
    let restricted = SwitchPoint::new(peer, &["p", "private"], access)?;
    assert!(matches!(
        restricted
            .get(PointFormat::Cbor, 4u64.to_be_bytes())
            .read(&mut host),
        Err(PointReadError::Response(PointError::ResponseCode(131)))
    ));
    if !protected {
        let denied = SwitchPoint::new(peer, &["p", "secure", "0", "ioo"], access)?;
        assert!(matches!(
            denied
                .get(PointFormat::Cbor, 5u64.to_be_bytes())
                .read(&mut host),
            Err(PointReadError::Response(PointError::ResponseCode(129)))
        ));
    }
    println!("Rust Point API GET verified JSON, CBOR and denial over {access:?} on IPv6 loopback.");
    Ok(())
}
