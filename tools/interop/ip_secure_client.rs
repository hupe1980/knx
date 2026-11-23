use knx::{
    ip_secure::Received,
    keyring::Keyring,
    telegram::{GroupService, GroupTelegram, LinkPrimitive},
};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0; 6];
    stream.read_exact(&mut header).unwrap();
    let len = u16::from_be_bytes([header[4], header[5]]) as usize;
    assert!((6..=4118).contains(&len));
    let mut packet = header.to_vec();
    packet.resize(len, 0);
    stream.read_exact(&mut packet[6..]).unwrap();
    packet
}

fn main() {
    let port: u16 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    // Original ETS 5.7.2 test export from the pinned Calimero core checkout.
    let exported = std::fs::read_to_string(std::env::args().nth(2).unwrap()).unwrap();
    let ets = Keyring::open(&exported, "pwd").unwrap();
    ets.group_receiver(0x1101, 0x110c, "1/1/1".parse().unwrap())
        .unwrap();
    // The virtual gateway uses matching independently derived synthetic credentials.
    let keys = Keyring::open(include_str!("loopback.knxkeys"), "example").unwrap();
    let start = keys.handshake(0x7100, 0x7101, [0, 0, 0, 0, 1, 1]).unwrap();
    stream.write_all(start.request()).unwrap();
    let (pending, auth) = start.authenticate(&read_frame(&mut stream)).unwrap();
    stream.write_all(&auth).unwrap();
    let mut session = pending.finish(&read_frame(&mut stream)).unwrap();
    let connect = [
        6, 16, 2, 5, 0, 26, 8, 2, 0, 0, 0, 0, 0, 0, 8, 2, 0, 0, 0, 0, 0, 0, 4, 4, 2, 0,
    ];
    stream.write_all(&session.send(&connect).unwrap()).unwrap();
    let Received::Frame(response) = session.receive(&read_frame(&mut stream)).unwrap() else {
        panic!("expected secure CONNECT_RESPONSE")
    };
    let data = response.as_bytes();
    assert_eq!(&data[..6], &[6, 16, 2, 6, 0, 20]);
    assert_eq!(data[7], 0);
    assert_eq!(&data[16..], &[4, 4, 0x71, 1]);
    let channel = data[6];
    let time_group = "3/4/5".parse().unwrap();
    let read = GroupTelegram::request(0x7101, time_group, GroupService::Read);
    let mut request = [0u8; 21];
    request[..10].copy_from_slice(&[6, 16, 4, 0x20, 0, 21, 4, channel, 0, 0]);
    assert_eq!(read.encode(&mut request[10..]).unwrap(), 11);
    let mut confirmed = false;
    let mut observed = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(12);
    while std::time::Instant::now() < deadline {
        let Received::Frame(frame) = session.receive(&read_frame(&mut stream)).unwrap() else {
            continue;
        };
        let body = frame.as_bytes();
        assert_eq!(&body[2..4], &[4, 0x20]);
        assert_eq!(&body[6..8], &[4, channel]);
        let telegram = GroupTelegram::parse(&body[10..]).unwrap();
        if telegram.destination == time_group {
            confirmed |= matches!(
                telegram.primitive,
                LinkPrimitive::Confirmation { successful: true }
            ) && telegram.source == 0x7101
                && telegram.service == GroupService::Read;
            if telegram.source == 0x7100
                && let Some(value) = telegram.indicated_value()
            {
                if !observed && knx::text_time::TimeOfDay::decode(value.as_bytes()).is_ok() {
                    // The first live clock update proves Calimero has attached the
                    // tunnel to its virtual subnet before the client issues the read.
                    observed = true;
                    stream.write_all(&session.send(&request).unwrap()).unwrap();
                }
            }
        }
        if confirmed && observed {
            break;
        }
    }
    assert!(
        confirmed && observed,
        "protected tunnel clock observed={observed}, group read confirmed={confirmed}"
    );
    stream.write_all(&session.keepalive().unwrap()).unwrap();
    let disconnect = [6, 16, 2, 9, 0, 16, channel, 0, 8, 2, 0, 0, 0, 0, 0, 0];
    stream
        .write_all(&session.send(&disconnect).unwrap())
        .unwrap();
    let Received::Frame(response) = session.receive(&read_frame(&mut stream)).unwrap() else {
        panic!("expected secure DISCONNECT_RESPONSE")
    };
    assert_eq!(response.as_bytes(), &[6, 16, 2, 10, 0, 8, channel, 0]);
    stream.write_all(&session.close().unwrap()).unwrap();
    println!(
        "Calimero authenticated, confirmed the Rust group read and delivered DPT 10.001 feedback over the protected TCP tunnel."
    );
}
