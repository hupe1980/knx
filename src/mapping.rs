use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

const MAX_MAPPING_BYTES: usize = 16 * 1024;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Point {
    pub address: String,
    dpt: String,
    // Keep the entered notation for the report and compare groups by numeric identity.
    #[serde(skip)]
    group: Option<GroupAddress>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Room {
    pub room: String,
    pub light: String,
    pub command: Point,
    pub feedback: Point,
}

use knx::GroupAddress;

impl Point {
    fn three_level(&self) -> String {
        self.group.expect("validated room address").to_string()
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum RenderTarget {
    /// Top-level configuration.yaml snippet (starts with knx:)
    Config,
    /// Contents of the knx: section (starts with light:)
    Knx,
    /// Entries in the knx.light list
    Lights,
}

impl Room {
    pub fn read_input(path: &Path) -> Result<Self, String> {
        if path == Path::new("-") {
            Self::decode(io::stdin().lock())
        } else {
            Self::read_file(path)
        }
    }

    pub fn read_file(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Err("Select a readable regular room mapping file.".into());
        }
        let file = File::open(path).map_err(|_| "Cannot open mapping. Check permissions.")?;
        Self::decode(file)
    }

    fn decode(reader: impl Read) -> Result<Self, String> {
        let mut data = Vec::new();
        reader
            .take(MAX_MAPPING_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| "Cannot read the room mapping.")?;
        if data.len() > MAX_MAPPING_BYTES {
            return Err("Limit the room mapping to 16 KiB.".into());
        }
        let text = std::str::from_utf8(&data).map_err(|_| "Save the mapping as UTF-8 YAML.")?;
        let mut options = serde_saphyr::Options::default();
        options.reject_unsupported_tags = true;
        options.reject_tagged_field_names = true;
        if let Some(budget) = &mut options.budget {
            budget.max_depth = 4;
            budget.max_documents = 1;
            budget.max_aliases = 0;
            budget.max_anchors = 0;
        }
        let room: Self = serde_saphyr::from_str_with_options(text, options)
            .map_err(|error| {
                let position = error.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
                format!("Invalid room YAML{position}. Check field names, duplicate keys, nesting and tags against the example.")
            })?;
        room.validate()
    }

    fn validate(mut self) -> Result<Self, String> {
        let mut issues = Vec::new();
        for (field, label) in [("room", &self.room), ("light", &self.light)] {
            if label.trim().is_empty()
                || label.chars().count() > 120
                || label.chars().any(|c| c.is_control() && c != '\n')
            {
                issues.push(format!(
                    "{field} needs a printable label of 1 to 120 characters."
                ));
            }
        }
        for (field, point) in [
            ("command", &mut self.command),
            ("feedback", &mut self.feedback),
        ] {
            point.group = point.address.parse().ok();
            if point.group.is_none() {
                issues.push(format!("{field}.address must be a nonzero ETS group address. Use main/middle/sub (31/7/255 max), main/sub (31/2047 max), or decimal (65535 max), without leading zeros."));
            }
            if point.dpt != "1.001" {
                issues.push(format!("{field}.dpt must be 1.001. Check the group address DPT in ETS or its linked group objects if unassigned."));
            }
        }
        if self.command.group.is_some() && self.command.group == self.feedback.group {
            issues.push(
                "feedback.address needs an independent actuator status address for this profile."
                    .into(),
            );
        }
        if issues.is_empty() {
            Ok(self)
        } else {
            Err(issues.join("\n"))
        }
    }

    pub fn name(&self) -> String {
        format!("{} {}", self.room, self.light)
    }

    pub fn changes(&self, next: &Self) -> Vec<String> {
        let mut changes = Vec::new();
        if self.room != next.room {
            changes.push("Room label changed.".into());
        }
        if self.light != next.light {
            changes.push("Light label changed.".into());
        }
        for (role, old, new) in [
            ("Command", &self.command, &next.command),
            ("Feedback", &self.feedback, &next.feedback),
        ] {
            if old.group != new.group {
                changes.push(format!(
                    "{role}: {} -> {}",
                    old.three_level(),
                    new.three_level()
                ));
            }
        }
        if self.command.group == next.feedback.group && self.feedback.group == next.command.group {
            changes.push(
                "Command and feedback exchanged roles. Check both actuator objects in ETS.".into(),
            );
        }
        changes
    }

    pub fn render(&self, target: RenderTarget) -> Result<String, String> {
        let lights = serde_json::json!([{"name": self.name(), "address": self.command.three_level(),
            "state_address": self.feedback.three_level()}]);
        let config = match target {
            RenderTarget::Config => serde_json::json!({"knx": {"light": lights}}),
            RenderTarget::Knx => serde_json::json!({"light": lights}),
            RenderTarget::Lights => lights,
        };
        serde_saphyr::to_string(&config)
            .map_err(|_| "Could not serialize light configuration.".into())
    }
}
