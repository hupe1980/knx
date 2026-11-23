# knx

A Rust library for KNX datapoints, group telegrams and KNXnet/IP. Use the group address and datapoint type (DPT) assigned in ETS, the tool used to commission the installation. The default crate has no external dependencies.

## Use the library

With Rust 1.89 or newer, run `cargo add knx` in your application.

```rust
use knx::{GroupAddress, decode_temperature_celsius};

let room: GroupAddress = "2/1/3".parse().unwrap();
assert_eq!(u16::from(room), 4355);
assert_eq!(decode_temperature_celsius(&[0x07, 0xd0]), Ok(20.0)); // DPT 9.001
```

Run `cargo run --locked --example datapoints` from the checkout. [Rust API reference](https://docs.rs/knx/) · [Documentation](https://hupe1980.github.io/knx/documentation/) · [Search datapoints](https://hupe1980.github.io/knx/inventory/)

Use the transport APIs to discover gateways, exchange group telegrams, inspect bus-monitor captures and bind commissioned lights or sunblinds. Choose Cargo features according to the interface your application uses. [Transport guide](https://hupe1980.github.io/knx/transport/)

| Feature | Use it for |
| --- | --- |
| `data-secure` | Send and receive authenticated, encrypted group traffic and synchronize sender counters with a commissioned peer. |
| `keyring` | Import signed ETS group bindings and tunnel credentials for Data Secure and IP Secure. |
| `ip-secure` | Establish an authenticated unicast session over a caller-owned TCP stream. |
| `secure-routing` | Send and receive authenticated multicast routing records with an external clock or autonomous Timer Notify synchronization. |
| `linux-routing` | Read multicast destination, ingress interface and socket loss on Linux. |
| `cli` | Install the offline command-line tools. |

## Optional CLI

Install the CLI to inspect payloads and preview a Home Assistant light mapping offline.

```sh
cargo install knx --locked --features cli
knx decode 9.001 07 D0       # 20 °C
knx encode 8.005 -2 s        # FF FE
knx example | knx render -
```

For mapping, handover reports and exit codes, run `knx guide` or read the [field guide](https://hupe1980.github.io/knx/guide/). Run `knx replay` for the synthetic charging-meter example.

## Development

From the checkout, run `just` to list development commands. `just test` runs both Rust test suites. `just check` also checks formatting, Clippy, rustdoc, release-evidence tooling and the Zola site. Use `just serve` for a live site preview.

[Changelog](CHANGELOG.md) · [MIT license](LICENSE-MIT) · [Apache 2.0 license](LICENSE-APACHE)
