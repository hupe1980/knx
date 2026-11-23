//! KNXnet/IP 1.0 headers, UDP/IPv4 discovery and gateway descriptions.
//!
//! Search uses a caller-configured IPv4 multicast socket. Select the outgoing
//! multicast interface before passing the socket and bind it to the address
//! that gateways should reply to. An advertisement is an unauthenticated claim.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

/// KNX system setup multicast endpoint for legacy UDP discovery.
pub const SEARCH_ENDPOINT: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 23, 12), 3671);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpError {
    Header,
    Length,
    Endpoint,
    Service,
    Selection,
    Description,
}

impl fmt::Display for IpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Header => "Supply a KNXnet/IP version 1.0 header.",
            Self::Length => "Supply exactly one complete KNXnet/IP datagram.",
            Self::Endpoint => "Supply a supported UDP/IPv4 endpoint.",
            Self::Selection => "The reply does not match the requested search criteria.",
            Self::Service => "Select a supported KNXnet/IP service.",
            Self::Description => "Check the device and service description blocks.",
        })
    }
}
impl std::error::Error for IpError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpFrame<'a> {
    pub service: u16,
    pub body: &'a [u8],
}

impl<'a> IpFrame<'a> {
    /// Split a complete datagram. The service parser validates the body.
    pub fn parse(datagram: &'a [u8]) -> Result<Self, IpError> {
        let [header_len, version, sh, sl, lh, ll, body @ ..] = datagram else {
            return Err(IpError::Length);
        };
        if (*header_len, *version) != (6, 0x10) {
            return Err(IpError::Header);
        }
        if usize::from(u16::from_be_bytes([*lh, *ll])) != datagram.len() {
            return Err(IpError::Length);
        }
        Ok(Self {
            service: u16::from_be_bytes([*sh, *sl]),
            body,
        })
    }
}

/// Encode an IPv4 UDP HPAI. Individual services determine whether zero endpoints are valid.
pub fn encode_endpoint(endpoint: SocketAddrV4) -> [u8; 8] {
    let [a, b, c, d] = endpoint.ip().octets();
    let [h, l] = endpoint.port().to_be_bytes();
    [8, 1, a, b, c, d, h, l]
}

/// Decode exactly one UDP/IPv4 HPAI.
pub fn decode_endpoint(data: &[u8]) -> Result<SocketAddrV4, IpError> {
    let [8, 1, a, b, c, d, h, l] = data else {
        return Err(IpError::Endpoint);
    };
    Ok(SocketAddrV4::new(
        Ipv4Addr::new(*a, *b, *c, *d),
        u16::from_be_bytes([*h, *l]),
    ))
}

pub(crate) fn is_unicast(endpoint: SocketAddrV4) -> bool {
    let ip = endpoint.ip();
    !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast() && endpoint.port() != 0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryRequest {
    Search,
    Description,
}

impl DiscoveryRequest {
    pub fn encode(self, reply_to: SocketAddrV4) -> Result<[u8; 14], IpError> {
        if !is_unicast(reply_to) {
            return Err(IpError::Endpoint);
        }
        let mut frame = [
            6,
            0x10,
            2,
            match self {
                Self::Search => 1,
                Self::Description => 3,
            },
            0,
            14,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        frame[6..].copy_from_slice(&encode_endpoint(reply_to));
        Ok(frame)
    }
}

/// Legacy service family identifiers in the supported-services DIB.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ServiceFamily {
    Core = 2,
    DeviceManagement = 3,
    Tunnelling = 4,
    Routing = 5,
    RemoteLogging = 6,
    RemoteConfigurationDiagnosis = 7,
    ObjectServer = 8,
}

/// Mandatory selection criteria in a Core v2 extended search. Defaults to all gateways.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExtendedSearch {
    pub programming_mode: bool,
    pub mac: Option<[u8; 6]>,
    /// Required family and its minimum advertised version.
    pub service: Option<(ServiceFamily, std::num::NonZeroU8)>,
    /// Ask for the manufacturer's opaque description block when available.
    pub manufacturer_data: bool,
}

