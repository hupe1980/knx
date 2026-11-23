"""Exchange authenticated group telegrams between XKNX and Rust over local pipes."""

import json
import subprocess
import tempfile
from pathlib import Path

from xknx.cemi.flags import CEMIAddressType, CEMIFrameFormat
from xknx.secure.data_secure_asdu import (
    SecureData,
    SecurityAlgorithmIdentifier,
    SecurityALService,
    SecurityControlField,
)
from xknx.telegram.tpci import TDataGroup

ROOT = Path(__file__).resolve().parents[2]


def control_field():
    return SecurityControlField(
        False,
        SecurityAlgorithmIdentifier.CCM_ENCRYPTION,
        False,
        SecurityALService.S_A_DATA,
    )


def indication(key, sequence):
    apdu = bytes([0, 0x80, *range(32)])
    scf = control_field()
    body = SecureData.init_from_plain_apdu(
        key,
        apdu,
        scf,
        sequence,
        bytes.fromhex("50010400"),
        CEMIAddressType.GROUP,
        CEMIFrameFormat.STANDARD,
        TDataGroup(),
    )
    return bytes.fromhex("29003ce0500104002e03f110") + body.to_knx()


def main():
    # Public synthetic keys. The host uses one sender high-water mark through rotation.
    a, b = bytes(range(16)), bytes(range(16, 32))
    frames = [
        indication(a, 21),
        indication(b, 21),
        indication(a, 22),
        indication(b, 22),
        indication(a, 21),
        indication(a, 23),
    ]
    with tempfile.TemporaryDirectory(prefix="knx-secure-peer-") as directory:
        app = Path(directory)
        (app / "Cargo.toml").write_text(
            '[package]\nname="group-peer"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nknx={path='
            + json.dumps(str(ROOT))
            + ',features=["data-secure"]}\n[[bin]]\nname="group-peer"\npath="main.rs"\n'
        )
        (app / "main.rs").write_bytes(
            (ROOT / "tools/interop/data_secure_client.rs").read_bytes()
        )
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--offline"],
            cwd=app,
            input=b"".join(frames),
            stdout=subprocess.PIPE,
            check=True,
        )

    output = result.stdout
    for seq, key, apdu in [
        (31, a, bytes([0, 0])),
        (32, a, bytes([0, 0x81])),
        (33, b, bytes([0, 0x40, *range(240)])),
        (34, a, bytes([0, 0x80])),
    ]:
        size = int.from_bytes(output[:2], "big")
        frame, output = output[2 : 2 + size], output[2 + size :]
        length = 12 + len(apdu)
        flags = 0xBC if length <= 15 else 0x3C
        assert len(frame) == 10 + length
        assert frame[:12] == bytes(
            [0x11, 0, flags, 0xE0, 0x50, 2, 4, 0, length, 3, 0xF1, 0x10]
        )
        body = SecureData.from_knx(frame[12:])
        assert int.from_bytes(body.sequence_number_bytes, "big") == seq
        assert (
            body.get_plain_apdu(
                key,
                control_field(),
                frame[4:8],
                CEMIAddressType.GROUP,
                CEMIFrameFormat.STANDARD,
                TDataGroup(),
            )
            == apdu
        )
    assert not output
    print(
        "XKNX and Rust verified encrypted group traffic in both directions across key rotation."
    )


if __name__ == "__main__":
    main()
