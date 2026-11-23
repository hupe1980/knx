+++
title = "Rust library guide"
description = "Encode and decode KNX control commands, scenes and measurements, match group addresses and validate timestamped consumption readings."
template = "guide.html"
+++

## Add the library

With Rust 1.89 or newer, run `cargo add knx --path ../knx` in your application. Adjust the path to the checkout. The default crate has no external dependencies.

## Decode signed watts

```rust
assert_eq!(knx::decode_power_watts(&[0xc6, 0xbb, 0x80, 0]), Ok(-24_000.0));
```

DPT 14.056 carries finite signed watts. The commissioned meter convention determines the meaning of a negative value. Run `cargo run --locked --example decode_power` for a standalone example.

## Decode a reading

Bind a `PowerMeter` to one site, group address and nonnegative consumption range. Supply trusted observation and reception times in Unix seconds.

```rust
use knx::{MeterError, PowerMeter};

fn main() -> Result<(), MeterError> {
    let mut meter = PowerMeter::new("west", "2/1/3", 60_000.0)?;
    let reading = meter.observe("west", "2/1/3", "14.056", &[0x46, 0xbb, 0x80, 0],
                                Some(1_795_335_135), 1_795_335_136)?;
    assert_eq!(reading.watts, 24_000.0);
    assert_eq!(reading.observed_at, 1_795_335_135);
    Ok(())
}
```

Keep the meter across reconnects. A duplicate or older timestamp returns `MeterError::OutOfOrder` and leaves the last accepted reading intact. Your application decides how long that reading is fresh.

## Datapoint coverage

Read the datapoint type (DPT) assigned to the group address in ETS, the engineering tool used to commission the building. Pass only the application bytes to the decoder. For a switch, supply its extracted bit as one byte containing 0 or 1. Fixed-width encoders return byte arrays. Variable-length codecs use caller-provided storage.

<div class="codec-coverage" role="region" aria-label="Datapoint codec coverage" tabindex="0">

| DPT | Application value | Payload example | Decode | Encode |
| --- | --- | --- | --- | --- |
| 1.001 Switch | One bit, off or on | Extracted bit `1` → on | `decode_switch` | `encode_switch` |
| 3.007 Dimming | Relative brightness change | `0B` → increase by 1/4 of full range | `decode_dimming` | `encode_dimming` |
| 3.008 Blinds | Relative blind or slat movement | `03` → up by 1/4 of full range | `decode_blind_control` | `encode_blind_control` |
| 5.001 Scaling | Unsigned byte scaled to 0–100% | `FF` → 100% | `decode_scaling_percent` | `encode_scaling_percent` |
| 5.003 Angle | 0–360 degrees | `FF` → 360° | `decode_angle_degrees` | `encode_angle_degrees` |
| 5.004 Whole percent | Unsigned byte in percentage points | `FF` → 255% | `decode_percent_u8` | `encode_percent_u8` |
| 5.005 Decimal factor | Integer ratio | `0A` → 10 | `decode_decimal_factor` | `encode_decimal_factor` |
| 5.006 Tariff | Unavailable or supplier number 1–254 | `00` → unavailable | `Tariff::decode` | `tariff.encode()` |
| 5.010 Pulse count | Whole pulses, 0–255 | `2A` → 42 pulses | `decode_pulse_count` | `encode_pulse_count` |
| 6.001 Signed percent | Signed whole percent | `80` → −128% | `decode_percent_i8` | `encode_percent_i8` |
| 6.010 Signed count | Signed counter pulses | `FF` → −1 pulse | `decode_signed_pulse_count` | `encode_signed_pulse_count` |
| 6.020 Status with mode | Five set flags and an active mode | `52` → A, C and E set, mode 1 | `StatusWithMode::decode` | `status.encode()` |
| 7.002–7.007 Time period | Unsigned interval | 7.005 `00 02` → 2000 ms | `dpt.decode_ms` | `dpt.encode_ms` |
| 8.002–8.007 Delta time | Signed interval | 8.005 `FF FE` → −2000 ms | `dpt.decode_ms` | `dpt.encode_ms` |
| 9.001 Temperature | KNX two-byte float in °C | `07 D0` → 20 °C; `7F FF` denotes invalid data | `decode_temperature_celsius` | `encode_temperature_celsius` |
| 13.010 Active energy | Signed 32-bit integer in Wh | `FF FF FF FF` → −1 Wh | `decode_energy_wh` | `encode_energy_wh` |
| 14.056 Power | Four-byte float in W | `C6 BB 80 00` → −24,000 W | `decode_power_watts` | `encode_power_watts` |
| 17.001 Scene number | ETS display number 1–64 | `3F` → scene 64 | `decode_scene_number` | `encode_scene_number` |
| 18.001 Scene control | Activate or learn a scene | `BF` → learn scene 64 | `decode_scene_control` | `encode_scene_control` |

