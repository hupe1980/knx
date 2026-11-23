#![cfg(feature = "cli")]

use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};
use tempfile::NamedTempFile;

const SAMPLE: &str = include_str!("../examples/kitchen.yaml");

fn cli(args: &[&str], data: &[u8], answers: &str) -> Output {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command.args(args).arg(file.path());
    run_with_input(command, answers.as_bytes())
}

fn run_with_input(mut command: Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn render_and_append() {
    let expected = json!([{"name":"Kitchen ceiling","address":"1/1/1","state_address":"1/1/2"}]);
    for (args, document) in [
        (vec!["render"], json!({"knx":{"light":expected}})),
        (
            vec!["render", "--target", "config"],
            json!({"knx":{"light":expected}}),
        ),
        (vec!["render", "--target", "knx"], json!({"light":expected})),
        (vec!["render", "--target", "lights"], expected),
    ] {
        let output = cli(&args, SAMPLE.as_bytes(), "");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let parsed: Value =
            serde_saphyr::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
        assert_eq!(parsed, document);
        assert_eq!(output.stdout, cli(&args, SAMPLE.as_bytes(), "").stdout);
    }
}

#[test]
fn name_roundtrip() {
    // offline-light-mapping/SR-001
    let data = SAMPLE.replace("ceiling", r#""Küche:\n  other: [value]""#);
    let output = cli(&["render"], data.as_bytes(), "");
    let parsed: Value =
        serde_saphyr::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
    assert_eq!(
        parsed["knx"]["light"][0]["name"],
        "Kitchen Küche:\n  other: [value]"
    );
}

#[test]
fn diagnostics_and_limits() {
    // offline-light-mapping/SR-002 offline-light-mapping/SR-005
    for (data, message) in [
        (format!("{SAMPLE}password: SENSITIVE\n"), "field"),
        (format!("{SAMPLE}room: SENSITIVE\n"), "duplicate"),
        (SAMPLE.replace(", dpt: \"1.001\"", ""), "command.dpt"),
        (SAMPLE.replace("1.001", "1.005"), "command.dpt"),
        (SAMPLE.replace("1/1/2", "1/1/1"), "feedback.address"),
        (SAMPLE.replace("1/1/2", "31/7/256"), "feedback.address"),
        (SAMPLE.replace("1/1/1", "0/0/0"), "command.address"),
        (SAMPLE.replace("1/1/1", "1/8/1"), "command.address"),
        (SAMPLE.replace("1/1/1", "32/0/1"), "command.address"),
        (SAMPLE.replace("1/1/1", "01/1/1"), "command.address"),
        (SAMPLE.replace("Kitchen", "&x SENSITIVE"), "YAML"),
        (SAMPLE.replace("Kitchen", "!custom SENSITIVE"), "YAML"),
        (String::from("room: 'SENSITIVE"), "YAML"),
        (
            String::from("room: {nested: {a: {b: {c: SENSITIVE}}}}"),
            "YAML",
        ),
        (SAMPLE.replace("Kitchen", "*undefined"), "YAML"),
        (format!("{SAMPLE}---\nroom: SENSITIVE"), "YAML"),
        ("x".repeat(16385), "16 KiB"),
    ] {
        let out = cli(&["render"], data.as_bytes(), "");
        assert_eq!(out.status.code(), Some(2), "{message}");
        assert!(out.stdout.is_empty());
        let error = String::from_utf8(out.stderr).unwrap();
        assert!(error.contains(message), "{error}");
        assert!(!error.contains("SENSITIVE"));
    }
    assert_eq!(cli(&["validate"], &[0xff], "").status.code(), Some(2));
    let bounds = SAMPLE
        .replace("1/1/1", "0/0/1")
        .replace("1/1/2", "31/7/255");
    assert!(cli(&["validate"], bounds.as_bytes(), "").status.success());
}

#[test]
fn handover_outcomes() {
    // assisted-handover/SR-003 assisted-handover/SR-004
    for (edits, status) in [
        (vec![], "verified"),
        (vec![(0, "")], "inconclusive"),
        (vec![(1, "unknown"), (4, "no")], "failed"),
        (vec![(5, "no")], "failed"),
        (vec![(7, "plain")], "failed"),
        (vec![(9, "plain")], "failed"),
        (vec![(11, "plain")], "failed"),
        (vec![(6, "unknown")], "inconclusive"),
        (vec![(10, "plain-allowed"), (11, "plain")], "verified"),
    ] {
        let mut answers = vec!["yes"; 6];
        answers.extend(["secure-required", "secure"].repeat(3));
        for (i, value) in edits {
            answers[i] = value;
        }
        let out = cli(
            &["handover"],
            SAMPLE.as_bytes(),
            &format!("invalid\n{}\n", answers.join("\n")),
        );
        assert_eq!(out.status.code(), Some(i32::from(status != "verified")));
        let record: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(record["result"], status);
        assert_eq!(record["evidence"], "installer-reported");
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .contains("listed answer")
        );
    }
    let out = cli(&["handover"], SAMPLE.as_bytes(), "yes\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn bounded_interview() {
    // assisted-handover/SR-008
    let result = cli(&["handover"], SAMPLE.as_bytes(), &"Q".repeat(200));
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(!String::from_utf8(result.stderr).unwrap().contains("QQQQ"));
}

#[test]
fn standalone_example() {
    let dir = tempfile::tempdir().unwrap();
    let example = Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("example")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(example.status.success());
    assert_eq!(example.stdout, SAMPLE.as_bytes());
    assert!(cli(&["validate"], &example.stdout, "").status.success());
}

#[test]
fn ets_address_styles() {
    // offline-light-mapping/SR-009
    for (command, feedback) in [("1/1/1", "1/258"), ("1/257", "2306"), ("2305", "1/1/2")] {
        let mapping = SAMPLE.replace("1/1/1", command).replace("1/1/2", feedback);
        let out = cli(&["render"], mapping.as_bytes(), "");
        assert!(out.status.success());
        let result: Value =
            serde_saphyr::from_str(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
        let light = &result["knx"]["light"][0];
        assert_eq!(light["address"], "1/1/1");
        assert_eq!(light["state_address"], "1/1/2");
        let record = cli(&["handover"], mapping.as_bytes(), &"unknown\n".repeat(12));
        let json: Value = serde_json::from_slice(&record.stdout).unwrap();
        assert_eq!(json["mapping"]["command"]["address"], command);
        assert_eq!(json["mapping"]["feedback"]["address"], feedback);
    }
    for (a, b) in [
        ("1/257", "2305"),
        ("2305", "1/1/1"),
        ("31/2047", "65535"),
        ("1", "0/0/1"),
    ] {
        let mapping = SAMPLE.replace("1/1/1", a).replace("1/1/2", b);
        for action in ["validate", "render", "handover"] {
            let out = cli(&[action], mapping.as_bytes(), "");
            assert_eq!(out.status.code(), Some(2));
            assert!(out.stdout.is_empty());
            assert!(String::from_utf8_lossy(&out.stderr).contains("independent actuator"));
        }
    }
    for bad in [
        "65536", "0", "0/0", "1/2048", "32/0", "+1", "00", "1/02", "1/2/3/4",
    ] {
        let out = cli(&["validate"], SAMPLE.replace("1/1/2", bad).as_bytes(), "");
        assert_eq!(out.status.code(), Some(2), "{bad}");
    }
}

#[test]
fn report_file_preserves_prior_records() {
    // assisted-handover/SR-010
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.json");
    let args = ["handover", "--output", path.to_str().unwrap()];
    let aborted = cli(&args, SAMPLE.as_bytes(), "yes\n");
    assert_eq!(aborted.status.code(), Some(2));
    assert!(!path.exists());

    let saved = cli(&args, SAMPLE.as_bytes(), &"unknown\n".repeat(12));
    assert_eq!(saved.status.code(), Some(1));
    assert!(saved.stdout.is_empty());
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["result"],
        "inconclusive"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o077, 0);
    }
    let second = cli(&args, SAMPLE.as_bytes(), "");
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn competing_report_is_preserved() {
    // assisted-handover/SR-010
    use std::io::{BufRead, BufReader};
    let dir = tempfile::tempdir().unwrap();
    let mapping = dir.path().join("room.yaml");
    let report = dir.path().join("competing.json");
    std::fs::write(&mapping, SAMPLE).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("handover")
        .arg(mapping)
        .arg("--output")
        .arg(&report)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first_line = String::new();
    BufReader::new(child.stderr.as_mut().unwrap())
        .read_line(&mut first_line)
        .unwrap();
    assert!(first_line.starts_with("Verify"));
    std::fs::write(&report, "other interview").unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all("unknown\n".repeat(12).as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(std::fs::read_to_string(report).unwrap(), "other interview");
}

#[test]
fn findings_explain_the_verdict_in_both_outputs() {
    // assisted-handover/SR-003 assisted-handover/SR-004
    let answers = [
        "yes",
        "no",
        "yes",
        "unknown",
        "yes",
        "yes",
        "secure-required",
        "secure",
        "secure-required",
        "plain",
        "unknown",
        "plain",
    ];
    let out = cli(&["handover"], SAMPLE.as_bytes(), &answers.join("\n"));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["result"], "failed");
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 4);
    let terminal = String::from_utf8(out.stderr).unwrap();
    for (finding, (check, status)) in findings.iter().zip([
        ("dashboard_feedback", "failed"),
        ("wall_feedback", "unknown"),
        ("command group", "failed"),
        ("feedback group", "unknown"),
    ]) {
        assert_eq!(finding["check"], check);
        assert_eq!(finding["status"], status);
        let action = finding["action"].as_str().unwrap();
        assert!(!action.is_empty());
        assert!(terminal.contains(&format!("{check} ({status}): {action}")));
    }

    let passed = cli(
        &["handover"],
        SAMPLE.as_bytes(),
        &("yes\n".repeat(6) + &"plain-allowed\nplain\n".repeat(3)),
    );
    let report: Value = serde_json::from_slice(&passed.stdout).unwrap();
    assert_eq!(report["findings"], json!([]));
    assert!(passed.status.success());
}

#[test]
fn invalid_report_parent_fails_before_interview() {
    // assisted-handover/SR-010
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("missing/report.json");
    let out = cli(
        &["handover", "--output", report.to_str().unwrap()],
        SAMPLE.as_bytes(),
        "",
    );
    assert_eq!(out.status.code(), Some(2));
    let diagnostic = String::from_utf8(out.stderr).unwrap();
    assert!(diagnostic.contains("directory"));
    assert!(!diagnostic.contains("Verify"));
    assert!(out.stdout.is_empty());
    assert!(!report.exists());
}

#[test]
fn render_target_help_and_errors() {
    let bad = cli(&["render", "--target", "unknown"], SAMPLE.as_bytes(), "");
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    let help = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["render", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for value in ["config", "knx:", "lights"] {
        assert!(help.contains(value), "{help}");
    }
}

#[test]
fn independent_field_errors_are_reported_together() {
    // offline-light-mapping/SR-002 offline-light-mapping/SR-009
    let data = "room: ''\nlight: ''\ncommand: {address: SECRET, dpt: SECRET}\nfeedback: {address: SECRET, dpt: SECRET}";
    let mut previous = None;
    for action in ["validate", "render", "handover"] {
        let out = cli(&[action], data.as_bytes(), "");
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        let diagnostics = String::from_utf8(out.stderr).unwrap();
        assert!(!diagnostics.contains("SECRET"));
        let fields: Vec<_> = diagnostics
            .lines()
            .map(|s| s.split_once(' ').unwrap().0)
            .collect();
        assert_eq!(
            fields,
            [
                "room",
                "light",
                "command.address",
                "command.dpt",
                "feedback.address",
                "feedback.dpt"
            ]
        );
        if let Some(previous) = previous {
            assert_eq!(previous, diagnostics);
        }
        previous = Some(diagnostics);
    }

    let alias = SAMPLE.replace("1/1/2", "2305").replace("1.001", "SECRET");
    let out = cli(&["validate"], alias.as_bytes(), "");
    let errors = String::from_utf8(out.stderr).unwrap();
    assert!(errors.contains("command.dpt") && errors.contains("feedback.dpt"));
    assert!(errors.contains("independent actuator") && !errors.contains("SECRET"));
}

#[test]
fn pipe_and_file_inputs_share_the_same_contract() {
    // offline-light-mapping/SR-002 offline-light-mapping/SR-005
    let mut limit = SAMPLE.as_bytes().to_vec();
    limit.resize(16 * 1024, b' ');
    let oversized = [limit.as_slice(), b" "].concat();
    for input in [
        SAMPLE.as_bytes(),
        limit.as_slice(),
        oversized.as_slice(),
        &[0xff],
        b"room: [PRIVATE",
        b"room: ''\nlight: ''",
    ] {
        for args in [vec!["validate"], vec!["render", "--target", "lights"]] {
            let expected = cli(&args, input, "");
            let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
            command.args(args).arg("-");
            let actual = run_with_input(command, input);
            assert_eq!(actual.status.code(), expected.status.code());
            assert_eq!(actual.stdout, expected.stdout);
            assert_eq!(actual.stderr, expected.stderr);
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["handover", "-"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("interview answers")
    );
}

#[test]
fn guide_is_available_outside_the_checkout() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("guide")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    let source = include_str!("../site/content/guide/_index.md").replace("\r\n", "\n");
    let (_, body) = source.split_once("\n+++\n").unwrap();
    assert_eq!(
        text.trim_end(),
        format!("# KNX light handover guide\n\n{}", body.trim())
    );
    assert!(text.contains("knx handover"));
}

#[test]
fn source_archive_selects_public_files_without_git() {
    // offline-light-mapping/SR-011. Simulate a source export, where Git ignore rules cannot help.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let public = [
        "src/main.rs",
        "tests/cli.rs",
        "examples/kitchen.yaml",
        "examples/decode_meter.rs",
        "examples/decode_power.rs",
        "examples/datapoints.rs",
        "examples/interface_objects.rs",
        "README.md",
        "CHANGELOG.md",
        "site/static/standards.csv",
        "site/content/inventory/_index.md",
        "site/templates/inventory.html",
        "tools/standards_inventory.py",
        "tools/release_evidence.py",
        "tools/interop/switch_peer.py",
        "tools/interop/switch_client.rs",
        "tools/interop/switch-peer.lock",
        "tools/interop/application_profiles_client.rs",
        "tools/interop/application_profiles_peer.py",
        "tools/interop/tunnel_peer.py",
        "tools/interop/tunnel_client.rs",
        "tools/interop/data_secure_client.rs",
        "tools/interop/data_secure_peer.py",
        "tools/interop/ip_secure_client.rs",
        "tools/interop/SecureRoutingPeer.java",
        "tools/interop/DataSyncPeer.java",
        "tools/interop/data_sync_client.rs",
        "tools/interop/calimero_client.rs",
        "tools/interop/calimero_loopback.py",
        "tools/test_release_evidence.py",
        "site/config.toml",
        "site/content/_index.md",
        "site/content/documentation/_index.md",
        "site/templates/documentation.html",
        "site/content/guide/_index.md",
        "site/content/transport/_index.md",
        "site/templates/base.html",
        "site/templates/guide.html",
        "site/templates/index.html",
        "site/static/style.css",
        "site/static/mark.svg",
        "site/static/kitchen.webp",
        "LICENSE-APACHE",
        "LICENSE-MIT",
    ];
    let local = [
        "concepts/NOTES.md",
        "specs/001-example/spec.md",
        ".claude/settings.json",
        ".specify/memory/constitution.md",
        "site/public/index.html",
        "examples/household.yaml",
        "handover.json",
        "project.knxkeys",
    ];
    std::fs::write(root.join("Cargo.toml"), include_str!("../Cargo.toml")).unwrap();
    for name in public.iter().chain(&local) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    let output = Command::new(env!("CARGO"))
        .current_dir(root)
        .args(["package", "--offline", "--list"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8(output.stdout).unwrap();
    for file in public {
        assert!(listing.lines().any(|line| line == file), "missing {file}");
    }
    for file in local {
        assert!(!listing.lines().any(|line| line == file), "included {file}");
    }
}

#[test]
fn diff_distinguishes_notation_roles_and_invalid_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let previous = dir.path().join("previous.yaml");
    std::fs::write(&previous, SAMPLE).unwrap();
    let compare = |new: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
        command.arg("diff").arg(&previous).arg("-");
        run_with_input(command, new.as_bytes())
    };
    assert!(
        compare(&SAMPLE.replace("1/1/1", "2305").replace("1/1/2", "1/258"))
            .status
            .success()
    );
    let renamed = compare(&SAMPLE.replace("Kitchen", "Living"));
    assert_eq!(renamed.status.code(), Some(1));
    assert_eq!(renamed.stdout, b"Room label changed.\n");
    let swapped = compare(&SAMPLE.replace("1/1/1", "2306").replace("1/1/2", "2305"));
    assert_eq!(swapped.status.code(), Some(1));
    let output = String::from_utf8(swapped.stdout).unwrap();
    assert!(
        output.contains("Command: 1/1/1 -> 1/1/2")
            && output.contains("Feedback: 1/1/2 -> 1/1/1")
            && output.contains("exchanged roles")
    );
    let invalid = compare("private: secret");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    let diagnostic = String::from_utf8(invalid.stderr).unwrap();
    assert!(diagnostic.contains("Proposed mapping") && !diagnostic.contains("secret"));
    assert_eq!(std::fs::read_to_string(previous).unwrap(), SAMPLE);
}

#[test]
fn decode_accepts_ets_values_and_reports_errors_without_echoing_arguments() {
    let decode = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_knx"))
            .arg("decode")
            .args(args)
            .output()
            .unwrap()
    };
    for (args, expected) in [
        (&["1.001", "01"][..], "on"),
        (&["1.001", "00"], "off"),
        (&["2.001", "01"], "release"),
        (&["3.007", "08"], "stop"),
        (&["3.008", "0f"], "down 1/64"),
        (&["17.001", "3f"], "64"),
        (&["18.001", "bf"], "learn 64"),
        (&["5.001", "ff"], "100 %"),
        (&["9.001", "07", "D0"], "20 °C"),
        (&["13.010", "ff FF ff FF"], "-1 Wh"),
        (&["14.056", "c6bb8000"], "-24000 W"),
    ] {
        let output = decode(args);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{expected}\n")
        );
    }
    for (args, hint) in [
        (&["9.001", "7fff"][..], "invalid data"),
        (&["9.001", "07"], "received 1"),
        (&["1.001", "ff"], "range"),
        (&["6.020", "FF"], "mode bits"),
        (&["6.001", "0080"], "received 2"),
        (&["5.006", "FF"], "range"),
        (&["5.010", "0001"], "received 2"),
        (&["2.012", "04"], "range"),
        (&["2.001", "0001"], "received 2"),
        (&["3.007", "10"], "range"),
        (&["17.001", "40"], "range"),
        (&["18.001", "c0"], "range"),
        (&["private-input", "00"], "supported DPT"),
        (&["14.056", "7f800000"], "finite"),
        (&["1.001", "0 1"], "pairs"),
        (&["1.001", "00", "-private-input"], "pairs"),
        (&["1.001", "é"], "pairs"),
        (&["14.056", "0011223344"], "received 5"),
        (&["29.010", "00000001"], "received 4"),
        (&["29.011", "001122334455667788"], "eight"),
        (&["29.012", "0011223344556x77"], "pairs"),
        (&["14.056", ""], "received 0"),
    ] {
        let output = decode(args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains(hint) && !error.contains("private-input"),
            "{error}"
        );
    }
}

