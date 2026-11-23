"""Run with uv run --with xknx==4.1.1 python tunnel_peer.py from any directory."""

import json
import subprocess
import tempfile
from pathlib import Path
from socket import AF_INET, SOCK_DGRAM, socket

from xknx.cemi import CEMIFrame, CEMIMessageCode
from xknx.knxip import (
    HPAI,
    ConnectionStateRequest,
    ConnectionStateResponse,
    ConnectRequest,
    ConnectResponse,
    ConnectResponseData,
    DisconnectRequest,
    DisconnectResponse,
    KNXIPFrame,
    TunnellingAck,
    TunnellingRequest,
)
from xknx.telegram.address import IndividualAddress
from xknx.telegram.apci import GroupValueWrite

here = Path(__file__).resolve().parent
root = next(p for p in here.parents if (p / "Cargo.toml").exists())
with (
    tempfile.TemporaryDirectory() as folder,
    socket(AF_INET, SOCK_DGRAM) as control,
    socket(AF_INET, SOCK_DGRAM) as data,
):
    for s in (control, data):
        s.bind(("127.0.0.1", 0))
        s.settimeout(4)
    app = Path(folder)
    (app / "src").mkdir()
    (app / "src/main.rs").write_bytes((here / "tunnel_client.rs").read_bytes())
    (app / "Cargo.toml").write_text(
        '[package]\nname="loopback"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nknx={path='
        + json.dumps(str(root))
        + "}\n"
    )
    subprocess.run(["cargo", "build", "--offline", "-q"], cwd=app, check=True)
    with subprocess.Popen(
        [app / "target/debug/loopback", str(control.getsockname()[1])]
    ) as client:

        def recv(s, cls):
            raw, endpoint = s.recvfrom(512)
            frame, tail = KNXIPFrame.from_knx(raw)
            assert isinstance(frame.body, cls) and not tail
            return frame.body, endpoint

        def reply(s, body, peer):
            s.sendto(KNXIPFrame.init_from_body(body).to_knx(), peer)

        _, peer = recv(control, ConnectRequest)
        reply(
            control,
            ConnectResponse(
                21,
                data_endpoint=HPAI(*data.getsockname()),
                crd=ConnectResponseData(individual_address=IndividualAddress("1.1.42")),
            ),
            peer,
        )
        read, peer = recv(data, TunnellingRequest)
        assert read.raw_cemi[-4:] == bytes.fromhex("01 01 00 00")
        reply(data, TunnellingAck(21, read.sequence_counter), peer)
        reply(
            data,
            TunnellingRequest(21, 0, bytes.fromhex("29 00 bc e0 11 02 0a 01 01 00 41")),
            peer,
        )
        ack, _ = recv(data, TunnellingAck)
        assert ack.communication_channel_id == 21 and ack.sequence_counter == 0
        extended, peer = recv(data, TunnellingRequest)
        assert extended.sequence_counter == 1
        frame = CEMIFrame.from_knx(extended.raw_cemi)
        assert frame.code == CEMIMessageCode.L_DATA_REQ
        assert (
            str(frame.data.src_addr) == "1.1.42" and str(frame.data.dst_addr) == "1/2/1"
        )
        assert isinstance(frame.data.payload, GroupValueWrite)
        assert bytes(frame.data.payload.value.value) == bytes(range(253))
        reply(data, TunnellingAck(21, extended.sequence_counter), peer)
        frame.code = CEMIMessageCode.L_DATA_IND
        reply(data, TunnellingRequest(21, 1, frame.to_knx()), peer)
        ack, _ = recv(data, TunnellingAck)
        assert ack.communication_channel_id == 21 and ack.sequence_counter == 1
        _, peer = recv(control, ConnectionStateRequest)
        reply(control, ConnectionStateResponse(21), peer)
        _, peer = recv(control, DisconnectRequest)
        reply(control, DisconnectResponse(21), peer)
        assert client.wait(timeout=3) == 0
    print(
        "XKNX 4.1.1 payloads, Rust client, separate IPv4 loopback control and data ports: passed"
    )
