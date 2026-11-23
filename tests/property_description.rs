use knx::{DescriptionError, PropertyDescriptionRead};

#[test]
fn classic_read_by_pid_or_index_resolves_metadata() {
    let pid = 54.try_into().unwrap();
    let a = PropertyDescriptionRead::by_id(0x1101, 2, pid);
    let b = PropertyDescriptionRead::by_index(0x1101, 2, 7);
    assert_eq!(a.request(), [3, 0xd8, 2, 54, 0]);
    assert_eq!(b.request(), [3, 0xd8, 2, 0, 7]);
    // Encoded by Calimero Description.toByteArray and DataUnitBuilder.createAPDU.
    let response = [3, 0xd9, 2, 54, 7, 0x84, 0x0f, 0xff, 0x32];
    let d = a.response(0x1101, &response).unwrap();
    assert_eq!(b.response(0x1101, &response), Ok(d));
    assert_eq!(
        (d.object_index, d.property_id, d.property_index),
        (2, pid, 7)
    );
    assert_eq!(
        (d.data_type, d.write_enabled, d.max_elements),
        (4, true, 4095)
    );
    assert_eq!((d.read_level, d.write_level), (3, 2));
    let mut alternate_index = response;
    alternate_index[4] = 200;
    assert_eq!(
        a.response(0x1101, &alternate_index).unwrap().property_index,
        200
    );
    let mut alternate_pid = response;
    alternate_pid[3] = 68;
    assert_eq!(
        b.response(0x1101, &alternate_pid)
            .unwrap()
            .property_id
            .get(),
        68
    );
    assert_eq!(
        a.response(0x1101, &[3, 0xd9, 2, 54, 0, 0, 0, 0, 0]),
        Err(DescriptionError::Unavailable)
    );
    assert_eq!(
        a.response(0x1102, &response),
        Err(DescriptionError::Mismatch)
    );
    for (index, q) in [(2, a), (3, a), (4, b)] {
        let mut other = response;
        other[index] ^= 1;
        assert_eq!(q.response(0x1101, &other), Err(DescriptionError::Mismatch));
    }
}
#[test]
fn undefined_property_rejects_and_unknown_formats_preserve_the_boundary() {
    let q = PropertyDescriptionRead::by_index(9, 1, 2);
    let mut negative = [3, 0xd9, 1, 0, 2, 0, 0, 0, 0];
    assert_eq!(q.response(9, &negative), Err(DescriptionError::Unavailable));
    for end in 0..negative.len() {
        assert_eq!(
            q.response(9, &negative[..end]),
            Err(DescriptionError::UnsupportedFormat)
        );
    }
    negative[7] = 1;
    assert_eq!(
        q.response(9, &negative),
        Err(DescriptionError::UnsupportedFormat)
    );
    negative[3] = 1;
    for (index, value) in [(1, 0xd8), (5, 0x40), (6, 0x10)] {
        let mut bad = negative;
        bad[index] = value;
        assert_eq!(
            q.response(9, &bad),
            Err(DescriptionError::UnsupportedFormat)
        );
    }
}