impl ExtendedSearch {
    /// Encode an extended UDP/IPv4 request including the two identity DIBs.
    pub fn encode(self, reply_to: SocketAddrV4) -> Result<Vec<u8>, IpError> {
        let mut frame = DiscoveryRequest::Search.encode(reply_to)?.to_vec();
        frame[3] = 0x0b;
        if self.programming_mode {
            frame.extend([2, 0x81]);
        }
        if let Some(mac) = self.mac {
            frame.extend([8, 0x82]);
            frame.extend(mac);
        }
        if let Some((family, version)) = self.service {
            frame.extend([4, 0x83, family as u8, version.get()]);
        }
        if self.manufacturer_data {
            frame.extend([6, 0x84, 1, 2, 0xfe, 0]);
        } else {
            frame.extend([4, 0x84, 1, 2]);
        }
        frame[5] = frame.len() as u8;
        Ok(frame)
    }

    /// Check an extended reply against every requested selection criterion.
    pub fn response(self, datagram: &[u8]) -> Result<Gateway, IpError> {
        let gateway = Gateway::parse_search(datagram, 0x020c)?;
        if (self.programming_mode && !gateway.programming_mode)
            || self.mac.is_some_and(|mac| mac != gateway.mac)
            || self.service.is_some_and(|(family, version)| {
                gateway.advertised_version(family).unwrap_or(0) < version.get()
            })
        {
            return Err(IpError::Selection);
        }
        Ok(gateway)
    }

    /// Run a bounded extended search on a caller-selected multicast interface.
    pub fn discover(
        self,
        socket: UdpSocket,
        deadline: Instant,
        cancelled: impl FnMut() -> bool,
    ) -> io::Result<DiscoveryReport> {
        collect_search(socket, SEARCH_ENDPOINT, Some(self), deadline, cancelled)
    }
}

/// Vendor-defined description bytes associated with a KNX manufacturer identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManufacturerData {
    pub manufacturer: u16,
    pub data: Vec<u8>,
}

/// Weinzierl ObjectServer advertisement. Versions are the raw protocol octets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BaosAdvertisement {
    pub protocol_version: u8,
    /// Required KNXnet/IP security version, when advertised by the device.
    pub required_security_version: Option<u8>,
}

impl ManufacturerData {
    /// Read the ObjectServer records in a Weinzierl DIB (BAOS binary protocol §6.4).
    /// An absent security record is an unauthenticated advertisement, not permission to use plain IP.
    pub fn baos(&self) -> Result<Option<BaosAdvertisement>, IpError> {
        if self.manufacturer != 0x00c5 {
            return Ok(None);
        }
        let mut records = self.data.as_slice();
        let mut version = None;
        let mut security = None;
        while !records.is_empty() {
            let [kind, len, ..] = records else {
                return Err(IpError::Description);
            };
            if *len < 4 || *len % 2 != 0 {
                return Err(IpError::Description);
            }
            let (record, rest) = records
                .split_at_checked(usize::from(*len))
                .ok_or(IpError::Description)?;
            let target = match kind {
                1 => &mut version,
                2 => &mut security,
                _ => return Err(IpError::Description),
            };
            for &[protocol, value] in record[2..].as_chunks::<2>().0 {
                if protocol == 0xf0 && (value == 0 || target.replace(value).is_some()) {
                    return Err(IpError::Description);
                }
            }
            records = rest;
        }
        match (version, security) {
            (Some(protocol_version), required_security_version) => Ok(Some(BaosAdvertisement {
                protocol_version,
                required_security_version,
            })),
            (None, None) => Ok(None),
            (None, Some(_)) => Err(IpError::Description),
        }
    }
}

/// Information advertised by one gateway. Family identifiers and versions are wire numbers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gateway {
    pub control_endpoint: SocketAddrV4,
    pub serial: [u8; 6],
    pub mac: [u8; 6],
    pub programming_mode: bool,
    pub manufacturer_data: Option<ManufacturerData>,
    pub individual_address: u16,
    pub name: String,
    pub service_versions: BTreeMap<u8, u8>,
}

impl Gateway {
    /// Report the version claimed by the peer for a named legacy family.
    pub fn advertised_version(&self, family: ServiceFamily) -> Option<u8> {
        self.service_versions.get(&(family as u8)).copied()
    }

    /// Parse a legacy search response. The returned endpoint is peer-supplied.
    pub fn search_response(datagram: &[u8]) -> Result<Self, IpError> {
        Self::parse_search(datagram, 0x0202)
    }

