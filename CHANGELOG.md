# Changelog

## Unreleased

- Added offline cEMI bus-monitor diagnostics with capture loss, link errors and 16- or 32-bit relative timestamps.
- Added coupler group-filter bitmap inspection and per-medium hop counters. Enforced the PL110 group range and extended cEMI length on RF indications.
- Added password-authenticated ETS keyring import for normal group keys and unicast tunnel credentials.
- Added a Rust library with zero default dependencies for KNX group addresses and application payloads, including binary controls, scalar measurements, calendar values and compound building datapoints.
- Added signed power decoding and a site-bound consumption meter that validates the source, range and observation time.
- Added standard and extended cEMI group services (up to 253 application data octets), individual connected transport, filtered Core v2 and legacy UDP/IPv4 discovery, link-layer tunnel handling and paced routing sends and multicast reception.
- Added local interface-object enumeration, remote property descriptions, property reads and authorized updates with readback, per-layer protection policy checks and commissioned Point API switch reads through a host-owned CoAP transport, with plain and OSCORE software-peer tests.
- Added opt-in KNX IP Secure TCP sessions and paced authenticated multicast transmission and reception with authenticated Timer Notify synchronization and durable group time.
- Added authenticated and encrypted group transmission and reception and normal-mode individual sequence synchronization behind `data-secure`, PL110 domain checks, and client bindings for switching, dimming and sunblind actuators and room/outside temperature sensors.
- Added a reproducible release evidence report linking each tested datapoint direction to a passing vector.
- Added an optional offline CLI for payload inspection, Home Assistant light mappings, handover reports and synthetic meter replay.
