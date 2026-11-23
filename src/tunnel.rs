//! UDP/IPv4 link-layer tunnelling for caller-owned sockets.
//!
//! Send `connect_request` to the selected control peer, then pass its reply to
//! `Tunnel::accept`. Check the UDP source endpoint before passing each datagram.
//! Use the returned data endpoint for traffic and the control peer for teardown.

use crate::ip_core::{IpFrame, decode_endpoint, encode_endpoint, is_unicast};
use crate::telegram::{GroupTelegram, LinkPrimitive};
use std::{fmt, net::SocketAddrV4, time::Duration};

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);
pub const ACK_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TunnelError {
    Malformed,
    Refused(u8),
    Busy,
    Closed,
    Unexpected,
}
impl fmt::Display for TunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "Supply a complete UDP/IPv4 link-layer tunnel frame.",
            Self::Refused(_) => {
                "The gateway refused the tunnel. Check available channels and configuration."
            }
            Self::Busy => "Finish the pending request before submitting another.",
            Self::Closed => "Establish a new tunnel before sending.",
            Self::Unexpected => "Check the peer, channel, service and sequence.",
        })
    }
}
impl std::error::Error for TunnelError {}

/// Advertise concrete local control and data endpoints for a link-layer connection.
pub fn connect_request(control: SocketAddrV4, data: SocketAddrV4) -> Result<[u8; 26], TunnelError> {
    if !is_unicast(control) || !is_unicast(data) {
        return Err(TunnelError::Malformed);
    }
    let mut packet = [0; 26];
    packet[..6].copy_from_slice(&[6, 16, 2, 5, 0, 26]);
    packet[6..14].copy_from_slice(&encode_endpoint(control));
    packet[14..22].copy_from_slice(&encode_endpoint(data));
    packet[22..].copy_from_slice(&[4, 4, 2, 0]);
    Ok(packet)
}

