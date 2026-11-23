use crate::mapping::Room;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Read, Write},
    path::Path,
};

#[derive(serde::Serialize)]
struct Finding {
    check: &'static str,
    status: &'static str,
    action: &'static str,
}

const CHECKS: [(&str, &str, &str); 6] = [
    (
        "dashboard_fixture",
        "Did the dashboard operate the intended fixture?",
        "Confirm the command address against the intended actuator channel in ETS.",
    ),
    (
        "dashboard_feedback",
        "Did fresh actuator feedback match the dashboard change?",
        "Observe the actuator status address while changing the light from the dashboard.",
    ),
    (
        "wall_fixture",
        "Did the wall switch operate the intended fixture?",
        "Operate the wall switch and identify the fixture that changes.",
    ),
    (
        "wall_feedback",
        "Did fresh feedback update the resident display after the wall switch?",
        "Compare a fresh actuator status telegram with the state shown on the dashboard.",
    ),
    (
        "lost_feedback",
        "Could the resident identify uncertainty while feedback was unavailable?",
        "Check what the resident can tell from the display during feedback loss.",
    ),
    (
        "recovery",
        "After reconnecting, did state recover without repeat actuation?",
        "Observe the fixture and its feedback when the interface reconnects.",
    ),
];

fn ask(prompt: &str, choices: &[&str]) -> Result<String, String> {
    loop {
        eprint!("{prompt} [{}/unknown] ", choices.join("/"));
        io::stderr()
            .flush()
            .map_err(|_| "Cannot write interview prompt.")?;
        let mut answer = String::new();
        if io::stdin()
            .lock()
            .take(129)
            .read_line(&mut answer)
            .map_err(|_| "Cannot read interview answer.")?
            == 0
        {
            return Err("Interview interrupted. Start again for a complete record.".into());
        }
        if answer.len() > 128 {
            return Err("Answer too long. Enter a listed option.".into());
        }
        let answer = answer.trim().to_ascii_lowercase();
        if answer.is_empty() || answer == "unknown" {
            return Ok("unknown".into());
        }
        if choices.contains(&answer.as_str()) {
            return Ok(answer);
        }
        eprintln!("Enter a listed answer, or press Enter for unknown.");
    }
}

pub fn interview(room: &Room) -> Result<(Value, u8), String> {
    eprintln!(
        "Verify {:?}. Command {}, feedback {}.",
        room.name(),
        room.command.address,
        room.feedback.address
    );
    let mut findings = Vec::new();
    let mut observations = BTreeMap::new();
    for (id, prompt, action) in CHECKS {
        let answer = ask(prompt, &["yes", "no"])?;
        if answer != "yes" {
            findings.push(Finding {
                check: id,
                status: if answer == "no" { "failed" } else { "unknown" },
                action,
            });
        }
        observations.insert(id, answer);
    }
    let mut protection = BTreeMap::new();
    for scope in ["IP connection", "command group", "feedback group"] {
        let policy = ask(
            &format!("{scope} policy?"),
            &["secure-required", "plain-allowed"],
        )?;
        let observed = ask(
            &format!("{scope} observed protection?"),
            &["secure", "plain"],
        )?;
        if policy == "unknown" || observed == "unknown" {
            findings.push(Finding {
                check: scope,
                status: "unknown",
                action: "Confirm the required policy and observed protection for this scope.",
            });
        } else if policy == "secure-required" && observed == "plain" {
            findings.push(Finding { check: scope, status: "failed", action: "Review the secure configuration for this scope and verify protection again." });
        }
        protection.insert(scope, json!({"policy": policy, "observed": observed}));
    }
    let status = if findings.iter().any(|finding| finding.status == "failed") {
        "failed"
    } else if !findings.is_empty() {
        "inconclusive"
    } else {
        "verified"
    };
    eprintln!("Installer-reported handover result: {status}.");
    for finding in &findings {
        eprintln!(
            "  {} ({}): {}",
            finding.check, finding.status, finding.action
        );
    }
    Ok((
        json!({"result": status, "evidence": "installer-reported", "mapping": room,
        "observations": observations, "protection": protection, "findings": findings}),
        u8::from(!findings.is_empty()),
    ))
}

pub fn check_destination(path: &Path) -> Result<(), String> {
    match path.symlink_metadata() {
        Ok(_) => return Err("Report path already exists. Choose a new filename.".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(_) => {
            return Err(
                "Cannot access report path. Check the output directory and permissions.".into(),
            );
        }
    }
    let last_component = path
        .as_os_str()
        .as_encoded_bytes()
        .rsplit(|b| std::path::is_separator(char::from(*b)))
        .next();
    if matches!(last_component, None | Some(b"" | b"." | b"..")) {
        return Err("Supply a filename in an existing writable directory.".into());
    }
    // An anonymous probe leaves no named staging file during the interview.
    tempfile::tempfile_in(report_directory(path))
        .map(drop)
        .map_err(|_| "Cannot create report. Choose an existing writable directory.".into())
}

fn report_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub fn save_report(path: &Path, report: &str) -> Result<(), String> {
    let mut pending = tempfile::NamedTempFile::new_in(report_directory(path))
        .map_err(|_| "Cannot create report. Choose an existing writable directory.")?;
    writeln!(pending, "{report}").map_err(|_| "Cannot write report. Check available space.")?;
    pending
        .persist_noclobber(path)
        .map_err(|error| match error.error.kind() {
            io::ErrorKind::AlreadyExists => "Report path already exists. Choose a new filename.",
            _ => "Cannot save report. Check the output directory and permissions.",
        })?;
    Ok(())
}