    fn parse_search(datagram: &[u8], service: u16) -> Result<Self, IpError> {
        let frame = IpFrame::parse(datagram)?;
        if frame.service != service {
            return Err(IpError::Service);
        }
        let (hpai, blocks) = frame.body.split_at_checked(8).ok_or(IpError::Length)?;
        Self::from_blocks(decode_endpoint(hpai)?, blocks)
    }

    /// A description response identifies its endpoint through the UDP peer.
    pub fn description_response(peer: SocketAddrV4, datagram: &[u8]) -> Result<Self, IpError> {
        let frame = IpFrame::parse(datagram)?;
        if frame.service != 0x0204 {
            return Err(IpError::Service);
        }
        Self::from_blocks(peer, frame.body)
    }

    fn from_blocks(endpoint: SocketAddrV4, mut blocks: &[u8]) -> Result<Self, IpError> {
        if !is_unicast(endpoint) {
            return Err(IpError::Endpoint);
        }
        let mut device = None;
        let mut families = None;
        let mut manufacturer_data = None;
        while !blocks.is_empty() {
            let [len, kind, ..] = blocks else {
                return Err(IpError::Description);
            };
            if *len < 2 {
                return Err(IpError::Description);
            }
            let (dib, rest) = blocks
                .split_at_checked(usize::from(*len))
                .ok_or(IpError::Description)?;
            match kind {
                1 => {
                    if device.is_some() || dib.len() != 54 {
                        return Err(IpError::Description);
                    }
                    device = Some(dib);
                }
                2 => {
                    if families.is_some() || dib.len() < 4 || dib.len() % 2 != 0 {
                        return Err(IpError::Description);
                    }
                    let mut versions = BTreeMap::new();
                    for pair in dib[2..].as_chunks::<2>().0 {
                        if pair[1] == 0 || versions.insert(pair[0], pair[1]).is_some() {
                            return Err(IpError::Description);
                        }
                    }
                    families = Some(versions);
                }
                0xfe => {
                    let [_, _, high, low, data @ ..] = dib else {
                        return Err(IpError::Description);
                    };
                    if manufacturer_data.is_some() {
                        return Err(IpError::Description);
                    }
                    manufacturer_data = Some(ManufacturerData {
                        manufacturer: u16::from_be_bytes([*high, *low]),
                        data: data.to_vec(),
                    });
                }
                _ => {}
            }
            blocks = rest;
        }
        let device = device.ok_or(IpError::Description)?;
        let name = &device[24..54];
        let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
        let mut serial = [0; 6];
        serial.copy_from_slice(&device[8..14]);
        Ok(Self {
            control_endpoint: endpoint,
            serial,
            mac: device[18..24].try_into().expect("validated device DIB"),
            programming_mode: device[3] & 1 != 0,
            manufacturer_data,
            individual_address: u16::from_be_bytes([device[4], device[5]]),
            name: name[..end].iter().copied().map(char::from).collect(),
            service_versions: families.ok_or(IpError::Description)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryEnd {
    Deadline,
    Cancelled,
    Capacity,
}

#[derive(Debug)]
pub struct DiscoveryReport {
    pub gateways: Vec<Gateway>,
    pub rejected: usize,
    pub end: DiscoveryEnd,
}

fn wait(socket: &UdpSocket, deadline: Instant) -> io::Result<bool> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Ok(false);
    }
    socket.set_read_timeout(Some(remaining.min(Duration::from_millis(20))))?;
    Ok(true)
}

fn local_endpoint(socket: &UdpSocket) -> io::Result<SocketAddrV4> {
    match socket.local_addr()? {
        SocketAddr::V4(addr) if is_unicast(addr) => Ok(addr),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            IpError::Endpoint,
        )),
    }
}

