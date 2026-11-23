# Changelog

## 0.1.1 (2026-11-23)

- Updated the README and guides with crates.io installation commands and a link to the Rust API reference.
- Configured tag-based releases through crates.io Trusted Publishing.

## 0.1.0 (2026-11-23)

- Decode and encode KNX datapoints in Rust with zero default dependencies. Bind commissioned addresses to meters, lights, temperature sensors and sunblinds.
- Exchange cEMI group telegrams over KNXnet/IP tunnelling or routing. Inspect bus-monitor captures, hop counts and coupler filter entries.
- Enable Data Secure for encrypted group traffic and sequence synchronization, or IP Secure for authenticated TCP sessions and multicast routing. Import credentials from an ETS keyring.
- Read interface properties, verify authorized updates and read commissioned KNX IoT switch properties through a host-provided CoAP transport.
- Use the optional CLI to inspect payloads, preview Home Assistant light mappings, record a handover or replay synthetic meter readings.
