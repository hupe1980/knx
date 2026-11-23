use knx::{
    PropertyRead, PropertyReadError, PropertyUpdate, PropertyUpdateError, PropertyUpdateState::*,
};

// Weinzierl BAOS Starter Kit user's guide §§5.2.4–5.2.5, pp. 49–51.
const READ: [u8; 7] = [0xfc, 0, 0, 1, 54, 16, 1];
const WRITE: [u8; 8] = [0xf6, 0, 0, 1, 54, 16, 1, 1];
const CONFIRM: [u8; 7] = [0xf5, 0, 0, 1, 54, 16, 1];
const BEFORE: [u8; 8] = [0xfb, 0, 0, 1, 54, 16, 1, 0];
const AFTER: [u8; 8] = [0xfb, 0, 0, 1, 54, 16, 1, 1];

fn setting() -> PropertyRead {
    PropertyRead::new(0, 1, 54, 1, 1, 1).unwrap()
}

#[test]
fn vendor_transaction_distinguishes_write_confirmation_and_readback() {
    let mut change = PropertyUpdate::new(setting(), &[1]).unwrap();
    assert_eq!(
        change.write_request(|_, _| panic!()),
        Err(PropertyUpdateError::UnexpectedState)
    );
    assert_eq!(change.read_request(), READ);
    assert_eq!(change.confirm_read(&BEFORE), Ok(Different));
    let request = change
        .write_request(|target, value| target == setting() && value == [1])
        .unwrap();
    assert_eq!(request, WRITE);
    assert_eq!(change.state(), WriteUnconfirmed);
    assert_eq!(change.confirm_write(&CONFIRM), Ok(()));
    assert_eq!(change.state(), ReadRequired);
    assert_eq!(
        change.confirm_read(&AFTER),
        Err(PropertyUpdateError::UnexpectedState)
    );
    assert_eq!(change.read_request(), READ);
    assert_eq!(change.confirm_read(&AFTER), Ok(Matches));
    assert_eq!(
        change.write_request(|_, _| panic!()),
        Err(PropertyUpdateError::UnexpectedState)
    );
}

#[test]
fn interrupted_write_requires_fresh_read_and_fresh_authorization() {
    // Simulated device either applied the bytes before disconnect or never received them.
    for applied in [true, false] {
        let mut change = PropertyUpdate::new(setting(), &[1]).unwrap();
        change.read_request();
        change.confirm_read(&BEFORE).unwrap();
        assert_eq!(change.write_request(|_, _| true).unwrap(), WRITE);
        assert_eq!(
            change.write_request(|_, _| panic!()),
            Err(PropertyUpdateError::UnexpectedState)
        );
        change.read_request();
        assert!(change.confirm_write(&CONFIRM).is_err());
        assert!(change.confirm_read(&AFTER[..7]).is_err());
        assert_eq!(
            change.write_request(|_, _| panic!()),
            Err(PropertyUpdateError::UnexpectedState)
        );
        let observed = change
            .confirm_read(if applied { &AFTER } else { &BEFORE })
            .unwrap();
        if applied {
            assert_eq!(observed, Matches);
            assert_eq!(
                change.write_request(|_, _| panic!()),
                Err(PropertyUpdateError::UnexpectedState)
            );
        } else {
            assert_eq!(observed, Different);
            assert_eq!(
                change.write_request(|_, _| false),
                Err(PropertyUpdateError::AuthorizationDenied)
            );
            assert_eq!(
                change.write_request(|_, _| panic!()),
                Err(PropertyUpdateError::UnexpectedState)
            );
            change.read_request();
            change.confirm_read(&BEFORE).unwrap();
            assert_eq!(change.write_request(|_, _| true).unwrap(), WRITE);
        }
    }
}

#[test]
fn wrong_range_and_partial_confirmations_never_authorize_another_write() {
    let target = PropertyRead::new(0x1200, 3, 26, 0x381, 2, 2).unwrap();
    let desired = [0x41, 0x42, 0x43, 0x44];
    let mut update = PropertyUpdate::new(target, &desired).unwrap();
    assert!(matches!(
        PropertyUpdate::new(target, &[1]),
        Err(PropertyUpdateError::InvalidSetting)
    ));
    let count = PropertyRead::new(4, 1, 33, 0, 1, 2).unwrap();
    assert!(matches!(
        PropertyUpdate::new(count, &[0, 1]),
        Err(PropertyUpdateError::InvalidSetting)
    ));
    update.read_request();
    update
        .confirm_read(&[0xfb, 0x12, 0, 3, 26, 0x23, 0x81, 0, 0, 0, 0])
        .unwrap();
    assert_eq!(
        update.write_request(|_, _| true).unwrap(),
        [0xf6, 0x12, 0, 3, 26, 0x23, 0x81, 0x41, 0x42, 0x43, 0x44]
    );
    let valid = [0xf5, 0x12, 0, 3, 26, 0x23, 0x81];
    for n in 0..7 {
        assert!(update.confirm_write(&valid[..n]).is_err());
        let mut other = valid;
        other[n] ^= 1;
        assert!(update.confirm_write(&other).is_err());
    }
    let mut extra = valid.to_vec();
    extra.push(0);
    assert!(update.confirm_write(&extra).is_err());
    assert_eq!(update.state(), WriteUnconfirmed);
    let mut denied = extra;
    denied[5] = 3;
    denied[7] = 11;
    assert_eq!(
        update.confirm_write(&denied),
        Err(PropertyUpdateError::Property(
            PropertyReadError::AccessDenied
        ))
    );
    assert_eq!(update.state(), ReadRequired);
    assert_eq!(
        update.write_request(|_, _| panic!()),
        Err(PropertyUpdateError::UnexpectedState)
    );
}
