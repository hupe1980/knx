use knx::{MeterError, PowerMeter};

fn main() -> Result<(), MeterError> {
    let mut meter = PowerMeter::new("west", "2/1/3", 60_000.0)?;
    let reading = meter.observe(
        "west",
        "2/1/3",
        "14.056",
        &[0x46, 0xbb, 0x80, 0],
        Some(1_795_335_135),
        1_795_335_136,
    )?;
    println!("{} W at {}", reading.watts, reading.observed_at);
    Ok(())
}
