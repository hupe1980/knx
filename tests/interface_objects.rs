use knx::{InterfaceObjectList, PropertyReadError};

/// A synthetic local device. Interpret each request independently from the client implementation.
fn respond(objects: &[u16], request: [u8; 7]) -> Vec<u8> {
    assert_eq!(&request[..5], &[0xfc, 0, 0, 1, 71]);
    let count = usize::from(request[5] >> 4);
    let start = usize::from(request[5] & 15) * 256 + usize::from(request[6]);
    let mut response = request.to_vec();
    response[0] = 0xfb;
    if start == 0 {
        assert_eq!(count, 1);
        response.extend_from_slice(&(objects.len() as u16).to_be_bytes());
    } else {
        assert!((1..=15).contains(&count));
        for value in &objects[start - 1..start - 1 + count] {
            response.extend_from_slice(&value.to_be_bytes());
        }
    }
    response
}

#[test]
fn repeated_types_and_final_short_page_resolve_zero_based_indices() {
    let types = [0, 6, 8, 6, 0x1234];
    let mut list = InterfaceObjectList::new(2).unwrap();
    assert_eq!(
        list.read_property(0, 1, 1, 1, 2),
        Err(PropertyReadError::ListIncomplete)
    );
    let mut observed = Vec::new();
    while let Some(read) = list.next_request() {
        assert!(list.object_types().is_none());
        let request = read.request();
        observed.push([request[5], request[6]]);
        list.accept(&respond(&types, request)).unwrap();
    }
    assert_eq!(observed, [[0x10, 0], [0x20, 1], [0x20, 3], [0x10, 5]]);
    assert_eq!(list.object_types(), Some(types.as_slice()));
    assert_eq!(
        list.read_property(3, 1, 1, 1, 2).unwrap().request(),
        [0xfc, 0, 6, 2, 1, 0x10, 1]
    );
    assert_eq!(
        list.read_property(4, 1, 1, 1, 2).unwrap().request(),
        [0xfc, 0x12, 0x34, 1, 1, 0x10, 1]
    );
    assert_eq!(
        list.read_property(5, 1, 1, 1, 2),
        Err(PropertyReadError::InvalidRequest)
    );
    assert_eq!(list.accept(&[]), Err(PropertyReadError::OtherRequest));
}

#[test]
fn partial_mismatched_and_negative_replies_never_advance_or_publish_a_list() {
    let mut list = InterfaceObjectList::new(2).unwrap();
    let initial = list.next_request();
    for (frame, error) in [
        (
            &[0xfb, 0, 0, 1, 71, 0, 0, 11][..],
            PropertyReadError::AccessDenied,
        ),
        (
            &[0xfb, 0, 0, 1, 71, 0, 0, 7],
            PropertyReadError::UnsupportedProperty,
        ),
        (
            &[0xfb, 0, 0, 1, 71, 0x10, 0, 2],
            PropertyReadError::InvalidConfirmation,
        ),
        (
            &[0xfb, 0, 0, 1, 71, 0x10, 0, 0x10, 0],
            PropertyReadError::ListTooLong,
        ),
    ] {
        assert_eq!(list.accept(frame), Err(error));
        assert_eq!(list.next_request(), initial);
    }
    let objects = [0, 4, 7, 4];
    let initial = initial.unwrap().request();
    list.accept(&respond(&objects, initial)).unwrap();
    let page = list.next_request().unwrap();
    let complete = respond(&objects, page.request());
    for cut in 0..complete.len() {
        assert!(list.accept(&complete[..cut]).is_err());
        assert_eq!(list.next_request(), Some(page));
        assert!(list.object_types().is_none());
    }
    let mut short = complete.clone();
    short[5] = 0x10;
    short.truncate(9);
    assert_eq!(
        list.accept(&short),
        Err(PropertyReadError::InvalidConfirmation)
    );
    assert_eq!(
        list.accept(&respond(&objects, initial)),
        Err(PropertyReadError::OtherRequest)
    );
    list.accept(&complete).unwrap();
    let last = list.next_request().unwrap();
    assert_eq!(list.accept(&complete), Err(PropertyReadError::OtherRequest));
    assert_eq!(
        list.accept(&[0xfb, 0, 0, 1, 71, 0, 3, 11]),
        Err(PropertyReadError::AccessDenied)
    );
    assert_eq!(list.next_request(), Some(last));
    assert!(list.object_types().is_none());
    list.accept(&respond(&objects, last.request())).unwrap();
    assert_eq!(list.object_types(), Some(objects.as_slice()));
}

#[test]
fn count_and_instance_boundaries_are_independent() {
    assert!(InterfaceObjectList::new(0).is_err());
    assert!(InterfaceObjectList::new(16).is_err());
    let mut empty = InterfaceObjectList::default();
    empty.accept(&[0xfb, 0, 0, 1, 71, 0x10, 0, 0, 0]).unwrap();
    assert_eq!(empty.object_types(), Some(&[][..]));
    assert!(empty.next_request().is_none());
    let mut list = InterfaceObjectList::default();
    let types = vec![0xff01; 4095];
    let mut reads = 0;
    while let Some(r) = list.next_request() {
        list.accept(&respond(&types, r.request())).unwrap();
        reads += 1;
    }
    assert_eq!(reads, 274);
    assert_eq!(list.object_types(), Some(types.as_slice()));
    assert_eq!(
        list.read_property(254, 1, 1, 1, 2).unwrap().request(),
        [0xfc, 255, 1, 255, 1, 0x10, 1]
    );
    assert_eq!(
        list.read_property(255, 1, 1, 1, 2),
        Err(PropertyReadError::InstanceOutOfRange)
    );
}
