use knx::{CodecError, decode_power_watts};

fn main() -> Result<(), CodecError> {
    let watts = decode_power_watts(&[0xc6, 0xbb, 0x80, 0x00])?;
    println!("{watts} W");
    Ok(())
}