</div>

Search the [subtype inventory](@/inventory/_index.md) for a specific ID. Use `knx::domain` for DALI, blinds and weather, `knx::hvac_reports` for heating and cooling reports, and `knx::parameters` for commissioning parameters. Run `cargo doc --all-features --no-deps --open` to browse the Rust types and methods.

**Scaling and angles.** Both use the nearest byte, with half steps rounded up. On 5.001, 50% encodes as `80` and decodes to about 50.196%. On 5.004, 50% encodes exactly as `32`. On 5.003, 180° encodes as `80`.

**Tariffs.** Numbers 1–254 are supplier-defined. `00` means unavailable. Discard `FF` and retain the last valid observation.

**Temperature.** The encoder accepts −273 to 670433.28 °C and chooses the finest exponent that fits. Halfway mantissas round away from zero. The minimum input decodes to −272.96 °C. The wire value `7F FF` indicates invalid data.


## Binary functions and scalar measurements

Select a family by its commissioned ID. A door contact (1.019) and an open/close command (1.009) assign opposite meanings to the same bit. `BinaryDpt::labels()` returns the zero and one labels in wire order. Both values of 1.017 decode as `trigger`.

```rust
let wind = knx::Float16Dpt::from_dpt("9.005").unwrap();
assert_eq!(wind.decode(&[0x04, 0xe2]), Ok(12.5));
assert_eq!(wind.unit(), "m/s");
```

`Float16Dpt` handles the KNX two-byte float. The exponent determines the precision. The encoder rounds to the nearest valid mantissa at the finest fitting exponent, with halfway ties away from zero. `7F FF` signals invalid data. `Float32Dpt` handles finite IEEE binary32 values. Check the unit when binding a point. DPT 9.024 is kW and 14.056 is W.

Integer codecs preserve counts exactly. `decode_percent_hundredths` (8.010) returns hundredths of a percentage point and rejects `7F FF`. `decode_flow_rate_ticks` (13.002) returns increments of 0.0001 m³/h. `decode_supply_ma` (7.012) returns `None` when the device has no bus power supply functionality. Check the device profile before using parameter types such as 7.013 or 8.012 on the bus. Operating-hour datapoints 12.100–12.102 require a companion 13.100 signed seconds point.

## Time periods and deltas

Select `TimeIntervalDpt::from_dpt` using the exact ID. A period has 0–65,535 ticks. A delta has −32,768–32,767 ticks. The API takes and returns integer milliseconds. Use `resolution_ms()` and `range_ms()` to validate a user-entered interval.

| Period | Delta | Duration of one tick |
| --- | --- | --- |
| 7.002 | 8.002 | 1 ms |
| 7.003 | 8.003 | 10 ms |
| 7.004 | 8.004 | 100 ms |
| 7.005 | 8.005 | 1 s |
| 7.006 | 8.006 | 1 min |
| 7.007 | 8.007 | 1 h |

