//! Read a commissioned KNX IoT switch property through a host-owned CoAP transport.
//!
//! Bind a [`SwitchPoint`] to the commissioned unicast peer, path and access context.
//! Use [`SwitchGet::read`] with your synchronous transport or [`SwitchGet::complete`] after
//! an asynchronous GET. The host owns transport security, token issuance and deadlines.
//!
//! ```
//! use knx::iot::{PointFormat, decode_switch_response};
//! // GET /p/lsab/0/ioo → 2.05 Content, application/cbor
//! assert_eq!(decode_switch_response(69, 60, &[0xa1, 1, 0xf5]), Ok(true));
//! assert_eq!(PointFormat::Json.switch_body(false), b"false");
//! ```

/// Supported content formats for a value-only switch property representation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PointFormat {
    Json,
    Cbor,
}

impl PointFormat {
    /// CoAP Content-Format or Accept option number.
    pub const fn number(self) -> u16 {
        match self {
            Self::Json => 50,
            Self::Cbor => 60,
        }
    }

    /// Response body for a known Point API boolean property.
    pub const fn switch_body(self, on: bool) -> &'static [u8] {
        match (self, on) {
            (Self::Cbor, true) => &[0xa1, 1, 0xf5],
            (Self::Cbor, false) => &[0xa1, 1, 0xf4],
            (Self::Json, true) => b"true",
            (Self::Json, false) => b"false",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PointError {
    /// Expected CoAP 2.05 Content.
    ResponseCode(u8),
    /// Expected a commissioned IPv6 unicast peer and a property path.
    Target,
    /// Response belongs to a different peer, token or security association.
    Exchange,
    /// Expected application/json (50) or application/cbor (60).
    ContentFormat,
    /// Expected a single boolean property value in the selected format.
    Body,
}

impl std::fmt::Display for PointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ResponseCode(code) => {
                return write!(
                    f,
                    "CoAP GET returned {}.{:02}. Expected 2.05 Content.",
                    code >> 5,
                    code & 31
                );
            }
            Self::Target => "Select a commissioned IPv6 unicast property resource.",
            Self::Exchange => {
                "Check the CoAP peer, exchange token and verified security association."
            }
            Self::ContentFormat => "Select a JSON or CBOR Point API response.",
            Self::Body => "Supply a value-only response for the commissioned boolean property.",
        })
    }
}
impl std::error::Error for PointError {}

/// Decode the successful GET response for the selected boolean resource.
/// Supply the packed CoAP response code (2.05 = 69), and the received Content-Format.
pub fn decode_switch_response(
    code: u8,
    content_format: u16,
    body: &[u8],
) -> Result<bool, PointError> {
    if code != 69 {
        return Err(PointError::ResponseCode(code));
    }
    match content_format {
        50 => {
            let whitespace = |byte: &u8| matches!(byte, b' ' | b'\n' | b'\r' | b'\t');
            let start = body
                .iter()
                .position(|b| !whitespace(b))
                .ok_or(PointError::Body)?;
            let end = body
                .iter()
                .rposition(|b| !whitespace(b))
                .ok_or(PointError::Body)?;
            match &body[start..=end] {
                b"true" => Ok(true),
                b"false" => Ok(false),
                _ => Err(PointError::Body),
            }
        }
        60 => {
            let map = if let [0xbf, entries @ .., 0xff] = body {
                entries
            } else {
                strip_cbor_one(body, 0xa0).ok_or(PointError::Body)?
            };
            match strip_cbor_one(map, 0) {
                Some([0xf4]) => Ok(false),
                Some([0xf5]) => Ok(true),
                _ => Err(PointError::Body),
            }
        }
        _ => Err(PointError::ContentFormat),
    }
}

// RFC 8949 permits wider arguments for a one-entry map or the unsigned integer key 1.
fn strip_cbor_one(input: &[u8], major: u8) -> Option<&[u8]> {
    let (&head, rest) = input.split_first()?;
    match head.checked_sub(major)? {
        1 => Some(rest),
        width @ 24..=27 => {
            let (argument, tail) = rest.split_at_checked(1 << (width - 24))?;
            let (last, leading) = argument.split_last()?;
            (*last == 1 && leading.iter().all(|&b| b == 0)).then_some(tail)
        }
        _ => None,
    }
}

use std::net::SocketAddrV6;

/// Security association selected by the host for this direct CoAP exchange.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PointAccess {
    Plain,
    /// A host-assigned, nonsecret handle. The transport authenticates the response
    /// in this exact OSCORE context before reporting the handle to the reader.
    Oscore(u64),
}

