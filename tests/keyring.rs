#![cfg(feature = "keyring")]
use knx::{
    data_secure::{GroupSender, OutgoingSequenceStore},
    keyring::{Error, Keyring, MAX_XML_BYTES},
    telegram::GroupService,
};

const XML: &str = include_str!("../tools/interop/loopback.knxkeys");

struct One;
impl OutgoingSequenceStore for One {
    type Error = ();
    fn reserve(&mut self, source: u16) -> Result<Option<u64>, ()> {
        assert_eq!(source, 0x7101);
        Ok(Some(1))
    }
}

#[test]
fn authenticated_export_binds_keys_to_the_commissioned_scope() {
    let xml = format!(
        "\u{feff}{}",
        XML.replace(
            "Project=\"Example\" CreatedBy=\"Fixture\"",
            "CreatedBy=\"Fixture\" Project=\"Examp&#108;e\""
        )
    );
    let keys = Keyring::open(&xml, "example").unwrap();
    let group = "1/1/1".parse().unwrap();
    let sender = keys.group_sender(0x7101, group).unwrap();
    let actual = sender.request(GroupService::Read, &mut One).unwrap();
    let expected = GroupSender::new(0x7101, group, std::array::from_fn(|n| n as u8))
        .request(GroupService::Read, &mut One)
        .unwrap();
    assert_eq!(actual.as_bytes(), expected.as_bytes());
    assert!(keys.group_receiver(0x7101, 0x710c, group).is_ok());
    assert!(matches!(
        keys.group_receiver(0x7101, 0x710d, group),
        Err(Error::Binding)
    ));
    assert!(matches!(
        keys.group_sender(0x7102, group),
        Err(Error::Binding)
    ));
    assert!(matches!(
        keys.group_sender(0x7101, "1/1/2".parse().unwrap()),
        Err(Error::Binding)
    ));
    assert!(matches!(
        keys.handshake(0x7101, 0x7101, [1; 6]),
        Err(Error::Binding)
    ));
}
#[test]
fn bad_signature_content_and_password_release_no_binding() {
    for changed in [
        XML.replace("Example", "Altered"),
        XML.replace("7.1.12", "7.1.11"),
        XML.replace("l4LTA", "g4LTA"),
    ] {
        assert!(matches!(
            Keyring::open(&changed, "example"),
            Err(Error::Authentication)
        ));
    }
    assert!(matches!(
        Keyring::open(XML, "incorrect"),
        Err(Error::Authentication)
    ));
    assert!(matches!(Keyring::open(XML, ""), Err(Error::Authentication)));
}
#[test]
fn xml_and_work_budgets_reject_unsupported_input() {
    for invalid in [
        format!("<!DOCTYPE Keyring>{XML}"),
        XML.replace("http://knx.org/xml/keyring/1", "urn:other"),
        XML.replace("<GroupAddresses>", "<GroupAddresses>unverified-text"),
        XML.replace("Key=\"", "xmlns=\"urn:other\" Key=\""),
    ] {
        assert!(matches!(
            Keyring::open(&invalid, "example"),
            Err(Error::Format)
        ));
    }
    let deep = XML.replace(
        "<GroupAddresses>",
        &format!("{}{}<GroupAddresses>", "<x>".repeat(34), "</x>".repeat(34)),
    );
    assert!(matches!(Keyring::open(&deep, "example"), Err(Error::Limit)));
    assert!(matches!(
        Keyring::open(&"a".repeat(MAX_XML_BYTES + 1), "example"),
        Err(Error::Limit)
    ));
    assert!(matches!(
        Keyring::open(&XML.replace("Example", &"x".repeat(256)), "example"),
        Err(Error::Limit)
    ));
}

#[test]
fn independently_signed_ambiguous_group_keys_are_rejected() {
    let entry = r#"<Group Address="2305" Key="l4LTAfYHbBg7/B+d6xlsLw=="/>"#;
    let xml = XML
        .replace(entry, &entry.repeat(2))
        .replace("2DP5KJefsycC48FiZ47fvw==", "lHaeZUDWucUrn5pUjHaBww==");
    assert!(matches!(
        Keyring::open(&xml, "example"),
        Err(Error::Binding)
    ));
}
