+++
title = "KNX Rust documentation"
description = "Start with a KNX temperature reading, then choose a guide for datapoint codecs, KNXnet/IP connections or Home Assistant light mappings."
template = "documentation.html"
+++

## Decode a temperature

A KNX group address identifies a function, such as a room temperature. The datapoint type (DPT) assigned in the ETS commissioning project gives its bytes a meaning. For DPT 9.001, the two bytes below represent degrees Celsius.

From a Rust 1.89 or newer application, add the library from a local checkout.

```sh
cargo add knx --path ../knx
```

```rust
use knx::decode_temperature_celsius;

assert_eq!(decode_temperature_celsius(&[0x07, 0xd0]), Ok(20.0));
```

Pass the extracted application payload to the decoder. If you have a complete telegram, start with the [transport guide](@/transport/_index.md#read-a-group-telegram).
