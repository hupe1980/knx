//! Interoperate with the virtual Calimero server from the discovery quickstart.
use knx::{
    ip_core::{ExtendedSearch, ServiceFamily, describe, discover},
    routing::{RoutingEvent, RoutingMembership, RoutingReceiver},
};
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .ok_or("expected peer port or secure routing interface index")?;
    if first == "--group-timer" {
        return group_timer(
            args.next().ok_or("expected interface index")?.parse()?,
            args.next()
                .ok_or("expected secure multicast group")?
                .parse()?,
        );
    }
    if first == "--secure-routing" {
        return secure_routing(
            args.next().ok_or("expected interface index")?.parse()?,
            args.next()
                .ok_or("expected secure multicast group")?
                .parse()?,
        );
    }
    let group: Ipv4Addr = args
        .next()
        .ok_or("expected plain multicast group")?
        .parse()?;
    let local = Ipv4Addr::LOCALHOST;
    let search_socket = || -> std::io::Result<UdpSocket> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.bind(&SocketAddrV4::new(local, 0).into())?;
        socket.set_multicast_if_v4(&local)?;
        Ok(socket.into())
    };
    let deadline = || Instant::now() + Duration::from_secs(2);
    let report = discover(search_socket()?, deadline(), || false)?;
    let peer = SocketAddrV4::new(local, first.parse()?);
    let gateway = report
        .gateways
        .iter()
        .find(|g| g.control_endpoint == peer)
        .ok_or("loopback gateway missing")?;
    let details = describe(local, peer, deadline(), || false)?;
    assert_eq!(gateway.name, "Calimero loopback");
    assert_eq!(gateway.individual_address, 0x7100);
    assert_eq!(gateway.control_endpoint, details.control_endpoint);
    assert_eq!(gateway.individual_address, details.individual_address);
    assert_eq!(gateway.serial, details.serial);
    assert_eq!(gateway.name, details.name);
    assert_eq!(gateway.service_versions, details.service_versions);
    assert!(gateway.advertised_version(ServiceFamily::Routing).is_some());
    let criteria = ExtendedSearch {
        service: Some((ServiceFamily::Core, 2.try_into()?)),
        manufacturer_data: true,
        ..Default::default()
    };
    let extended = criteria.discover(search_socket()?, deadline(), || false)?;
    let v2 = extended
        .gateways
        .iter()
        .find(|g| g.control_endpoint == peer && g.serial == details.serial)
        .ok_or("missing Core v2 peer")?;
    let baos = v2
        .manufacturer_data
        .as_ref()
        .ok_or("missing requested manufacturer DIB")?
        .baos()?
        .ok_or("missing ObjectServer advertisement")?;
    assert_eq!(baos.protocol_version, 0x20);
    println!("Legacy search, extended Core v2 search and description matched at {peer}.");

    let listen = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    listen.set_reuse_address(true)?;
    #[cfg(unix)]
    listen.set_reuse_port(true)?;
    listen.bind(&SocketAddrV4::new(group, 3671).into())?;
    let listen: UdpSocket = listen.into();
    listen.set_read_timeout(Some(Duration::from_secs(6)))?;
    let _membership = RoutingMembership::join(&listen, group, local)?;
    let receiver = RoutingReceiver::new(local, None)?;
    let mut buf = [0; 512];
    let (n, sender) = listen.recv_from(&mut buf)?;
    let SocketAddr::V4(sender) = sender else {
        return Err("expected IPv4".into());
    };
    assert_eq!(*sender.ip(), local);
    let Some(RoutingEvent::Indication(indication)) = receiver.receive(local, sender, &buf[..n])?
    else {
        return Err("expected routed group indication".into());
    };
    assert_eq!(indication.destination.to_string(), "3/4/5");
    assert_eq!(
        indication
            .indicated_value()
            .ok_or("missing time")?
            .as_bytes()
            .len(),
        3
    );
    println!("Received the virtual gateway's three-byte time at 3/4/5.");
    drop(_membership);
    drop(listen);
    let outbound = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    outbound.bind(&SocketAddrV4::new(local, 0).into())?;
    outbound.set_multicast_if_v4(&local)?;
    let mut sender = knx::routing::RoutingSender::new(outbound.into(), group, 0x7102)?;
    assert_eq!(
        sender.send("3/4/5".parse()?, knx::telegram::GroupService::Read)?,
        knx::routing::SendOutcome::Sent,
    );
    Ok(())
}