#[derive(Debug, Eq, PartialEq)]
pub enum Event<'a> {
    /// Gateway receipt of a request. Bus confirmation arrives separately.
    Acknowledged,
    /// Send the acknowledgement. Deliver a present telegram once.
    Incoming {
        ack: [u8; 10],
        telegram: Option<GroupTelegram<'a>>,
    },
    /// A complete link-layer indication or confirmation falls outside the group decoder.
    Unhandled { ack: [u8; 10], cemi: &'a [u8] },
    /// A sequence outside the receive window requires no acknowledgement.
    Discard,
    /// Send the exact pending datagram once more.
    Retry(&'a [u8]),
    /// Delivery is uncertain. Send a disconnect to the control endpoint.
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlEvent {
    Healthy,
    RetryHeartbeat([u8; 16]),
    /// Send to the selected control peer. A pending command has unknown delivery.
    Disconnect {
        request: [u8; 16],
        delivery_uncertain: bool,
    },
    /// Reply to a server-initiated disconnect on the control endpoint.
    Disconnected {
        response: [u8; 8],
        delivery_uncertain: bool,
    },
}

pub struct Tunnel {
    heartbeat: Option<(SocketAddrV4, u8)>,
    channel: u8,
    control_endpoint: SocketAddrV4,
    data_endpoint: SocketAddrV4,
    assigned_address: u16,
    transmit: u8,
    receive: u8,
    closed: bool,
    retried: bool,
    pending_len: usize,
    pending: [u8; 10 + GroupTelegram::MAX_ENCODED_LEN],
}
impl Tunnel {
    /// Accept a CONNECT_RESPONSE from the selected control peer.
    pub fn accept(
        control_endpoint: SocketAddrV4,
        sender: SocketAddrV4,
        datagram: &[u8],
    ) -> Result<Self, TunnelError> {
        if !is_unicast(control_endpoint) || sender != control_endpoint {
            return Err(TunnelError::Unexpected);
        }
        let frame = IpFrame::parse(datagram).map_err(|_| TunnelError::Malformed)?;
        if frame.service != 0x0206 {
            return Err(TunnelError::Unexpected);
        }
        let [channel, status, rest @ ..] = frame.body else {
            return Err(TunnelError::Malformed);
        };
        if *status != 0 {
            if !rest.is_empty() {
                return Err(TunnelError::Malformed);
            }
            return Err(TunnelError::Refused(*status));
        }
        let [hpai @ .., 4, 4, hi, lo] = rest else {
            return Err(TunnelError::Malformed);
        };
        let endpoint = decode_endpoint(hpai).map_err(|_| TunnelError::Malformed)?;
        if !is_unicast(endpoint) {
            return Err(TunnelError::Malformed);
        }
        Ok(Self {
            heartbeat: None,
            control_endpoint,
            channel: *channel,
            data_endpoint: endpoint,
            assigned_address: u16::from_be_bytes([*hi, *lo]),
            transmit: 0,
            receive: 0,
            closed: false,
            retried: false,
            pending_len: 0,
            pending: [0; 10 + GroupTelegram::MAX_ENCODED_LEN],
        })
    }

    pub fn data_endpoint(&self) -> SocketAddrV4 {
        self.data_endpoint
    }
    /// Peer/sequence correlation on this UDP channel supplies plain IP evidence.
    pub const fn ip_protection(&self) -> crate::ProtectionEvidence {
        crate::ProtectionEvidence::Plain
    }

    pub fn assigned_address(&self) -> u16 {
        self.assigned_address
    }

    /// Stage one request. Start the acknowledgement timer when the datagram is sent.
    pub fn request(&mut self, telegram: &GroupTelegram<'_>) -> Result<&[u8], TunnelError> {
        self.check_available()?;
        if telegram.primitive != LinkPrimitive::Request {
            return Err(TunnelError::Unexpected);
        }
        let size = telegram
            .encode(&mut self.pending[10..])
            .map_err(|_| TunnelError::Malformed)?;
        Ok(self.stage(size))
    }

    /// Stage a sealed group request whose authenticated source matches this tunnel.
    #[cfg(feature = "data-secure")]
    pub fn secure_request(
        &mut self,
        request: &crate::data_secure::SecureRequest,
    ) -> Result<&[u8], TunnelError> {
        self.check_available()?;
        if request.source() != self.assigned_address {
            return Err(TunnelError::Unexpected);
        }
        let frame = request.as_bytes();
        self.pending[10..]
            .get_mut(..frame.len())
            .ok_or(TunnelError::Malformed)?
            .copy_from_slice(frame);
        Ok(self.stage(frame.len()))
    }

    fn check_available(&self) -> Result<(), TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        if self.pending_len != 0 {
            return Err(TunnelError::Busy);
        }
        Ok(())
    }

    fn stage(&mut self, frame_len: usize) -> &[u8] {
        let len = 10 + frame_len;
        let [high, low] = (len as u16).to_be_bytes();
        self.pending[..10].copy_from_slice(&[
            6,
            16,
            4,
            32,
            high,
            low,
            4,
            self.channel,
            self.transmit,
            0,
        ]);
        self.pending_len = len;
        self.retried = false;
        &self.pending[..len]
    }

    /// Handle a one-second acknowledgement timeout. Restart the timer after a retry.
    pub fn ack_timeout(&mut self) -> Result<Event<'_>, TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        if self.pending_len == 0 {
            return Err(TunnelError::Unexpected);
        }
        if self.retried {
            self.closed = true;
            self.pending_len = 0;
            return Ok(Event::Indeterminate);
        }
        self.retried = true;
        Ok(Event::Retry(&self.pending[..self.pending_len]))
    }

    /// Accept only frames from the negotiated data endpoint.
    pub fn receive<'a>(
        &'a mut self,
        peer: SocketAddrV4,
        datagram: &'a [u8],
    ) -> Result<Event<'a>, TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        if peer != self.data_endpoint {
            return Err(TunnelError::Unexpected);
        }
        let frame = IpFrame::parse(datagram).map_err(|_| TunnelError::Malformed)?;
        let [4, channel, seq, status, rest @ ..] = frame.body else {
            return Err(TunnelError::Malformed);
        };
        if *channel != self.channel {
            return Err(TunnelError::Unexpected);
        }
        match frame.service {
            0x0421 => {
                if !rest.is_empty() {
                    return Err(TunnelError::Malformed);
                }
                if self.pending_len == 0 || *seq != self.transmit {
                    return Err(TunnelError::Unexpected);
                }
                if *status != 0 {
                    return self.ack_timeout();
                }
                self.pending_len = 0;
                self.transmit = self.transmit.wrapping_add(1);
                Ok(Event::Acknowledged)
            }
            0x0420 => {
                if *status != 0 {
                    return Err(TunnelError::Malformed);
                }
                let ack = [6, 16, 4, 33, 0, 10, 4, *channel, *seq, 0];
                if *seq == self.receive.wrapping_sub(1) {
                    return Ok(Event::Incoming {
                        ack,
                        telegram: None,
                    });
                }
                if *seq != self.receive {
                    return Ok(Event::Discard);
                }
                // Validate the cEMI envelope independently of our supported group services.
                let [0x29 | 0x2e, info_len, data @ ..] = rest else {
                    return Err(TunnelError::Malformed);
                };
                let body = data
                    .get(usize::from(*info_len)..)
                    .ok_or(TunnelError::Malformed)?;
                let npdu_len = body.get(6).ok_or(TunnelError::Malformed)?;
                if body.len() != 8 + usize::from(*npdu_len) {
                    return Err(TunnelError::Malformed);
                }
                self.receive = self.receive.wrapping_add(1);
                Ok(match GroupTelegram::parse(rest) {
                    Ok(telegram) => Event::Incoming {
                        ack,
                        telegram: Some(telegram),
                    },
                    Err(_) => Event::Unhandled { ack, cemi: rest },
                })
            }
            _ => Err(TunnelError::Unexpected),
        }
    }

    /// Begin a heartbeat after HEARTBEAT_INTERVAL. Arm HEARTBEAT_TIMEOUT on send.
    pub fn heartbeat(&mut self, local: SocketAddrV4) -> Result<[u8; 16], TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        if self.heartbeat.is_some() {
            return Err(TunnelError::Busy);
        }
        let packet = self.control_request(7, local)?;
        self.heartbeat = Some((local, 0));
        Ok(packet)
    }

    /// Retry a failed heartbeat up to three times, then disconnect.
    pub fn heartbeat_timeout(&mut self) -> Result<ControlEvent, TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        let (local, retries) = self.heartbeat.ok_or(TunnelError::Unexpected)?;
        if retries < 3 {
            self.heartbeat = Some((local, retries + 1));
            return Ok(ControlEvent::RetryHeartbeat(
                self.control_request(7, local)?,
            ));
        }
        let delivery_uncertain = self.has_pending_request();
        Ok(ControlEvent::Disconnect {
            request: self.disconnect(local)?,
            delivery_uncertain,
        })
    }

    pub fn heartbeat_response(
        &mut self,
        peer: SocketAddrV4,
        datagram: &[u8],
    ) -> Result<ControlEvent, TunnelError> {
        if self.closed {
            return Err(TunnelError::Closed);
        }
        if peer != self.control_endpoint || self.heartbeat.is_none() {
            return Err(TunnelError::Unexpected);
        }
        let frame = IpFrame::parse(datagram).map_err(|_| TunnelError::Malformed)?;
        let [channel, status] = frame.body else {
            return Err(TunnelError::Malformed);
        };
        if frame.service != 0x0208 || *channel != self.channel {
            return Err(TunnelError::Unexpected);
        }
        if *status != 0 {
            return self.heartbeat_timeout();
        }
        self.heartbeat = None;
        Ok(ControlEvent::Healthy)
    }

    pub fn remote_disconnect(
        &mut self,
        peer: SocketAddrV4,
        datagram: &[u8],
    ) -> Result<ControlEvent, TunnelError> {
        if peer != self.control_endpoint {
            return Err(TunnelError::Unexpected);
        }
        let frame = IpFrame::parse(datagram).map_err(|_| TunnelError::Malformed)?;
        let [channel, 0, hpai @ ..] = frame.body else {
            return Err(TunnelError::Malformed);
        };
        if frame.service != 0x0209 || *channel != self.channel {
            return Err(TunnelError::Unexpected);
        }
        let endpoint = decode_endpoint(hpai).map_err(|_| TunnelError::Malformed)?;
        if endpoint != peer {
            return Err(TunnelError::Unexpected);
        }
        let delivery_uncertain = self.has_pending_request();
        self.close();
        Ok(ControlEvent::Disconnected {
            response: [6, 16, 2, 10, 0, 8, *channel, 0],
            delivery_uncertain,
        })
    }

    fn close(&mut self) {
        self.closed = true;
        self.pending_len = 0;
        self.heartbeat = None;
    }

    /// Validate the server reply to the local disconnect on the control channel.
    pub fn disconnect_response(
        &self,
        peer: SocketAddrV4,
        datagram: &[u8],
    ) -> Result<(), TunnelError> {
        if !self.closed || peer != self.control_endpoint {
            return Err(TunnelError::Unexpected);
        }
        let frame = IpFrame::parse(datagram).map_err(|_| TunnelError::Malformed)?;
        let [channel, status] = frame.body else {
            return Err(TunnelError::Malformed);
        };
        if frame.service != 0x020a || *channel != self.channel {
            return Err(TunnelError::Unexpected);
        }
        if *status != 0 {
            return Err(TunnelError::Refused(*status));
        }
        Ok(())
    }

    pub fn has_pending_request(&self) -> bool {
        self.pending_len != 0
    }

    fn control_request(&self, service: u8, local: SocketAddrV4) -> Result<[u8; 16], TunnelError> {
        if !is_unicast(local) {
            return Err(TunnelError::Malformed);
        }
        let mut packet = [
            6,
            16,
            2,
            service,
            0,
            16,
            self.channel,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        packet[8..].copy_from_slice(&encode_endpoint(local));
        Ok(packet)
    }

    /// Produce a DISCONNECT_REQUEST and close this channel. Pending delivery is uncertain.
    pub fn disconnect(&mut self, local_control: SocketAddrV4) -> Result<[u8; 16], TunnelError> {
        let packet = self.control_request(9, local_control)?;
        self.close();
        Ok(packet)
    }
}
