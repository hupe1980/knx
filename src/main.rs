mod codec_cli;
mod handover;
mod mapping;
mod replay;

use clap::{Parser, Subcommand};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "knx",
    version,
    about = "Encode and decode KNX payloads, map lights and replay a building submeter offline"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Decode an extracted KNX application payload offline
    Decode {
        #[arg(allow_hyphen_values = true, help = codec_cli::DPT_HELP)]
        dpt: String,
        /// Hex byte pairs, compact or space-separated (e.g. 07 D0)
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        hex: Vec<String>,
    },
    /// Encode a value as extracted application bytes without sending a telegram
    Encode {
        #[arg(allow_hyphen_values = true, help = codec_cli::DPT_HELP)]
        dpt: String,
        /// Value or command, e.g. on, 20, increase 1/4, learn 64 or -2 s
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true, value_name = "VALUE")]
        value: Vec<String>,
    },
    /// Replay a synthetic charging submeter and emit JSON
    Replay {
        /// Read a JSON fixture from a file or - for stdin
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Omit the final branch reading
        #[arg(long)]
        missing_final: bool,
        /// Age of the independent connection reading at the snapshot, in seconds
        #[arg(long, default_value_t = 0)]
        connection_age: u32,
    },
    /// Print a bundled fixture to stdout
    Example {
        /// Print the charging replay JSON instead of the kitchen mapping
        #[arg(long)]
        charging: bool,
    },
    /// Print the offline installer field guide as Markdown
    Guide,
    /// Check a room mapping offline
    Validate {
        /// Mapping file, or - to read YAML from stdin
        mapping: PathBuf,
    },
    /// Compare two room mappings offline (exit 1 when they differ)
    Diff {
        /// Previously reviewed mapping file, or - for stdin
        before: PathBuf,
        /// Proposed mapping file, or - for stdin
        after: PathBuf,
    },
    /// Preview Home Assistant YAML
    Render {
        /// Mapping file, or - to read YAML from stdin
        mapping: PathBuf,
        /// Location where the generated YAML will be inserted
        #[arg(long, value_enum, default_value = "config")]
        target: mapping::RenderTarget,
    },
    /// Record the installer’s fixture and feedback checks as JSON
    Handover {
        /// Room mapping file (stdin is used for interview answers)
        mapping: PathBuf,
        /// Save the completed JSON report to a new file
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
}

fn run(command: Command) -> Result<u8, String> {
    let (output, status) = match command {
        Command::Decode { dpt, hex } => (codec_cli::Dpt::parse(&dpt)?.decode(&hex)?, 0),
        Command::Encode { dpt, value } => (codec_cli::Dpt::parse(&dpt)?.encode(&value)?, 0),
        Command::Replay {
            fixture,
            missing_final,
            connection_age,
        } => {
            let report = replay::run(fixture.as_deref(), missing_final, connection_age)?;
            (
                serde_json::to_string_pretty(&report).map_err(|_| "Cannot serialize replay.")?,
                0,
            )
        }
        Command::Guide => {
            let source = include_str!("../site/content/guide/_index.md").replace("\r\n", "\n");
            let (_, body) = source
                .split_once("\n+++\n")
                .ok_or("Cannot read the embedded field guide.")?;
            (
                format!("# KNX light handover guide\n\n{}", body.trim_start()),
                0,
            )
        }
        Command::Example { charging } => {
            let fixture = if charging {
                include_str!("../examples/charging.json")
            } else {
                include_str!("../examples/kitchen.yaml")
            };
            (fixture.to_owned(), 0)
        }
        Command::Validate { mapping } => {
            mapping::Room::read_input(&mapping)?;
            ("Valid switched-light mapping.".to_owned(), 0)
        }
        Command::Diff { before, after } => {
            if before.as_os_str() == "-" && after.as_os_str() == "-" {
                return Err("Use stdin for only one mapping. Supply a file for the other.".into());
            }
            let previous = mapping::Room::read_input(&before)
                .map_err(|err| format!("Previous mapping: {err}"))?;
            let proposed = mapping::Room::read_input(&after)
                .map_err(|err| format!("Proposed mapping: {err}"))?;
            let changes = previous.changes(&proposed);
            if changes.is_empty() {
                ("No mapping changes.".into(), 0)
            } else {
                (changes.join("\n"), 1)
            }
        }
        Command::Render { mapping, target } => {
            (mapping::Room::read_input(&mapping)?.render(target)?, 0)
        }
        Command::Handover { mapping, output } => {
            if mapping.as_os_str() == "-" {
                return Err(
                    "Handover reads interview answers from stdin. Supply a mapping file.".into(),
                );
            }
            let room = mapping::Room::read_file(&mapping)?;
            if let Some(path) = &output {
                handover::check_destination(path)?;
            }
            let (report, status) = handover::interview(&room)?;
            let report = serde_json::to_string_pretty(&report)
                .map_err(|_| "Cannot serialize handover record.")?;
            if let Some(path) = output {
                handover::save_report(&path, &report)?;
                eprintln!("Saved the completed handover report.");
                return Ok(status);
            }
            (report, status)
        }
    };
    writeln!(io::stdout().lock(), "{}", output.trim_end_matches('\n'))
        .map_err(|_| "Cannot write output.")?;
    Ok(status)
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