fn timeout(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

/// Send a legacy search on a caller-selected multicast interface and collect up to
/// 64 distinct gateways. The socket is consumed. Cancellation is polled between
/// datagrams and at intervals of up to 20 ms while idle.
pub fn discover(
    socket: UdpSocket,
    deadline: Instant,
    cancelled: impl FnMut() -> bool,
) -> io::Result<DiscoveryReport> {
    collect_search(socket, SEARCH_ENDPOINT, None, deadline, cancelled)
}

fn collect_search(
    socket: UdpSocket,
    destination: SocketAddrV4,
    extended: Option<ExtendedSearch>,
    deadline: Instant,
    mut cancelled: impl FnMut() -> bool,
) -> io::Result<DiscoveryReport> {
    socket.set_nonblocking(false)?;
    let local = local_endpoint(&socket)?;
    let mut gateways = BTreeMap::new();
    let mut rejected = 0usize;
    let mut buf = vec![0; usize::from(u16::MAX)];
    let mut sent = false;
    let end = loop {
        if cancelled() {
            break DiscoveryEnd::Cancelled;
        }
        if !wait(&socket, deadline)? {
            break DiscoveryEnd::Deadline;
        }
        if !sent {
            let request = match extended {
                Some(search) => search.encode(local),
                None => DiscoveryRequest::Search.encode(local).map(Vec::from),
            }
            .map_err(io::Error::other)?;
            socket.send_to(&request, destination)?;
            sent = true;
        }
        match socket.recv_from(&mut buf) {
            Ok((size, _)) => match extended.map_or_else(
                || Gateway::search_response(&buf[..size]),
                |search| search.response(&buf[..size]),
            ) {
                Ok(gateway) => {
                    gateways
                        .entry((
                            gateway.control_endpoint,
                            gateway.serial,
                            gateway.individual_address,
                        ))
                        .or_insert(gateway);
                    if gateways.len() == 64 {
                        break DiscoveryEnd::Capacity;
                    }
                }
                Err(_) => rejected = rejected.saturating_add(1),
            },
            Err(e) if timeout(&e) => {}
            Err(e) => return Err(e),
        }
    };
    Ok(DiscoveryReport {
        gateways: gateways.into_values().collect(),
        rejected,
        end,
    })
}

/// Request the description of a specific gateway from a local interface. Only a
/// response from that peer can satisfy the query. Cancellation returns Interrupted.
pub fn describe(
    local: Ipv4Addr,
    peer: SocketAddrV4,
    deadline: Instant,
    mut cancelled: impl FnMut() -> bool,
) -> io::Result<Gateway> {
    if !is_unicast(peer) || !is_unicast(SocketAddrV4::new(local, 1)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            IpError::Endpoint,
        ));
    }
    let socket = UdpSocket::bind(SocketAddrV4::new(local, 0))?;
    socket.connect(peer)?;
    let mut buf = vec![0; usize::from(u16::MAX)];
    let mut sent = false;
    loop {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Description cancelled.",
            ));
        }
        if !wait(&socket, deadline)? {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Description deadline reached.",
            ));
        }
        if !sent {
            let request = DiscoveryRequest::Description
                .encode(local_endpoint(&socket)?)
                .map_err(io::Error::other)?;
            socket.send(&request)?;
            sent = true;
        }
        match socket.recv(&mut buf) {
            Ok(size) => {
                if let Ok(gateway) = Gateway::description_response(peer, &buf[..size]) {
                    return Ok(gateway);
                }
            }
            Err(e) if timeout(&e) => {}
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_collector_filters_before_deduplication_and_capacity() {
        let bind = || UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let server = bind();
        let socket = bind();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let target = local_endpoint(&server).unwrap();
        let local = local_endpoint(&socket).unwrap();
        let search = ExtendedSearch {
            programming_mode: true,
            mac: Some([1, 2, 3, 4, 5, 6]),
            service: Some((ServiceFamily::Core, 2.try_into().unwrap())),
            manufacturer_data: true,
        };
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut request = [0; 64];
                let (size, peer) = server.recv_from(&mut request).unwrap();
                assert_eq!(peer, SocketAddr::V4(local));
                assert_eq!(&request[..size], search.encode(local).unwrap());
                let mut data = vec![6, 0x10, 2, 12, 0, 78];
                data.extend(encode_endpoint(target));
                let mut device = [0; 54];
                device[..4].copy_from_slice(&[54, 1, 2, 1]);
                device[18..24].copy_from_slice(&search.mac.unwrap());
                data.extend(device);
                data.extend([4, 2, 2, 2, 6, 0xfe, 0x12, 0x34, 7, 9]);
                // Every rejected reply has the same identity as the first valid one.
                for (offset, value) in [(3, 2), (17, 0), (32, 0), (70, 5), (71, 1)] {
                    let mut invalid = data.clone();
                    invalid[offset] = value;
                    server.send_to(&invalid, peer).unwrap();
                }
                server.send_to(&data[..19], peer).unwrap();
                server.send_to(&data, peer).unwrap();
                server.send_to(&data, peer).unwrap();
                for address in 1..64 {
                    data[19] = address;
                    server.send_to(&data, peer).unwrap();
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            let report = collect_search(
                socket,
                target,
                Some(search),
                Instant::now() + Duration::from_secs(2),
                || false,
            )
            .unwrap();
            assert_eq!(report.end, DiscoveryEnd::Capacity);
            assert_eq!(report.rejected, 6);
            assert_eq!(report.gateways.len(), 64);
            for gateway in report.gateways {
                assert!(gateway.programming_mode);
                assert_eq!(gateway.mac, search.mac.unwrap());
                assert_eq!(gateway.advertised_version(ServiceFamily::Core), Some(2));
                assert_eq!(
                    gateway.manufacturer_data,
                    Some(ManufacturerData {
                        manufacturer: 0x1234,
                        data: vec![7, 9],
                    })
                );
            }
        });
    }

    #[test]
    fn extended_collector_bounds_silent_waits_and_sends_no_request_after_stop() {
        let server = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let target = local_endpoint(&server).unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let search = ExtendedSearch::default();
        for (cancelled, expected) in [
            (true, DiscoveryEnd::Cancelled),
            (false, DiscoveryEnd::Deadline),
        ] {
            let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let report =
                collect_search(socket, target, Some(search), Instant::now(), || cancelled).unwrap();
            assert_eq!(report.end, expected);
            assert!(report.gateways.is_empty());
            let mut buf = [0; 64];
            assert!(timeout(&server.recv_from(&mut buf).unwrap_err()));
        }
        for expected in [DiscoveryEnd::Cancelled, DiscoveryEnd::Deadline] {
            let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let local = local_endpoint(&socket).unwrap();
            let started = Instant::now();
            let mut polls = 0;
            let report = collect_search(
                socket,
                target,
                Some(search),
                started + Duration::from_millis(100),
                || {
                    polls += 1;
                    expected == DiscoveryEnd::Cancelled && polls == 2
                },
            )
            .unwrap();
            assert_eq!(report.end, expected);
            assert!(report.gateways.is_empty());
            assert_eq!(report.rejected, 0);
            assert!(started.elapsed() < Duration::from_secs(1));
            let mut buf = [0; 64];
            let (size, _) = server.recv_from(&mut buf).unwrap();
            assert_eq!(&buf[..size], search.encode(local).unwrap());
            assert!(timeout(&server.recv_from(&mut buf).unwrap_err()));
        }
    }

    #[test]
    fn isolated_search_handles_duplicates_partial_and_distinct_interfaces() {
        let bind = || UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let server = bind();
        let socket = bind();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let target = local_endpoint(&server).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut buf = [0; 14];
                let (_, peer) = server.recv_from(&mut buf).unwrap();
                assert_eq!(&buf[..6], &[6, 0x10, 2, 1, 0, 14]);
                let mut data = vec![6, 0x10, 2, 2, 0, 72];
                data.extend_from_slice(&encode_endpoint(target));
                let mut device = [0; 54];
                device[..2].copy_from_slice(&[54, 1]);
                data.extend_from_slice(&device);
                data.extend_from_slice(&[4, 2, 2, 1]);
                server.send_to(&data[..19], peer).unwrap();
                server.send_to(&data, peer).unwrap();
                server.send_to(&data, peer).unwrap();
                // The same control endpoint can represent multiple KNX interfaces.
                for address in 1..64 {
                    data[19] = address;
                    server.send_to(&data, peer).unwrap();
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            let report = collect_search(
                socket,
                target,
                None,
                Instant::now() + Duration::from_secs(2),
                || false,
            )
            .unwrap();
            assert_eq!(report.gateways.len(), 64);
            assert_eq!(report.rejected, 1);
            assert_eq!(report.end, DiscoveryEnd::Capacity);
        })
    }
}
