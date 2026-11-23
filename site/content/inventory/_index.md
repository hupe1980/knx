+++
title = "KNX datapoint support by subtype"
description = "Look up a KNX subtype, its source section and the encode and decode support in the Rust library."
template = "inventory.html"
+++

Filter by subtype ID, name or support status. For example, `FF` means 100% on <a href="#dpt-5.001">5.001 Scaling</a> and 255% on <a href="#dpt-5.004">5.004 Percent_U8</a>. [Rust codec reference](@/library/_index.md#datapoint-coverage)

**Tested** means the indicated direction has a passing test against fixed wire bytes. **Layout-tested** covers the encoded fields. For M-Bus addresses (230.1000), use `MBusAddress::manufacturer_flag()` for the three-letter manufacturer code and the DLMS UA registry for the company. Interpret the medium using EN 13757-3 and the KNX RF metering profile. For DPT 277–284.1200, resolve register units and selectors from the commissioned meter profile. **Unsupported** means the crate has no verified codec for that direction. The [CSV](../standards.csv) includes test names and parameter bindings. Parameter IDs use a separate hexadecimal namespace. Use Rust’s `parameters::ParameterType` for commissioning values. A layout-only entry checks the assigned bit fields. An unspecified format permits lookup only. Group encode and decode columns show N/A for all parameters.

## Edition and scope

The catalog covers 454 numbered datapoints in v02.02.01, including 249.600 from §6.16. Chapter 11 also lists 32 assigned parameter IDs and four unnumbered parameters. Historical formats from annexes A and B appear by name. Consult §1.2 for reserved ranges and manufacturer-specific assignments.

Run `cargo test --test inventory` to check codec claims. To reproduce the CSV, install Poppler and run `python3 tools/standards_inventory.py /path/to/edition.pdf` with the [edition PDF](https://support.knx.org/hc/en-us/article_attachments/15392631105682).

Run `just evidence` in the checkout for a machine-readable report by subtype and direction. With uv, Git and JDK 21 or newer installed, `just evidence --peers` also exercises independent software implementations on loopback. Device interoperability requires a trial on the commissioned installation.