#[test]
fn offline_encoding_round_trips_and_redacts_invalid_values() {
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(args)
            .output()
            .unwrap()
    };
    for (dpt, value, hex, decoded) in [
        ("6.001", "-128", "80", "-128 %"),
        ("6.010", "127", "7F", "127 pulses"),
        ("6.010", "-1", "FF", "-1 pulse"),
        ("5.010", "1", "01", "1 pulse"),
        ("5.003", "360", "FF", "360 °"),
        ("5.005", "10", "0A", "10 ratio"),
        ("5.006", "unavailable", "00", "unavailable"),
        ("5.006", "254", "FE", "254"),
        ("5.010", "255", "FF", "255 pulses"),
        (
            "29.010",
            "-9223372036854775808",
            "80 00 00 00 00 00 00 00",
            "-9223372036854775808 Wh",
        ),
        (
            "29.012",
            "9223372036854775807",
            "7F FF FF FF FF FF FF FF",
            "9223372036854775807 VARh",
        ),
        ("13.011", "16909060", "01 02 03 04", "16909060 VAh"),
        ("13.012", "16909060", "01 02 03 04", "16909060 VARh"),
        ("13.013", "16909060", "01 02 03 04", "16909060 kWh"),
        ("13.014", "16909060", "01 02 03 04", "16909060 kVAh"),
        ("13.015", "16909060", "01 02 03 04", "16909060 kVARh"),
        ("13.016", "16909060", "01 02 03 04", "16909060 MWh"),
        (
            "29.010",
            "9007199254740993",
            "00 20 00 00 00 00 00 01",
            "9007199254740993 Wh",
        ),
        (
            "29.011",
            "9007199254740993",
            "00 20 00 00 00 00 00 01",
            "9007199254740993 VAh",
        ),
        (
            "29.012",
            "9007199254740993",
            "00 20 00 00 00 00 00 01",
            "9007199254740993 VARh",
        ),
        ("1.001", "off", "00", "off"),
        ("5.001", "100", "FF", "100 %"),
        ("5.004", "255", "FF", "255 %"),
        ("9.001", "20", "07 D0", "20 °C"),
        ("13.010", "-1", "FF FF FF FF", "-1 Wh"),
        ("14.056", "-24000", "C6 BB 80 00", "-24000 W"),
    ] {
        let encoded = run(&["encode", dpt, value]);
        assert!(encoded.status.success() && encoded.stderr.is_empty());
        let output = String::from_utf8(encoded.stdout).unwrap();
        assert_eq!(output, format!("{hex}\n"));
        let replay = run(&["decode", dpt, output.trim_end()]);
        assert!(replay.status.success());
        assert_eq!(
            String::from_utf8(replay.stdout).unwrap(),
            format!("{decoded}\n")
        );
    }
    for args in [
        &["1.001", "private"][..],
        &["5.001", "101"],
        &["7.003", "15", "ms"],
        &["8.003", "-1", "ms"],
        &["7.002", "-1", "ms"],
        &["8.005", "1"],
        &["8.005", "1", "private"],
        &["8.005", "1.5", "s"],
        &["7.007", "9223372036854775807", "h"],
        &["7.007", "65536", "h"],
        &["6.001", "-129"],
        &["6.010", "128"],
        &["6.010", "1.5"],
        &["6.020", "aa", "1"],
        &["6.020", "none", "3"],
        &["6.020", "", "0"],
        &["6.020", "private", "0"],
        &["6.020", "a"],
        &["5.003", "361"],
        &["5.003", "NaN"],
        &["5.005", "2.5"],
        &["5.006", "0"],
        &["5.006", "255"],
        &["5.006", "private"],
        &["5.010", "-1"],
        &["5.010", "256"],
        &["2.001", "private"],
        &["2.008", "close"],
        &["2.013", "release"],
        &["3.007", "increase", "1/3"],
        &["3.008", "decrease", "1/2"],
        &["3.008", "stop", "private"],
        &["17.001", "0"],
        &["18.001", "private", "64"],
        &["18.001", "learn", "65"],
        &["5.004", "256"],
        &["5.004", "50.5"],
        &["9.001", "-274"],
        &["13.010", "2147483648"],
        &["29.010", "9223372036854775808"],
        &["29.011", "-9223372036854775809"],
        &["29.012", "1.5"],
        &["29.010", "private"],
        &["13.014", "2", "private"],
        &["14.056", "NaN"],
        &["14.056", "1", "private"],
        &["private", "42"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .arg("encode")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error.is_empty() && !error.contains("private"));
    }
}

