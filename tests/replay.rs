#![cfg(feature = "cli")]

use serde_json::Value;
use std::process::Command;

fn replay(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("replay")
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn incident_and_snapshot_use_independent_measurements() {
    let confirmed = replay(&[]);
    assert_eq!(confirmed["delivery"], "confirmed");
    assert_eq!(confirmed["readings"][1]["watts"], 24000.);
    let missing = replay(&["--missing-final"]);
    assert_eq!(missing, replay(&["--missing-final"]));
    assert_eq!(missing["delivery"], "unconfirmed");
    assert_eq!(missing["readings"].as_array().unwrap().len(), 1);
    assert_eq!(missing["readings"][0]["watts"], 28000.);
    for report in [missing, confirmed] {
        assert!(report["snapshot_charging_w"].is_null());
        assert_eq!(report["connection"]["headroom_w"], 8000);
    }
    assert!(replay(&["--connection-age", "31"])["connection"]["headroom_w"].is_null());
}

#[test]
fn fixture_diagnostics_identify_the_point_without_echoing_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.json");
    std::fs::write(
        &path,
        include_str!("../examples/charging.json").replace("14.056", "sensitive"),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["replay", "--fixture"])
        .arg(path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let err = String::from_utf8(result.stderr).unwrap();
    assert!(
        err.contains("Charging sample 1") && err.contains("14.056") && !err.contains("sensitive")
    );
}

#[test]
fn installed_fixture_round_trips_through_stdin() {
    use std::io::Write;
    use std::process::Stdio;
    let bin = env!("CARGO_BIN_EXE_knx");
    let fixture = Command::new(bin)
        .args(["example", "--charging"])
        .output()
        .unwrap();
    assert!(fixture.status.success());
    assert_eq!(fixture.stdout, include_bytes!("../examples/charging.json"));
    let mut child = Command::new(bin)
        .args(["replay", "--fixture", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&fixture.stdout)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        replay(&[])
    );
}

#[test]
fn replay_fixture_byte_limit_applies_before_json_decoding() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.json");
    let mut input = b"[]".to_vec();
    input.resize(16 * 1024, b' ');
    for size in [16 * 1024, 16 * 1024 + 1] {
        input.resize(size, b' ');
        std::fs::write(&path, &input).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["replay", "--fixture"])
            .arg(&path)
            .output()
            .unwrap();
        if size == 16 * 1024 {
            assert!(result.status.success());
        } else {
            assert_eq!(result.status.code(), Some(2));
            assert!(result.stdout.is_empty());
            assert!(String::from_utf8(result.stderr).unwrap().contains("16 KiB"));
        }
    }
}
