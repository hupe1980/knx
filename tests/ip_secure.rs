#![cfg(feature = "ip-secure")]
use knx::ip_secure::{Error, Handshake};

#[test]
fn public_handshake_uses_fresh_os_keys_and_only_admits_commissioned_users() {
    let start = |user| Handshake::start([0, 0, 0, 0, 0, 1], user, [0x12; 16], [0x34; 16]);
    for user in [0, 128, 255] {
        assert!(matches!(start(user), Err(Error::Credentials)));
    }
    let first = start(1).unwrap();
    let second = start(127).unwrap();
    assert_ne!(&first.request()[14..], &second.request()[14..]);
    assert_eq!(
        &first.request()[..14],
        &[6, 16, 9, 0x51, 0, 46, 8, 2, 0, 0, 0, 0, 0, 0]
    );
    assert!(matches!(
        first.authenticate(&[6, 16, 9, 0x54, 0, 8, 0, 0]),
        Err(Error::Frame)
    ));
}