#[test]
fn offline_control_commands_encode_and_decode() {
    for (dpt, command, hex) in [
        ("8.005", "-2000 ms", "FF FE"),
        ("7.004", "300 ms", "00 03"),
        ("7.007", "235926000000 ms", "FF FF"),
        ("6.020", "ace 1", "52"),
        ("6.020", "none 0", "F9"),
        ("6.020", "abcde 2", "04"),
        ("3.007", "increase 1/4", "0B"),
        ("3.007", "decrease 1/2", "02"),
        ("3.007", "stop", "00"),
        ("3.008", "up 1/1", "01"),
        ("3.008", "down 1/64", "0F"),
        ("3.008", "stop", "00"),
        ("17.001", "1", "00"),
        ("18.001", "learn 64", "BF"),
        ("18.001", "activate 64", "3F"),
    ] {
        let encoded = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["encode", dpt])
            .args(command.split_whitespace())
            .output()
            .unwrap();
        assert!(encoded.status.success() && encoded.stderr.is_empty());
        assert_eq!(encoded.stdout, format!("{hex}\n").as_bytes());
        let decoded = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["decode", dpt, hex])
            .output()
            .unwrap();
        assert!(decoded.status.success() && decoded.stderr.is_empty());
        assert_eq!(decoded.stdout, format!("{command}\n").as_bytes());
    }
}