fn secure_routing(index: u32, group: Ipv4Addr) -> Result<(), Box<dyn std::error::Error>> {
    use knx::routing::SendOutcome;
    use knx::secure_routing::{SecureRoutingReceiver, SecureRoutingSender};
    use knx::telegram::GroupService;

    let local = Ipv4Addr::LOCALHOST;
    let key = std::array::from_fn(|i| i as u8);
    let listen = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    listen.set_reuse_address(true)?;
    #[cfg(unix)]
    listen.set_reuse_port(true)?;
    listen.bind(&SocketAddrV4::new(group, 3671).into())?;
    let listen: UdpSocket = listen.into();
    listen.set_read_timeout(Some(Duration::from_secs(2)))?;
    let _membership = RoutingMembership::join(&listen, group, local)?;
    let outbound = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    outbound.bind(&SocketAddrV4::new(local, 0).into())?;
    outbound.set_multicast_if_v4(&local)?;
    let own_endpoint = outbound
        .local_addr()?
        .as_socket()
        .ok_or("missing IPv4 endpoint")?;
    let mut sender = SecureRoutingSender::new(
        outbound.into(),
        index,
        group,
        0x7102,
        [1, 2, 3, 4, 5, 6],
        key,
        1000,
    )?;
    let mut peer = SecureRoutingReceiver::new(index, group, [0, 0, 0, 0, 0, 7], key, 100, 1000)?;
    let destination = "3/4/5".parse()?;
    assert_eq!(
        sender.send(Some(1001), destination, GroupService::Read)?,
        SendOutcome::Sent
    );
    let mut buf = [0; 512];
    for _ in 0..2 {
        let (n, source) = listen.recv_from(&mut buf)?;
        if source == own_endpoint {
            continue;
        }
        if !source.ip().is_loopback() {
            return Err("unexpected peer address".into());
        }
        let verified = sender.receive(
            &mut peer,
            index,
            SocketAddrV4::new(group, listen.local_addr()?.port()),
            Some(1002),
            &buf[..n],
        )?;
        assert!(
            matches!(verified.event(), RoutingEvent::Busy {wait,..} if wait == Duration::from_millis(100))
        );
        assert!(matches!(
            sender.send(Some(1003), destination, GroupService::Read)?,
            SendOutcome::RetryAt(_)
        ));
        return Ok(());
    }
    Err("missing authenticated Calimero busy report".into())
}

fn group_timer(index: u32, group: Ipv4Addr) -> Result<(), Box<dyn std::error::Error>> {
    use knx::secure_routing::{
        GroupClock, SecureRoutingSender, TimerConfig, TimerOrigin, TimerStart, TimerStore,
    };
    use knx::{routing::SendOutcome, telegram::GroupService};
    struct NewFixtureKey;
    impl TimerStore for NewFixtureKey {
        fn origin(&mut self) -> std::io::Result<TimerOrigin> {
            Ok(TimerOrigin::NewKey)
        }
        fn reserve_through(&mut self, _: u64) -> std::io::Result<()> {
            Ok(())
        }
    }
    let local = Ipv4Addr::LOCALHOST;
    let key = std::array::from_fn(|i| i as u8);
    let listen = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    listen.set_reuse_address(true)?;
    #[cfg(unix)]
    listen.set_reuse_port(true)?;
    let endpoint = SocketAddrV4::new(group, 3671);
    listen.bind(&endpoint.into())?;
    let listen: UdpSocket = listen.into();
    listen.set_read_timeout(Some(Duration::from_secs(2)))?;
    let _membership = RoutingMembership::join(&listen, group, local)?;
    let outbound = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    outbound.bind(&SocketAddrV4::new(local, 0).into())?;
    outbound.set_multicast_if_v4(&local)?;
    let outbound: UdpSocket = outbound.into();
    let SocketAddr::V4(local_source) = outbound.local_addr()? else {
        unreachable!()
    };
    let start = Instant::now();
    let mut clock = GroupClock::new(
        TimerConfig {
            interface_index: index,
            multicast_group: group,
            local_source,
            serial: [0, 0, 0, 0, 8, 1],
            latency_ms: 100,
            sync_fraction: 26,
        },
        key,
        NewFixtureKey,
        start,
        TimerStart::Immediate,
    )?;
    assert_eq!(clock.sample(start)?, None);
    let mut challenge = [0; 8];
    clock.poll(start, |to, bytes| {
        challenge.copy_from_slice(&bytes[12..20]);
        outbound.send_to(bytes, to).map(|_| ())
    })?;
    let mut buf = [0; 512];
    for _ in 0..8 {
        let (n, source) = listen.recv_from(&mut buf)?;
        let SocketAddr::V4(source) = source else {
            continue;
        };
        if !source.ip().is_loopback() {
            return Err("unexpected timer peer source".into());
        }
        let now = Instant::now();
        clock.observe(now, index, endpoint, source, &buf[..n])?;
        if let Some(timer) = clock.sample(now)? {
            // A physical reply to the outstanding challenge must raise the new clock
            // above our local elapsed time. This excludes timeout-only acquisition.
            if source == local_source
                || n != 36
                || buf[2..4] != [9, 0x55]
                || buf[12..20] != challenge
                || timer <= now.duration_since(start).as_millis() as u64 + 100
            {
                return Err("expected Calimero's advanced group time".into());
            }
            let mut sender = SecureRoutingSender::new(
                outbound,
                index,
                group,
                0x7102,
                [1, 2, 3, 4, 5, 6],
                key,
                timer,
            )?;
            std::thread::sleep(Duration::from_millis(2));
            let now = Instant::now();
            assert_eq!(
                sender.send(clock.sample(now)?, "3/4/5".parse()?, GroupService::Read)?,
                SendOutcome::Sent
            );
            clock.wrapper_sent(now)?;
            return Ok(());
        }
    }
    Err("no authenticated Calimero timer response".into())
}
