"""Emit release evidence after the crate's inventory tests pass (Python 3.9+)."""

import argparse
import csv
import json
import re
import subprocess
import sys
from pathlib import Path

from standards_inventory import CATALOG, DIGEST, EDITION

ROOT = Path(__file__).resolve().parents[1]


def cargo(*args):
    return subprocess.run(
        ["cargo", *args], cwd=ROOT, text=True, stdout=subprocess.PIPE, check=True
    ).stdout


def claims(rows, passed):
    entries = []
    seen = set()
    for row in rows:
        if row["kind"] in ("parameter", "unassigned-parameter"):
            if (
                row["edition"] != EDITION
                or row.get("parameter_validation")
                not in ("value", "layout", "unspecified")
                or any(
                    row[d] != "not-applicable" or row[f"{d}_evidence"]
                    for d in ("encode", "decode")
                )
            ):
                raise ValueError(
                    "parameter validation must be separate from group codec claims"
                )
            continue
        for direction in ("encode", "decode"):
            evidence = row[f"{direction}_evidence"]
            status = row[direction]
            if status == "unsupported" and not evidence:
                continue
            prefix = "tests/inventory.rs::"
            if (
                row["edition"] != EDITION
                or row["kind"] != "dpt"
                or status not in ("tested", "layout-tested")
                or not evidence.startswith(prefix)
                or evidence[len(prefix) :] not in passed
                or evidence in seen
            ):
                raise ValueError("inventory claim lacks a distinct passing vector")
            seen.add(evidence)
            entries.append(
                {
                    "dpt": row["id"],
                    "section": row["section"],
                    "direction": direction,
                    "result": "layout-tested"
                    if status == "layout-tested"
                    else "vector-tested",
                    "case": evidence,
                }
            )
    if not entries or "edition_and_support_are_traceable" not in passed:
        raise ValueError("missing inventory reconciliation or vector results")
    return entries


PEERS = {
    "plain_udp_tunnel": (
        "tunnel_peer.py",
        ["uv", "run", "--with", "xknx==4.1.1", "python", "-I"],
        "Scripted IPv4 peer using XKNX 4.1.1 frame codecs",
    ),
    "application_profiles": (
        "application_profiles_peer.py",
        ["uv", "run", "--with", "xknx==4.1.1", "python", "-I"],
        "XKNX 4.1.1 temperature sensors (FB 320/321), lighting actuators (417/418) and sunblind (800)",
    ),
    "group_data_secure": (
        "data_secure_peer.py",
        ["uv", "run", "--with", "xknx==4.1.1", "python", "-I"],
        "XKNX 4.1.1 bidirectional group A+C and key rotation",
    ),
    "point_api_switch": (
        "switch_peer.py",
        [
            "uv",
            "run",
            "--python",
            "3.13",
            "--with",
            "aiocoap[oscore]==0.4.17",
            "--with",
            "knxiotclient @ git+https://gitlab.knx.org/public-projects/knx-iot-point-api/knx-iot-point-api-configuration-client.git@47410f149eed02ea52b810e8b98e954f99362155",
            "python",
            "-I",
        ],
        "KNX Association reader, Rust oscore 0.1.2 and aiocoap, IPv6 plain and OSCORE JSON/CBOR reads and authenticated denial",
    ),
    "discovery_description_routing": (
        "calimero_loopback.py",
        [sys.executable, "-I"],
        "Pinned Calimero virtual gateway, mutually authenticated TCP tunnel with a group read and clock indication, legacy and extended discovery, description, bidirectional plain routing, secure multicast with external time and autonomous Timer Notify and individual normal-mode sequence synchronization",
    ),
}


def peer_results(run):
    results = {}
    for profile, (script, command, provenance) in PEERS.items():
        case = f"tools/interop/{script}"
        if run:
            subprocess.run([*command, case], cwd=ROOT, stdout=sys.stderr, check=True)
        results[profile] = {
            "result": "passed" if run else "not-run",
            "environment": "local pipe"
            if profile in {"group_data_secure", "application_profiles"}
            else "loopback",
            "peer": provenance,
            "case": case,
        }
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--peers",
        action="store_true",
        help="also run pinned loopback peers (requires uv, Git and JDK 21+)",
    )
    args = parser.parse_args()
    metadata = json.loads(
        cargo("metadata", "--locked", "--offline", "--no-deps", "--format-version", "1")
    )
    package = next(
        p
        for p in metadata["packages"]
        if Path(p["manifest_path"]) == ROOT / "Cargo.toml"
    )
    output = cargo(
        "test",
        "--locked",
        "--offline",
        "--test",
        "inventory",
        "--",
        "--format",
        "pretty",
        "--color",
        "never",
    )
    passed = set(re.findall(r"^test ([\w:]+) \.\.\. ok$", output, re.MULTILINE))
    with CATALOG.open(encoding="utf-8", newline="") as catalog:
        vectors = claims(csv.DictReader(catalog), passed)
    report = {
        "crate": package["name"],
        "version": package["version"],
        "profile": "Standalone KNX application payload codecs",
        "source": {
            "edition": EDITION,
            "pdf_sha256": DIGEST,
            "inventory": "site/static/standards.csv",
        },
        "fixture_provenance": "Local fixed application payload vectors in tests/inventory.rs",
        "directions": vectors,
        "software_interoperability": peer_results(args.peers),
        "limitations": {
            "other_directions": "See inventory for specified entries without verified codec support.",
            "physical_interoperability": "Unverified. Maintainer needs a named device, configuration and observations.",
            "certification": "Unverified. No certification evidence or official test-case mapping supplied.",
            "edition_updates": "Maintainer must reconcile subsequent supplements and errata separately.",
            "device_responsibilities": "The host owns its physical interface, commissioning and device profile verification.",
        },
    }
    json.dump(report, sys.stdout, indent=2)
    print()


if __name__ == "__main__":
    main()
