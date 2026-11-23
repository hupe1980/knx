use knx::PowerMeter;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::File, io::Read, path::Path};

const REQUEST_AT: i64 = 1795335120;
const SNAPSHOT_AT: i64 = REQUEST_AT + 180;
const CONNECTION_MAX_AGE: u32 = 30;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    site: String,
    group: String,
    dpt: String,
    payload: Vec<u8>,
    observed_at: Option<i64>,
    received_at: i64,
}

pub fn run(path: Option<&Path>, missing_final: bool, connection_age: u32) -> Result<Value, String> {
    const MAX_BYTES: u64 = 16 * 1024;
    let reader: Box<dyn Read> = match path {
        None => Box::new(&include_bytes!("../examples/charging.json")[..]),
        Some(p) if p == Path::new("-") => Box::new(std::io::stdin().lock()),
        Some(p) if p.is_file() => {
            Box::new(File::open(p).map_err(|_| "Select a readable JSON fixture.")?)
        }
        Some(_) => return Err("Select a regular JSON fixture file.".into()),
    };
    let mut bytes = Vec::new();
    reader
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read the charging fixture.")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Limit the fixture to 16 KiB.".into());
    }
    let mut samples: Vec<Sample> = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "Check the charging fixture JSON at line {}, column {} against the example.",
            e.line(),
            e.column()
        )
    })?;
    if missing_final {
        samples.pop();
    }
    let mut meter = PowerMeter::new("west", "2/1/3", 60000.0).map_err(|e| e.to_string())?;
    let mut readings = Vec::new();
    let mut confirmed = false;
    let mut snapshot_charging_w = None;
    for (i, s) in samples.iter().enumerate() {
        if s.received_at > SNAPSHOT_AT {
            return Err(format!(
                "Charging sample {} arrives after the replay clock.",
                i + 1
            ));
        }
        let reading = meter
            .observe(
                &s.site,
                s.group.as_str(),
                &s.dpt,
                &s.payload,
                s.observed_at,
                s.received_at,
            )
            .map_err(|e| format!("Charging sample {}: {e}", i + 1))?;
        readings
            .push(json!({"observed_at":reading.observed_at,"received_at":s.received_at,"watts":reading.watts}));
        if reading.observed_at > REQUEST_AT && reading.observed_at <= REQUEST_AT + 15 {
            confirmed = reading.watts <= 24000.0;
        }
        if reading.observed_at == SNAPSHOT_AT {
            snapshot_charging_w = Some(reading.watts);
        }
    }
    Ok(
        json!({"synthetic":true,"site":"west","asset":"charging-bank","snapshot_at":SNAPSHOT_AT,
        "request":{"at":REQUEST_AT,"watts":24000},"readings":readings,"snapshot_charging_w":snapshot_charging_w,
        "delivery":if confirmed {"confirmed"}else{"unconfirmed"},
        "connection":{"observed_at":SNAPSHOT_AT-i64::from(connection_age),"import_w":52000,"limit_w":60000,
        "headroom_w":(connection_age<=CONNECTION_MAX_AGE).then_some(8000)} }),
    )
}
