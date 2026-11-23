//! Examine a synthetic local management connection with two instances of object type 4.
use knx::{InterfaceObjectList, PropertyReadError};

fn main() -> Result<(), PropertyReadError> {
    let mut list = InterfaceObjectList::new(2)?;
    // Index zero is the count. Subsequent pages contain network-order u16 object types.
    let confirmations: &[&[u8]] = &[
        &[0xfb, 0, 0, 1, 71, 0x10, 0, 0, 4],
        &[0xfb, 0, 0, 1, 71, 0x20, 1, 0, 0, 0, 4],
        &[0xfb, 0, 0, 1, 71, 0x20, 3, 0, 8, 0, 4],
    ];
    for response in confirmations {
        let read = list.next_request().expect("pending page");
        println!("Local read {:02X?}", read.request());
        list.accept(response)?;
    }
    assert_eq!(list.object_types(), Some(&[0, 4, 8, 4][..]));
    // Read PID_OBJECT_TYPE on object index 3 (type 4, instance 2).
    let read = list.read_property(3, 1, 1, 1, 2)?;
    let response = [0xfb, 0, 4, 2, 1, 0x10, 1, 0, 4];
    assert_eq!(read.confirm(&response)?, [0, 4]);
    println!("Object 3 is type 4, instance 2.");
    Ok(())
}
