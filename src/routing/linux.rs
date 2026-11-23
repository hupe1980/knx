//! Linux per-datagram interface selection and receive-queue telemetry.

use nix::sys::socket::{ControlMessageOwned, MsgFlags, SockaddrIn, recvmsg, setsockopt, sockopt};
use std::{
    io::{self, IoSliceMut},
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    os::fd::AsRawFd,
};

/// Borrowed bytes and kernel metadata for one complete IPv4 UDP datagram.
#[derive(Debug)]
pub struct RoutingDatagram<'a> {
    pub bytes: &'a [u8],
    pub source: SocketAddrV4,
    /// IP destination from the packet header. Check it against the commissioned group.
    pub destination: Ipv4Addr,
    /// Local IPv4 address supplied in IP_PKTINFO.
    pub local_address: Ipv4Addr,
    /// Kernel interface index supplied in IP_PKTINFO.
    pub interface_index: u32,
    /// SO_RXQ_OVFL count since socket creation, when the kernel supplies it.
    /// Compute a wrapping difference between samples on the same socket.
    pub socket_drop_total: Option<u32>,
}

/// Linux recvmsg adapter for a host-configured IPv4 socket. The caller owns multicast
/// membership and can configure timeouts and nonblocking I/O before construction.
#[derive(Debug)]
pub struct LinuxRoutingSocket {
    socket: UdpSocket,
    ancillary: Vec<u8>,
}

impl LinuxRoutingSocket {
    pub fn new(socket: UdpSocket) -> io::Result<Self> {
        if !socket.local_addr()?.is_ipv4() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Bind an IPv4 routing socket.",
            ));
        }
        setsockopt(&socket, sockopt::Ipv4PacketInfo, &true)?;
        setsockopt(&socket, sockopt::RxqOvfl, &1)?;
        Ok(Self {
            socket,
            ancillary: nix::cmsg_space!(nix::libc::in_pktinfo, u32),
        })
    }

    /// Receive into caller storage. Reject truncation of either the payload or metadata.
    pub fn receive<'a>(&mut self, buffer: &'a mut [u8]) -> io::Result<RoutingDatagram<'a>> {
        let mut slices = [IoSliceMut::new(buffer)];
        let msg = recvmsg::<SockaddrIn>(
            self.socket.as_raw_fd(),
            &mut slices,
            Some(&mut self.ancillary),
            MsgFlags::empty(),
        )?;
        if msg
            .flags
            .intersects(MsgFlags::MSG_TRUNC | MsgFlags::MSG_CTRUNC)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Incomplete routing datagram or kernel metadata.",
            ));
        }
        let mut info = None;
        let mut socket_drop_total = None;
        for cmsg in msg.cmsgs()? {
            match cmsg {
                ControlMessageOwned::Ipv4PacketInfo(value) => info = Some(value),
                ControlMessageOwned::RxqOvfl(value) => socket_drop_total = Some(value),
                _ => {}
            }
        }
        let missing =
            || io::Error::new(io::ErrorKind::InvalidData, "Missing IPv4 routing metadata.");
        let info = info.ok_or_else(missing)?;
        let source = msg.address.ok_or_else(missing)?.into();
        let size = msg.bytes;
        Ok(RoutingDatagram {
            bytes: &buffer[..size],
            source,
            socket_drop_total,
            destination: Ipv4Addr::from(info.ipi_addr.s_addr.to_ne_bytes()),
            local_address: Ipv4Addr::from(info.ipi_spec_dst.s_addr.to_ne_bytes()),
            interface_index: u32::try_from(info.ipi_ifindex).map_err(|_| missing())?,
        })
    }
}
