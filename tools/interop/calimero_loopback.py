"""Run the Rust discovery and routing client against a source-pinned virtual KNX gateway."""

import json
import os
import platform
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REVISIONS = {
    "core": "68c99bffb5d065220410311fbdf8f372fd84555e",
    "device": "00586da006a5738f53276387a8e81af16222a005",
    "server": "865ee808ba255f28eb83e3a90b6d37869b733541",
}


def run(*args, cwd=None):
    subprocess.run(args, check=True, cwd=cwd)


def execute(work):
    loopback = "lo0" if platform.system() == "Darwin" else "lo"
    socket.if_nametoindex(loopback)
    javac = shutil.which("javac")
    if "JAVA_HOME" in os.environ:
        jdk = Path(os.environ["JAVA_HOME"]) / "bin"
    elif javac:
        jdk = Path(javac).parent
    else:
        raise SystemExit(
            "Install JDK 21 or newer and set JAVA_HOME before running the gateway check."
        )
    classes = work / "classes"
    classes.mkdir()
    sources = []
    for project, revision in REVISIONS.items():
        checkout = work / project
        run("git", "init", "-q", checkout)
        run(
            "git",
            "fetch",
            "-q",
            "--depth=1",
            f"https://github.com/calimero-project/calimero-{project}.git",
            revision,
            cwd=checkout,
        )
        run("git", "checkout", "-q", "FETCH_HEAD", cwd=checkout)
        tree = checkout / ("src/main/java" if project == "device" else "src")
        sources.extend(p for p in tree.rglob("*.java") if p.name != "module-info.java")
    sources.extend(ROOT / "tools/interop" / file for file in ("SecureRoutingPeer.java", "DataSyncPeer.java"))
    source_list = work / "sources"
    source_list.write_text("\n".join(f'"{p}"' for p in sources))
    run(jdk / "javac", "--release", "21", "-d", classes, f"@{source_list}")
    resources = [
        work / "core/resources",
        work / "device/src/main/resources",
        work / "server/resources",
    ]
    classpath = os.pathsep.join(map(str, [classes, *resources]))
    (work / "loopback.keys").write_text(
        "device.key=e158e4012047bd6cc41aafbc5c04c1fc"
        + "\nuser[1].key="
        + "56" * 16
        + "\nuser[2].key=03fcedb66660251ec81a1a716901696a"
        + "\n"
    )
    # Parallel runs must have independent unicast endpoints and routing memberships.
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    group = f"239.255.{port >> 8}.{port & 255}"
    config = work / "server.xml"
    config.write_text(f"""
<knxServer name="loopback" friendlyName="Calimero loopback">
  <trace format="json">routing.jsonl</trace>
  <discovery listenNetIf="{loopback}" outgoingNetIf="{loopback}" activate="true"/>
  <serviceContainer routing="true" netif="{loopback}" udpPort="{port}" keyfile="loopback.keys" securedServices="optional">
    <knxAddress type="individual">7.1.0</knxAddress>
    <routing>{group}</routing>
    <knxSubnet type="virtual">isolated</knxSubnet>
    <additionalAddresses><knxAddress type="individual">7.1.1</knxAddress></additionalAddresses>
    <tunnelingUsers><user id="2"><knxAddress type="individual">7.1.1</knxAddress></user></tunnelingUsers>
    <timeServer><datapoint stateBased="true" name="time" dptID="10.001" priority="low">
      <knxAddress type="group">3/4/5</knxAddress><expiration timeout="2"/>
    </datapoint></timeServer>
  </serviceContainer>
</knxServer>
""")
    manifest = work / "Cargo.toml"
    manifest.write_text(f"""
[package]
name = "knx-loopback-check"
version = "0.0.0"
edition = "2024"
[[bin]]
name = "knx-loopback-check"
path = "client.rs"
[[bin]]
name = "secure-session-check"
path = "secure_client.rs"
[[bin]]
name = "data-sync-check"
path = "data_sync_client.rs"
[dependencies]
knx = {{ path = {json.dumps(str(ROOT))}, features = ["keyring", "secure-routing", "data-secure"] }}
socket2 = {{ version = "=0.6.5", features = ["all"] }}
""")
    shutil.copy(ROOT / "tools/interop/calimero_client.rs", work / "client.rs")
    shutil.copy(ROOT / "tools/interop/ip_secure_client.rs", work / "secure_client.rs")
    shutil.copy(ROOT / "tools/interop/loopback.knxkeys", work / "loopback.knxkeys")
    shutil.copy(ROOT / "tools/interop/data_sync_client.rs", work / "data_sync_client.rs")
    run("cargo", "build", "--quiet", "--manifest-path", manifest)
    run(jdk / "java", "-cp", classpath, "io.calimero.secure.DataSyncPeer", work / "target/debug/data-sync-check")
    log = work / "server.log"
    with log.open("w") as output:
        server = subprocess.Popen(
            [
                jdk / "java",
                # The pinned server starts its virtual dispatcher immediately before
                # setting its run flag. Schedule both on one carrier during the trial.
                "-Djdk.virtualThreadScheduler.parallelism=1",
                "-Djdk.virtualThreadScheduler.maxPoolSize=1",
                "-Djava.net.preferIPv4Stack=true",
                "-cp",
                classpath,
                "io.calimero.server.Launcher",
                config,
            ],
            cwd=work,
            stdin=subprocess.PIPE,
            stdout=output,
            stderr=subprocess.STDOUT,
        )
        try:
            for _ in range(100):
                if server.poll() is not None:
                    raise RuntimeError(log.read_text())
                if (
                    "control endpoint" in log.read_text()
                    and f"127.0.0.1:{port} is up and running" in log.read_text()
                ):
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError(log.read_text())
            run(
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                manifest,
                "--bin",
                "secure-session-check",
                "--",
                str(port),
                work / "core/test/resources/KeyringTest.knxkeys",
            )
            run(
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                manifest,
                "--bin",
                "knx-loopback-check",
                "--",
                str(port),
                group,
            )
            trace = work / "routing.jsonl"
            for _ in range(50):
                lines = trace.read_text().splitlines()
                if any(
                    event.get("path") == "C→S"
                    and event.get("subnet") == "isolated"
                    and frame.get("hopCount") == 5
                    and frame.get("src") == "7.1.2"
                    and frame.get("dst") == "3/4/5"
                    and frame.get("apci") == "A_Group.read"
                    for event in map(json.loads, lines)
                    for frame in (event["frame"],)
                ):
                    print("Calimero forwarded the Rust group read on 3/4/5 with five hops remaining.")
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError(
                    "No matching group read in Calimero trace: " + trace.read_text()
                )
        except (RuntimeError, subprocess.CalledProcessError) as error:
            trace = work / "routing.jsonl"
            tail = trace.read_text().splitlines() if trace.exists() else []
            raise RuntimeError(log.read_text() + "\nVirtual KNX trace:\n" + "\n".join(tail)) from error
        finally:
            try:
                server.communicate(b"stop\n", timeout=4)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()

    for mode, prefix in [("routing", "239.254"), ("timer", "239.253")]:
        secure_group = f"{prefix}.{port >> 8}.{port & 255}"
        secure_multicast_peer(work, jdk, classpath, loopback, manifest, mode, secure_group)


def secure_multicast_peer(work, jdk, classpath, loopback, manifest, mode, secure_group):
    ready = work / f"{mode}.ready"
    with subprocess.Popen(
        [
            jdk / "java",
            "-Djava.net.preferIPv4Stack=true",
            "-cp",
            classpath,
            "SecureRoutingPeer",
            loopback,
            ready,
            secure_group,
            mode,
        ],
        cwd=work,
    ) as multicast:
        try:
            for _ in range(80):
                if multicast.poll() is not None:
                    raise RuntimeError(
                        "Calimero multicast peer exited before joining loopback"
                    )
                if ready.exists():
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError("Calimero multicast peer did not join loopback")
            run(
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                manifest,
                "--bin",
                "knx-loopback-check",
                "--",
                "--group-timer" if mode == "timer" else "--secure-routing",
                str(socket.if_nametoindex(loopback)),
                secure_group,
            )
            if multicast.wait(timeout=3) != 0:
                raise RuntimeError("Calimero multicast verification failed")
        finally:
            if multicast.poll() is None:
                multicast.kill()


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="knx-calimero-") as directory:
        execute(Path(directory))