#[test]
fn priority_commands_use_the_commissioned_function_labels() {
    for (id, zero, one) in [
        ("2.001", "off", "on"),
        ("2.002", "false", "true"),
        ("2.003", "disable", "enable"),
        ("2.004", "no-ramp", "ramp"),
        ("2.005", "no-alarm", "alarm"),
        ("2.006", "low", "high"),
        ("2.007", "decrease", "increase"),
        ("2.008", "up", "down"),
        ("2.009", "open", "close"),
        ("2.010", "stop", "start"),
        ("2.011", "inactive", "active"),
        ("2.012", "normal", "inverted"),
    ] {
        for (token, byte) in [("release", "00"), (zero, "02"), (one, "03")] {
            let encoded = Command::new(env!("CARGO_BIN_EXE_knx"))
                .args(["encode", id, token])
                .output()
                .unwrap();
            assert!(encoded.status.success() && encoded.stderr.is_empty());
            assert_eq!(encoded.stdout, format!("{byte}\n").as_bytes());
            let decoded = Command::new(env!("CARGO_BIN_EXE_knx"))
                .args(["decode", id, byte])
                .output()
                .unwrap();
            assert!(decoded.status.success() && decoded.stderr.is_empty());
            assert_eq!(decoded.stdout, format!("{token}\n").as_bytes());
        }
    }
}

#[test]
fn interval_cli_converts_explicit_units_and_explains_tick_errors() {
    for (id, value, unit, wire) in [
        ("8.005", "-2", "s", "FF FE"),
        ("7.006", "3", "min", "00 03"),
        ("7.007", "1", "h", "00 01"),
        ("8.002", "1", "ms", "00 01"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["encode", id, value, unit])
            .output()
            .unwrap();
        assert!(result.status.success() && result.stderr.is_empty());
        assert_eq!(result.stdout, format!("{wire}\n").as_bytes());
    }
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["encode", "7.003", "15", "ms"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("multiples of 10 ms")
    );
}