KNX restricts the 10 ms, 100 ms and minute subtypes to parameters and diagnostics unless a functional-block specification defines their use. [KNX Datapoint Types, §§3.8.2 and 3.9.2](https://support.knx.org/hc/en-us/article_attachments/15392631105682)

The CLI accepts a whole number followed by `ms`, `s`, `min` or `h`. For example, `knx encode 7.003 20 ms` yields `00 02`. An input of 15 ms for that subtype returns an error because the tick is 10 ms. Decode prints the exact millisecond value, including the sign.

## Status flags and mode

For DPT 6.020, `set_flags` lists A through E. A true value means that flag is set. The codec translates the active-low wire bits and checks that exactly one mode is selected. The device profile defines the meaning of each flag.

Use `knx encode 6.020 ace 1` to produce `52`. Supply each set flag once, or `none` if all flags are clear, followed by mode 0, 1 or 2. Decode prints flag letters in alphabetical order. [KNX Datapoint Types, §3.7](https://support.knx.org/hc/en-us/article_attachments/15392631105682)

## Priority controls

A DPT 2 override has three meanings. For a switch (2.001), `02` forces off, `03` forces on, and either `00` or `01` releases the override. The device then follows its configured behavior. Read its feedback point to observe the resulting state.

Select the commissioned subtype with `PriorityDpt::from_dpt`. Its `decode` method returns `PriorityControl::Release` or `PriorityControl::Set(bool)`. Supply the extracted two bits right-aligned in one byte. Encoding release produces `00`.

Use `knx encode 2.001 release` to produce a release payload. For a controlled value, use the corresponding CLI token below. [KNX Datapoint Types, §3.2](https://support.knx.org/hc/en-us/article_attachments/15392631105682)

| DPT | Function zero | Function one |
| --- | --- | --- |
| 2.001 | `off` | `on` |
| 2.002 | `false` | `true` |
| 2.003 | `disable` | `enable` |
| 2.004 | `no-ramp` | `ramp` |
| 2.005 | `no-alarm` | `alarm` |
| 2.006 | `low` | `high` |
| 2.007 | `decrease` | `increase` |
| 2.008 | `up` | `down` |
| 2.009 | `open` | `close` |
| 2.010 | `stop` | `start` |
| 2.011 | `inactive` | `active` |
| 2.012 | `normal` | `inverted` |

## Electrical energy registers

Select the DPT from the commissioned point. Each count represents one whole unit in the table. The signed value preserves the meter's direction convention. The Rust methods `encode(count)` and `decode(payload)` operate directly on integers.

| DPT | Unit | Rust subtype |
| --- | --- | --- |
| 13.010 | Wh | `Energy32::ActiveWh` |
| 13.011 | VAh | `Energy32::ApparentVah` |
| 13.012 | VARh | `Energy32::ReactiveVarh` |
| 13.013 | kWh | `Energy32::ActiveKwh` |
| 13.014 | kVAh | `Energy32::ApparentKvah` |
| 13.015 | kVARh | `Energy32::ReactiveKvarh` |
| 13.016 | MWh | `Energy32::ActiveMwh` |
| 29.010 | Wh | `Energy64::ActiveWh` |
| 29.011 | VAh | `Energy64::ApparentVah` |
| 29.012 | VARh | `Energy64::ReactiveVarh` |

For example, `knx decode 29.010 0020000000000001` returns `9007199254740993 Wh`. Keep the result as an integer through storage and export to preserve precision.

A device exposing a 64-bit energy point must also implement a 32-bit point in the same unit and resolution. Use `Energy64::companion()` to identify the corresponding subtype when checking a device profile. The host supplies each point’s group address. [KNX Datapoint Types, §3.28.1](https://support.knx.org/hc/en-us/article_attachments/15392631105682)

## Relative commands and scene numbers

For DPT 3.007 and 3.008, supply the four application bits in the low nibble of a single byte. Both `00` and `08` stop movement. Encoding stop produces `00`. A step is a fraction of the full range. `StepSize::from_denominator(4)` means one quarter.

The CLI accepts `increase` or `decrease` for dimming, and `up` or `down` for blinds. Follow the direction with one of `1/1`, `1/2`, `1/4`, `1/8`, `1/16`, `1/32` or `1/64`. Use `stop` alone.

For DPT 17.001, `knx encode 17.001 64` yields `3F`. For DPT 18.001, use `activate 64` to recall the scene or `learn 64` to store current settings. The latter encodes as `BF`. Both operations only produce application bytes in the CLI.

## Match ETS and transport addresses

Use `GroupAddress` as the key in a routing table. Decimal `4355`, two-level `2/259` and three-level `2/1/3` compare equal and display as `2/1/3`. Within a multi-site application, include the site in the key. The same address can belong to different installations.

```rust
use knx::GroupAddress;

fn main() -> Result<(), knx::InvalidGroupAddress> {
    let configured: GroupAddress = "2/259".parse()?;
    let received = GroupAddress::try_from(4355u16)?;
    assert_eq!(configured, received);
    assert_eq!(configured.to_string(), "2/1/3");
    Ok(())
}
```

Pass a parsed address or a raw `u16` directly to `PowerMeter::observe`. The library rejects zero, out-of-range components, leading zeros and extra whitespace.

## Check delivery protection

Pass your verifier’s per-message results to `DeliveryPolicy::check(ip, group)` before consuming a reading. IP tunnel protection and group Data Secure protection have separate requirements. Use `AuthenticatedFresh` only after the verifier checks the sender, integrity and freshness for that delivery. Failed verification or unavailable sequence state maps to `Unverified` and always rejects delivery. With the `data-secure` feature, a verified group result supplies its own group-layer evidence. A verified `ip-secure` TCP frame supplies its own IP-layer evidence. The host owns key provisioning and durable group sender sequences.

## Read or update a local interface property

Use `PropertyRead::new(object_type, instance, property, start, count, element_width)` with the device profile’s element width. Send `request()` through the local cEMI management connection and pass the same peer’s response to `confirm()`. The returned slice contains the full requested range. Match `AccessDenied` separately from `UnsupportedProperty`. The BAOS device-object serial number uses type 0, instance 1, property 11 and one six-byte element at index 1.

Use `InterfaceObjectList::default()` to discover local interface objects. Send each `next_request()` and pass its confirmation to `accept()`. After the last page, `object_types()` returns types in device order and `read_property()` resolves a zero-based object index to its type and instance. Try `cargo run --locked --example interface_objects` offline.

For a readable setting, construct `PropertyUpdate` with the target and desired bytes. Call `read_request` and pass its reply to `confirm_read`. If the result is `Different`, authorize the exact target and value in `write_request`. After a write confirmation or lost reply, read the same device again. Only `Matches` confirms the desired value. Serialize exchanges on the authenticated management connection and discard stale replies when reconnecting.

For remote property metadata, construct `PropertyDescriptionRead::by_id` or `by_index` and send its APDU on the selected individual connection. Pass the peer address with its response. The returned write flag and access levels describe the device policy. Local writes still require host authorization.

## Text and local time

Select `text_time::Charset` from the commissioned ASCII or Latin-1 subtype. `decode_utf8` returns borrowed text after validating the NUL terminator and UTF-8 bytes. `encode_utf8` writes the text and terminator into your buffer.

`decode_date_time` distinguishes a source clock fault from a partial local date. A missing year or time stays absent, and 24:00:00 stays end-of-day. The application supplies any timezone conversion.

## Tariffs, operating modes and lighting status

`TariffEnergy::decode` (235.001) returns signed Wh and the associated tariff. Check each field separately. For 269.1200, `DatedTariffEnergy` also exposes the local timestamp and its quality. A clock fault can accompany a valid register.

`HvacMode` (20.102) represents the five room operating commands. `LightingStatus` (207.600) reports level validity, failure and manual override independently. Select `LightingActuator::Switch` for a binary actuator or `Dimmer` for a dimmable output. [Subtype lookup](@/inventory/_index.md)

## Status-bearing measurements

For an LTE property response or info report, use `structured::LteDpt::decode_report`. The result distinguishes a measured value, override, fault and out-of-service point. DPT 205.100 uses 0.02 °C per count. A fault value of 1 means open circuit. If the override flag is also set, the main field holds the override value instead. For a property write, use `decode_command` to interpret the same final octet as a command. The commissioned application service determines which interpretation applies.

For DPT 229.001 use `structured::MeteringValue::decode_report` to keep the counter, its decimal scale and the status together. Use `structured::DatedMeasurement` for a DPT 266 timestamped electrical reading. Its KNX timestamp retains clock faults and missing fields.

## Use an ETS keyring

Enable the `keyring` Cargo feature and open a password-protected `.knxkeys` export with `keyring::Keyring::open(xml, password)`. The importer authenticates the complete export before accepting group bindings or tunnel credentials.

- `group_receiver(local_interface, sender, group)` requires a sender listed for that interface and group.
- `group_sender(local_interface, group)` uses the group's commissioned key and local source.
- `handshake(gateway, tunnel_slot, client_serial)` starts a mutually authenticated TCP session with the selected slot’s device and user credentials.

Individual addresses are wire-format `u16` values. For example, 7.1.12 is `0x710c`. Continue using the installation’s existing durable sequence stores after a reimport or key rotation. The importer accepts UTF-8 v1 exports up to 8 MiB and printable ASCII tunnel credentials. [Calimero keyring reference](https://github.com/calimero-project/calimero-core/blob/68c99bffb5d065220410311fbdf8f372fd84555e/src/io/calimero/secure/Keyring.java)
