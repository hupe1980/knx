+++
title = "KNX light handover guide"
description = "Install the Rust CLI, map a switched light from ETS, choose the right Home Assistant YAML format, and interpret the installer’s handover report."
template = "guide.html"
+++

## Install and try the example

Install the CLI from crates.io and render the bundled kitchen example.

```sh
cargo install knx --locked --features cli
knx example | knx render -
```

The preview contains one Home Assistant light named `Kitchen ceiling`. The CLI works offline after installation. Run `knx guide` for this guide at the property.

## Decode application bytes

Use the DPT assigned in ETS and supply the extracted value in hex. Compact, quoted and separate byte pairs are accepted.

```sh
knx decode 9.001 07 D0       # 20 °C
knx decode 13.010 FFFFFFFF   # -1 Wh
knx decode 1.001 01          # on
```

For a switch, extract the right-aligned bit first. A successful decode prints one value and exits 0. Invalid input exits 2 with a diagnostic on stderr. For example, temperature word `7F FF` reports invalid data.

## Encode a reference value

Run `knx decode --help` for the CLI’s supported subtypes. The Rust library covers additional types. Use the arguments for the commissioned subtype. The CLI prints the extracted bytes as uppercase hex.

```sh
knx encode 5.001 50        # 80 (scaled percent)
knx encode 5.004 50        # 32 (whole percent)
knx encode 8.005 -2 s      # FF FE (signed interval)
```

Run `cargo run --locked --example datapoints` from the checkout for more codec examples.

## Map a light from ETS

ETS is the engineering tool used to commission a KNX installation. A group address identifies a shared function on the bus. For a switched light, the **command** address takes an on/off request. The actuator reports its switching state on the **feedback** address. Check the physical fixture during handover. This profile requires two distinct groups, both assigned switch datapoint type (DPT) **1.001**. If the installation shares one group for both roles, use Home Assistant’s native KNX light configuration. [Home Assistant KNX lights](https://www.home-assistant.io/integrations/knx/#light)

In an empty directory, run `knx example > room.yaml`. Replace the labels and the two group addresses with the values for your fixture.

```yaml
room: Kitchen
light: ceiling
command: {address: "1/1/1", dpt: "1.001"}
feedback: {address: "1/1/2", dpt: "1.001"}
```

Run `knx validate room.yaml`. Enter addresses in three-level, two-level or raw decimal form, without leading zeros. For example, `1/1/2`, `1/258` and `2306` identify the same group. Output for Home Assistant uses three-level addresses. For a DPT mismatch, check the group address in ETS. When its DPT is unassigned, inspect the linked group objects. [ETS Group Monitor](https://support.knx.org/hc/en-us/articles/37592800979346-Datapoint-Type-selection-in-the-Group-Monitor)

## Render Home Assistant YAML

Choose the command for the location you are editing. Each preview goes to stdout.

| Destination | Command | Output starts with |
| --- | --- | --- |
| Main configuration | `knx render room.yaml` | `knx:` |
| File included by `knx: !include knx.yaml` | `knx render --target knx room.yaml` | `light:` |
| Existing KNX light list | `knx render --target lights room.yaml` | A list entry |

When a KNX section already exists, merge the entry into its light list at the existing indentation. Check the combined configuration in Home Assistant before applying it. Home Assistant handles the KNX/IP connection and any keyring. [Splitting the configuration](https://www.home-assistant.io/docs/configuration/splitting_configuration/)

## Review a proposed edit

Copy the reviewed mapping to `proposed.yaml`, edit the copy, then run `knx diff room.yaml proposed.yaml`. An address change appears by role, for example `Feedback: 1/1/2 -> 1/1/3`. Check that actuator object in ETS and observe the fixture during handover.

Diff exits 0 for equivalent mappings, 1 for changes and 2 for invalid input. Either file can come from stdin using `-`.

## Check the physical fixture

In the commissioned room, start the interview with a new filename in a writable directory:

```sh
knx handover room.yaml --output kitchen-handover.json
```

1. Switch the light from the dashboard. Confirm which light changed and look for fresh actuator feedback.
2. Use the wall switch. Confirm the same light changed and that fresh feedback updated the resident’s display.
3. During an agreed maintenance window, check what the resident can tell when feedback is lost. After reconnecting, watch for current state and any repeated actuation.
4. Record the required and observed protection separately for the IP connection, command group and feedback group. A secure tunnel alone cannot establish group protection. [Home Assistant KNX Secure](https://www.home-assistant.io/integrations/knx/#knx-secure)

Press Enter for an unknown answer. The report records installer observations. It includes room identifiers, so keep it with the local handover paperwork.

## Read the result

| Result | Exit | Meaning |
| --- | --- | --- |
| `verified` | 0 | All observations passed and protection met the reported policy. |
| `inconclusive` | 1 | An observation or policy is unknown. Repeat the checks listed in `findings`. |
| `failed` | 1 | A physical check failed or observed protection fell short. Each finding includes a follow-up action. |

A failed check takes precedence over an unknown answer. Omitting `--output` sends the completed JSON to stdout. Interview prompts and findings go to stderr. Existing report files are preserved.

For scripts, `validate` and `render` accept a filename or `-` for stdin, up to 16 KiB of UTF-8 YAML. Both return 0 on success. A malformed mapping, file error or interrupted interview exits with 2. Handover requires a mapping file because it reads answers from stdin.

## Troubleshoot feedback

**The wrong light switches.** Compare the command group with the intended actuator channel in ETS. A valid address can belong to another room.

**The light switches but the dashboard stays unchanged.** Operate the wall switch and watch for a new telegram from the feedback object. Compare its address with the generated `state_address`. A quiet bus leaves the observation unknown.

**State is missing after startup.** Look for a timed-out group read in Home Assistant. In ETS, check the read flag on the actuator object that holds the current state. A later wall-switch update establishes only that a new event arrived. [Home Assistant troubleshooting](https://www.home-assistant.io/integrations/knx/#group-address-cannot-be-read)

## Replay a charging submeter

Run the synthetic incident from the terminal. All three commands emit JSON and use a fixed clock.

```sh
knx replay
knx replay --missing-final
knx replay --connection-age 31
```

The sample requests a 24 kW charging limit. The branch reports 28 kW after five seconds and 24 kW after fifteen. With `--missing-final`, delivery remains unconfirmed. The separate connection reading leaves 8 kW of headroom when fresh; its age limit is 30 seconds.

To edit the sample, run `knx example --charging > copy.json` in an empty directory, then `knx replay --fixture copy.json`. Fixtures accept up to 16 KiB from a file or stdin (`-`). Supply Unix timestamps and four network-order bytes for each DPT 14.056 reading.