/// One commissioned boolean property, including any vendor-specific base path.
#[derive(Clone, Copy)]
pub struct SwitchPoint<'a> {
    peer: SocketAddrV6,
    segments: &'a [&'a str],
    access: PointAccess,
}

impl<'a> SwitchPoint<'a> {
    /// Supply decoded Uri-Path segments, e.g. `&["p", "lsab", "0", "ioo"]`.
    /// Each segment becomes one CoAP option. The host owns commissioning and access policy.
    pub fn new(
        peer: SocketAddrV6,
        segments: &'a [&'a str],
        access: PointAccess,
    ) -> Result<Self, PointError> {
        let ip = peer.ip();
        if peer.port() == 0
            || ip.is_multicast()
            || ip.is_unspecified()
            || ip.to_ipv4_mapped().is_some()
            || (ip.is_unicast_link_local() && peer.scope_id() == 0)
            || !segments.windows(2).any(|part| part[0] == "p")
            || segments
                .iter()
                .any(|part| part.is_empty() || part.len() > 255 || matches!(*part, "." | ".."))
        {
            return Err(PointError::Target);
        }
        Ok(Self {
            peer,
            segments,
            access,
        })
    }

    /// Start a value-only GET. Supply a fresh eight-byte token from the host CoAP stack.
    /// Send a single GET with the selected Accept and Uri-Path options. Handlers for
    /// transport errors, blockwise transfers and timeouts belong to that stack.
    pub const fn get(self, accept: PointFormat, token: [u8; 8]) -> SwitchGet<'a> {
        SwitchGet {
            point: self,
            accept,
            token,
        }
    }
}

/// Outstanding request. Use the same value to check the result after synchronous or async I/O.
pub struct SwitchGet<'a> {
    point: SwitchPoint<'a>,
    accept: PointFormat,
    token: [u8; 8],
}

impl<'a> SwitchGet<'a> {
    pub const fn peer(&self) -> SocketAddrV6 {
        self.point.peer
    }
    pub const fn uri_path(&self) -> &'a [&'a str] {
        self.point.segments
    }
    pub const fn access(&self) -> PointAccess {
        self.point.access
    }
    pub const fn accept(&self) -> PointFormat {
        self.accept
    }
    pub const fn token(&self) -> [u8; 8] {
        self.token
    }

    /// Complete the exchange once using the actual remote endpoint and token.
    /// For OSCORE, pass the verified inner response and its authenticated context.
    /// The host must return a transport error if verification or reassembly fails.
    pub fn complete(self, reply: PointReply<'_>) -> Result<bool, PointError> {
        let peer = self.peer();
        if (reply.peer.ip(), reply.peer.port(), reply.peer.scope_id())
            != (peer.ip(), peer.port(), peer.scope_id())
            || reply.token != self.token
            || reply.access != self.access()
        {
            return Err(PointError::Exchange);
        }
        if reply.code != 69 {
            return Err(PointError::ResponseCode(reply.code));
        }
        if reply.content_format != Some(self.accept.number()) {
            return Err(PointError::ContentFormat);
        }
        decode_switch_response(reply.code, self.accept.number(), reply.body)
    }
}

/// Metadata and reassembled body returned by the host's CoAP transport for one reply.
pub struct PointReply<'a> {
    pub peer: SocketAddrV6,
    pub token: &'a [u8],
    pub access: PointAccess,
    pub code: u8,
    pub content_format: Option<u16>,
    pub body: &'a [u8],
}

/// Host-owned CoAP exchange: perform the authorized GET and return its actual response.
/// Correlate message-layer acknowledgements and handle retransmission or cancellation here.
pub trait PointTransport {
    type Error;
    fn get(&mut self, request: &SwitchGet<'_>) -> Result<PointReply<'_>, Self::Error>;
}

/// Distinguishes a transport failure from an unusable or mismatched Point API response.
#[derive(Debug)]
pub enum PointReadError<E> {
    Transport(E),
    Response(PointError),
}

impl<E> std::fmt::Display for PointReadError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(_) => {
                f.write_str("The CoAP transport could not complete the property read.")
            }
            Self::Response(error) => error.fmt(f),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PointReadError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Response(e) => Some(e),
        }
    }
}

impl SwitchGet<'_> {
    /// Perform one commissioned read through a host-owned CoAP transport.
    pub fn read<T: PointTransport>(
        self,
        transport: &mut T,
    ) -> Result<bool, PointReadError<T::Error>> {
        let reply = transport.get(&self).map_err(PointReadError::Transport)?;
        self.complete(reply).map_err(PointReadError::Response)
    }
}
