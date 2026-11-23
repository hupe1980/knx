use knx::telegram::{GroupData, GroupService, GroupTelegram};
use knx::tunnel::{self, ControlEvent, Event, Tunnel};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

fn v4(a: SocketAddr) -> SocketAddrV4 {
    let SocketAddr::V4(a) = a else {
        panic!("expected IPv4")
    };
    a
}
fn main() {
    let port = std::env::args().nth(1).unwrap().parse().unwrap();
    let gateway = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let local = v4(socket.local_addr().unwrap());
    let mut buf = [0; 10 + GroupTelegram::MAX_ENCODED_LEN];
    socket
        .send_to(&tunnel::connect_request(local, local).unwrap(), gateway)
        .unwrap();
    let (size, peer) = socket.recv_from(&mut buf).unwrap();
    let mut t = Tunnel::accept(gateway, v4(peer), &buf[..size]).unwrap();
    let read = GroupTelegram::request(0, "1/2/1".parse().unwrap(), GroupService::Read);
    let data = t.data_endpoint();
    socket.send_to(t.request(&read).unwrap(), data).unwrap();
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    assert_eq!(t.receive(v4(p), &buf[..n]), Ok(Event::Acknowledged));
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    let Event::Incoming {
        ack,
        telegram: Some(ind),
    } = t.receive(v4(p), &buf[..n]).unwrap()
    else {
        panic!("expected group indication")
    };
    assert_eq!(ind.indicated_value().unwrap().as_bytes(), [1]);
    socket.send_to(&ack, data).unwrap();
    let values: Vec<u8> = (0..253).collect();
    let write = GroupTelegram::request(
        t.assigned_address(),
        read.destination,
        GroupService::Write(GroupData::Bytes(&values)),
    );
    socket.send_to(t.request(&write).unwrap(), data).unwrap();
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    assert_eq!(t.receive(v4(p), &buf[..n]), Ok(Event::Acknowledged));
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    let Event::Incoming {
        ack,
        telegram: Some(ind),
    } = t.receive(v4(p), &buf[..n]).unwrap()
    else {
        panic!("expected extended group indication")
    };
    assert_eq!(ind.indicated_value().unwrap().as_bytes(), values);
    socket.send_to(&ack, data).unwrap();
    socket
        .send_to(&t.heartbeat(local).unwrap(), gateway)
        .unwrap();
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    assert_eq!(
        t.heartbeat_response(v4(p), &buf[..n]),
        Ok(ControlEvent::Healthy)
    );
    socket
        .send_to(&t.disconnect(local).unwrap(), gateway)
        .unwrap();
    let (n, p) = socket.recv_from(&mut buf).unwrap();
    t.disconnect_response(v4(p), &buf[..n]).unwrap();
}
