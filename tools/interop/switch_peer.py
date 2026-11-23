"""Read Rust-produced Point API bodies using the KNX Association client over IPv6 loopback."""

import asyncio
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
from contextlib import ExitStack
from pathlib import Path

import aiocoap
import aiocoap.resource
import cbor2
from aiocoap.credentials import CredentialsMap
from aiocoap.oscore_sitewrapper import OscoreSiteWrapper

ROOT = Path(__file__).resolve().parents[2]
PATH = "/p/lsab/0/ioo"


def rust_bodies(directory):
    source = directory / "src"
    source.mkdir()
    (directory / "Cargo.toml").write_text(
        f'[package]\nname="switch_peer"\nversion="0.0.0"\nedition="2024"\n'
        f"[dependencies]\nknx={{path={json.dumps(str(ROOT))}}}\n"
        'coap-lite = "=0.13.3"\noscore = "=0.1.2"\n'
    )
    shutil.copy(ROOT / "tools/interop/switch_client.rs", source / "main.rs")
    shutil.copy(ROOT / "tools/interop/switch-peer.lock", directory / "Cargo.lock")
    subprocess.run(
        ["cargo", "build", "--locked", "--quiet"],
        cwd=directory,
        check=True,
        timeout=240,
    )
    executable = directory / "target/debug/switch_peer"
    lines = subprocess.check_output([executable], timeout=12)
    return [bytes.fromhex(line.decode()) for line in lines.splitlines()]


class Switch(aiocoap.resource.Resource):
    use_accept = False
    next_value = False

    async def render_get(self, request):
        if self.use_accept:
            fmt = request.opt.accept
            if (
                fmt not in (50, 60)
                or request.opt.observe is not None
                or request.opt.uri_query
            ):
                return aiocoap.Message(code=aiocoap.BAD_REQUEST)
            value = self.next_value
            self.next_value = not value
            body = json.dumps(value).encode() if fmt == 50 else cbor2.dumps({1: value})
            return aiocoap.Message(
                code=aiocoap.CONTENT, payload=body, content_format=fmt
            )
        return aiocoap.Message(
            code=aiocoap.CONTENT, payload=self.body, content_format=self.format
        )


class ProtectedSwitch(Switch):
    use_accept = True

    async def render_get(self, request):
        if ":guest" in request.remote.authenticated_claims:
            return aiocoap.Message(code=aiocoap.FORBIDDEN)
        if ":reader" not in request.remote.authenticated_claims:
            return aiocoap.Message(code=aiocoap.UNAUTHORIZED)
        return await super().render_get(request)


class Restricted(aiocoap.resource.Resource):
    async def render_get(self, request):
        return aiocoap.Message(code=aiocoap.FORBIDDEN)


async def reference_read(port, folder, fmt, expected):
    # This client patches aiocoap when imported. Keep it in its own process.
    from knxiotclient.coap_client import CoapClient, decode_payload

    client = CoapClient("::1", port, oscore_base_dir=folder)
    try:
        reply = await asyncio.wait_for(client.read(PATH), timeout=5)
        assert reply.code == "2.05 Content" and reply.content_format == fmt
        decoded = decode_payload(reply.payload, fmt)
        assert (decoded[1] if fmt == 60 else decoded) is expected
    finally:
        await client.close()


async def run():
    os.environ["AIOCOAP_CLIENT_TRANSPORT"] = "simple6"
    with tempfile.TemporaryDirectory() as tmp, ExitStack() as cleanup:
        directory = Path(tmp)
        bodies = rust_bodies(directory)
        resource = Switch()
        site = aiocoap.resource.Site()
        site.add_resource(PATH.strip("/").split("/"), resource)
        site.add_resource(["p", "private"], Restricted())
        site.add_resource(["p", "secure", "0", "ioo"], ProtectedSwitch())
        credentials = CredentialsMap()
        keys = {"reader": os.urandom(16), "guest": os.urandom(16)}
        for identity, recipient in [("reader", "01"), ("guest", "03")]:
            security = directory / identity
            security.mkdir(mode=0o700)
            (security / "secret.json").write_text(
                json.dumps(
                    {
                        "secret_hex": keys[identity].hex(),
                        "sender-id_hex": "",
                        "recipient-id_hex": recipient,
                    }
                )
            )
            label = ":" + identity
            credentials.load_from_dict({label: {"oscore": {"basedir": str(security)}}})
            # aiocoap 0.4.17 persists its replay window when releasing the context.
            cleanup.callback(credentials[label]._destroy)
        with socket.socket(socket.AF_INET6, socket.SOCK_DGRAM) as probe:
            probe.bind(("::1", 0))
            port = probe.getsockname()[1]
        context = await aiocoap.Context.create_server_context(
            OscoreSiteWrapper(site, credentials),
            bind=("::1", port),
            transports=["simplesocketserver"],
        )
        try:
            for body, (fmt, expected) in zip(
                bodies, [(50, False), (50, True), (60, False), (60, True)], strict=True
            ):
                resource.format = fmt
                resource.body = body
                await asyncio.to_thread(
                    subprocess.run,
                    [
                        sys.executable,
                        "-I",
                        __file__,
                        str(port),
                        str(directory / "contexts"),
                        str(fmt),
                        str(int(expected)),
                    ],
                    check=True,
                    timeout=10,
                )
            resource.use_accept = True
            for mode, test_key in (
                ("plain", b""),
                ("oscore", keys["reader"]),
                ("guest", keys["guest"]),
                ("unknown", os.urandom(16)),
            ):
                reader = await asyncio.create_subprocess_exec(
                    directory / "target/debug/switch_peer",
                    str(port),
                    mode,
                    stdin=asyncio.subprocess.PIPE,
                )
                try:
                    await asyncio.wait_for(
                        reader.communicate(test_key),
                        timeout=12,
                    )
                    assert reader.returncode == 0
                finally:
                    if reader.returncode is None:
                        reader.kill()
                        await reader.wait()
        finally:
            await context.shutdown()


if __name__ == "__main__":
    if len(sys.argv) == 5:
        asyncio.run(
            reference_read(
                int(sys.argv[1]),
                Path(sys.argv[2]),
                int(sys.argv[3]),
                bool(int(sys.argv[4])),
            )
        )
    else:
        asyncio.run(run())
