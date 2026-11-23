//! Password-authenticated import of ETS keyring v1 exports.
//!
//! Select a local interface and group before creating a Data Secure binding, or
//! select the commissioned gateway and tunnel slot to start an IP Secure session.
//! Replay state belongs to the application and survives subsequent keyring imports.
use crate::{
    GroupAddress,
    data_secure::{GroupReceiver, GroupSender},
    ip_secure::Handshake,
};
use aes::{
    Aes128,
    cipher::{BlockDecryptMut, KeyIvInit},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use pbkdf2::pbkdf2_hmac;
use roxmltree::{Document, Node, ParsingOptions};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

const NAMESPACE: &str = "http://knx.org/xml/keyring/1";
/// Maximum UTF-8 XML bytes accepted by the offline importer.
pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
    Format,
    Limit,
    Authentication,
    Binding,
    Entropy,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Format => "Supply a supported ETS keyring v1 export.",
            Self::Limit => "The keyring exceeds the import limits.",
            Self::Authentication => "The keyring signature or password could not be verified.",
            Self::Binding => "The export has no unique matching commissioned binding.",
            Self::Entropy => "System randomness is unavailable for the session.",
        })
    }
}
impl std::error::Error for Error {}

fn derive(password: &[u8], salt: &[u8]) -> Zeroizing<[u8; 16]> {
    let mut key = Zeroizing::new([0; 16]);
    pbkdf2_hmac::<Sha256>(password, salt, 65_536, &mut *key);
    key
}
fn field(hash: &mut Sha256, value: &[u8]) -> Result<(), Error> {
    let length = u8::try_from(value.len()).map_err(|_| Error::Limit)?;
    hash.update([length]);
    hash.update(value);
    Ok(())
}
// A maximum depth bounds recursion independently of the total XML node limit.
fn canonical(node: Node<'_, '_>, hash: &mut Sha256, depth: usize) -> Result<(), Error> {
    if depth > 32 || node.attributes().len() > 64 {
        return Err(Error::Limit);
    }
    if node.tag_name().namespace() != Some(NAMESPACE) {
        return Err(Error::Format);
    }
    hash.update([1]);
    field(hash, node.tag_name().name().as_bytes())?;
    let mut attrs: Vec<_> = node.attributes().collect();
    attrs.sort_unstable_by_key(|a| a.name());
    for attr in attrs {
        if attr.namespace().is_some() {
            return Err(Error::Format);
        }
        if attr.name() == "Signature" {
            if depth == 0 {
                continue;
            }
            return Err(Error::Format);
        }
        field(hash, attr.name().as_bytes())?;
        field(hash, attr.value().as_bytes())?;
    }
    for child in node.children() {
        if child.is_element() {
            canonical(child, hash, depth + 1)?;
        } else if child.is_text() && !child.text().unwrap_or("").trim().is_empty() {
            return Err(Error::Format);
        }
    }
    hash.update([2]);
    Ok(())
}
fn individual(value: &str) -> Result<u16, Error> {
    let mut parts = value.split('.');
    let mut component = || {
        parts
            .next()
            .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|v| v.parse::<u8>().ok())
            .ok_or(Error::Format)
    };
    let a = component()?;
    let l = component()?;
    let d = component()?;
    if a > 15 || l > 15 || parts.next().is_some() {
        return Err(Error::Format);
    }
    Ok(u16::from_be_bytes([a << 4 | l, d]))
}
fn attr<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str, Error> {
    node.attribute(name).ok_or(Error::Format)
}
fn group(node: Node<'_, '_>) -> Result<GroupAddress, Error> {
    attr(node, "Address")?.parse().map_err(|_| Error::Format)
}

fn decoded<const N: usize>(node: Node<'_, '_>, name: &str) -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    let length = STANDARD
        .decode_slice(attr(node, name)?, &mut bytes)
        .map_err(|_| Error::Format)?;
    if length != N {
        return Err(Error::Format);
    }
    Ok(bytes)
}

struct Interface {
    host: Option<u16>,
    credentials: Option<(u8, [u8; 32], [u8; 32])>,
    groups: BTreeMap<GroupAddress, BTreeSet<u16>>,
}

