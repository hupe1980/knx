use knx::*;

#[test]
fn both_scopes_need_independent_evidence() {
    use ProtectionEvidence::*;
    use ProtectionRequirement::*;
    for ip in [PlainAllowed, SecureRequired] {
        for group in [PlainAllowed, SecureRequired] {
            for ip_evidence in [Plain, AuthenticatedFresh, Unverified] {
                for group_evidence in [Plain, AuthenticatedFresh, Unverified] {
                    let expected = if ip_evidence == Unverified
                        || (ip == SecureRequired && ip_evidence == Plain)
                    {
                        Err(ProtectionError::Ip)
                    } else if group_evidence == Unverified
                        || (group == SecureRequired && group_evidence == Plain)
                    {
                        Err(ProtectionError::Group)
                    } else {
                        Ok(())
                    };
                    assert_eq!(
                        DeliveryPolicy { ip, group }.check(ip_evidence, group_evidence),
                        expected
                    );
                }
            }
        }
    }
}

#[test]
fn read_weinzierl_serial_number_and_reject_mismatches() {
    let read = PropertyRead::new(0, 1, 11, 1, 1, 6).unwrap();
    let confirmation = [0xfb, 0, 0, 1, 11, 0x10, 1, 0x00, 0xc5, 0, 0, 0, 0];
    assert_eq!(read.request(), [0xfc, 0, 0, 1, 11, 0x10, 1]);
    assert_eq!(read.confirm(&confirmation), Ok(&confirmation[7..]));
    for length in 0..confirmation.len() {
        assert!(read.confirm(&confirmation[..length]).is_err());
    }
    for field in 0..7 {
        let mut wrong = confirmation;
        wrong[field] ^= 1;
        assert!(read.confirm(&wrong).is_err());
    }
    let mut too_long = confirmation.to_vec();
    too_long.push(0);
    assert_eq!(
        read.confirm(&too_long),
        Err(PropertyReadError::InvalidConfirmation)
    );
}

#[test]
fn explicit_negative_results_and_index_boundaries() {
    use PropertyReadError::*;
    let read = PropertyRead::new(0x1234, 2, 3, 0xabc, 2, 1).unwrap();
    assert_eq!(read.request(), [0xfc, 0x12, 0x34, 2, 3, 0x2a, 0xbc]);
    for (code, err) in [
        (7, UnsupportedProperty),
        (11, AccessDenied),
        (9, DeviceError(9)),
    ] {
        assert_eq!(
            read.confirm(&[0xfb, 0x12, 0x34, 2, 3, 0x0a, 0xbc, code]),
            Err(err)
        );
    }
    assert_eq!(
        read.confirm(&[0xfb, 0x12, 0x34, 2, 3, 0x1a, 0xbc, 42]),
        Err(InvalidConfirmation)
    );
    assert!(PropertyRead::new(1, 1, 1, 0xfff, 1, 1).is_ok());
    assert!(PropertyRead::new(1, 1, 1, 0, 1, 2).is_ok());
    for (inst, start, count, width) in [
        (0, 1, 1, 1),
        (1, 0x1000, 1, 1),
        (1, 0xfff, 2, 1),
        (1, 1, 0, 1),
        (1, 1, 16, 1),
        (1, 1, 1, 0),
        (1, 0, 2, 2),
        (1, 0, 1, 1),
    ] {
        assert_eq!(
            PropertyRead::new(0, inst, 1, start, count, width),
            Err(InvalidRequest)
        );
    }
}