/// A verified export. Selected keys and interface credentials are decrypted on use.
pub struct Keyring {
    key: Zeroizing<[u8; 16]>,
    iv: [u8; 16],
    interfaces: BTreeMap<u16, Interface>,
    groups: BTreeMap<GroupAddress, [u8; 16]>,
}
impl Keyring {
    /// Verify the entire export before indexing any commissioned bindings.
    pub fn open(xml: &str, password: &str) -> Result<Self, Error> {
        if xml.len() > MAX_XML_BYTES || password.len() > 1024 {
            return Err(Error::Limit);
        }
        if password.is_empty() {
            return Err(Error::Authentication);
        }
        let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
        let document = Document::parse_with_options(
            xml,
            ParsingOptions {
                nodes_limit: 100_000,
                ..Default::default()
            },
        )
        .map_err(|_| Error::Format)?;
        let root = document.root_element();
        if root.tag_name().name() != "Keyring" {
            return Err(Error::Format);
        }
        let signature = decoded::<16>(root, "Signature").map_err(|_| Error::Authentication)?;
        let mut hash = Sha256::new();
        canonical(root, &mut hash, 0)?;
        let key = derive(password.as_bytes(), b"1.keyring.ets.knx.org");
        let mut base64_key = Zeroizing::new([0; 24]);
        STANDARD
            .encode_slice(&key[..], &mut base64_key[..])
            .map_err(|_| Error::Format)?;
        field(&mut hash, &base64_key[..])?;
        if !bool::from(hash.finalize()[..16].ct_eq(&signature)) {
            return Err(Error::Authentication);
        }
        let created = attr(root, "Created")?;
        let iv = Sha256::digest(created.as_bytes())[..16]
            .try_into()
            .expect("sixteen-byte IV");
        let mut result = Self {
            key,
            iv,
            interfaces: BTreeMap::new(),
            groups: BTreeMap::new(),
        };
        for section in root.children().filter(Node::is_element) {
            match section.tag_name().name() {
                "GroupAddresses" => {
                    for node in section
                        .children()
                        .filter(|n| n.has_tag_name((NAMESPACE, "Group")))
                    {
                        if result
                            .groups
                            .insert(group(node)?, decoded(node, "Key")?)
                            .is_some()
                        {
                            return Err(Error::Binding);
                        }
                    }
                }
                "Interface" => {
                    let Some(address) = section.attribute("IndividualAddress") else {
                        continue;
                    };
                    let host = section.attribute("Host").map(individual).transpose()?;
                    let credentials = match (
                        section.attribute("Type"),
                        section.attribute("UserID"),
                        section.attribute("Password"),
                        section.attribute("Authentication"),
                    ) {
                        (Some("Tunneling"), Some(user), Some(_), Some(_)) => {
                            let user = user
                                .parse::<u8>()
                                .ok()
                                .filter(|u| (1..=127).contains(u))
                                .ok_or(Error::Format)?;
                            Some((
                                user,
                                decoded(section, "Password")?,
                                decoded(section, "Authentication")?,
                            ))
                        }
                        _ => None,
                    };
                    let mut groups = BTreeMap::new();
                    for node in section
                        .children()
                        .filter(|n| n.has_tag_name((NAMESPACE, "Group")))
                    {
                        let senders = node
                            .attribute("Senders")
                            .unwrap_or("")
                            .split_ascii_whitespace()
                            .map(individual)
                            .collect::<Result<BTreeSet<_>, _>>()?;
                        if groups.insert(group(node)?, senders).is_some() {
                            return Err(Error::Binding);
                        }
                    }
                    if result
                        .interfaces
                        .insert(
                            individual(address)?,
                            Interface {
                                host,
                                credentials,
                                groups,
                            },
                        )
                        .is_some()
                    {
                        return Err(Error::Binding);
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }
    fn decrypt<const N: usize>(&self, encrypted: &[u8; N]) -> Zeroizing<[u8; N]> {
        let mut plain = Zeroizing::new(*encrypted);
        let mut cipher = cbc::Decryptor::<Aes128>::new((&*self.key).into(), (&self.iv).into());
        for block in plain.as_chunks_mut::<16>().0 {
            cipher.decrypt_block_mut(block.into());
        }
        plain
    }
    fn group_key(&self, group: GroupAddress) -> Result<Zeroizing<[u8; 16]>, Error> {
        Ok(self.decrypt(self.groups.get(&group).ok_or(Error::Binding)?))
    }
    fn assigned(&self, interface: u16, group: GroupAddress) -> Result<&BTreeSet<u16>, Error> {
        self.interfaces
            .get(&interface)
            .and_then(|i| i.groups.get(&group))
            .ok_or(Error::Binding)
    }
    /// Bind reception to a sender listed for this local interface and group.
    pub fn group_receiver(
        &self,
        interface: u16,
        sender: u16,
        group: GroupAddress,
    ) -> Result<GroupReceiver, Error> {
        if !self.assigned(interface, group)?.contains(&sender) {
            return Err(Error::Binding);
        }
        Ok(GroupReceiver::new(sender, group, *self.group_key(group)?))
    }
    /// Bind outgoing group traffic to the commissioned local interface.
    pub fn group_sender(&self, interface: u16, group: GroupAddress) -> Result<GroupSender, Error> {
        self.assigned(interface, group)?;
        Ok(GroupSender::new(interface, group, *self.group_key(group)?))
    }
    fn credential_hash(
        &self,
        encoded: &[u8; 32],
        salt: &[u8],
    ) -> Result<Zeroizing<[u8; 16]>, Error> {
        let plain = self.decrypt(encoded);
        let pad = usize::from(plain[31]);
        if !(1..=24).contains(&pad) || !plain[32 - pad..].iter().all(|b| usize::from(*b) == pad) {
            return Err(Error::Format);
        }
        let password = &plain[8..32 - pad];
        if password.is_empty() || !password.iter().all(|b| (0x20..=0x7e).contains(b)) {
            return Err(Error::Format);
        }
        Ok(derive(password, salt))
    }
    /// Start mutual authentication for one gateway and its ETS tunnelling slot.
    pub fn handshake(
        &self,
        host: u16,
        slot: u16,
        client_serial: [u8; 6],
    ) -> Result<Handshake, Error> {
        let interface = self
            .interfaces
            .get(&slot)
            .filter(|i| i.host == Some(host))
            .ok_or(Error::Binding)?;
        let (user, password, authentication) = interface.credentials.ok_or(Error::Binding)?;
        let device = self.credential_hash(
            &authentication,
            b"device-authentication-code.1.secure.ip.knx.org",
        )?;
        let user_key = self.credential_hash(&password, b"user-password.1.secure.ip.knx.org")?;
        Handshake::start(client_serial, user, *device, *user_key).map_err(|_| Error::Entropy)
    }
}
