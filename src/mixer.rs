//! Per-application volume control through the PipeWire/PulseAudio mixer.

use std::process::Command;

use serde_json::Value;

#[derive(Clone, Debug)]
pub struct SinkInput {
    pub index: u32,
    pub app: String,
    pub media: String,
    pub volume: f32,
    pub muted: bool,
}

fn run(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("could not run pactl: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn list() -> Result<Vec<SinkInput>, String> {
    let text = run(&["-f", "json", "list", "sink-inputs"])?;
    let raw: Vec<Value> =
        serde_json::from_str(&text).map_err(|e| format!("could not parse mixer: {e}"))?;

    Ok(raw
        .into_iter()
        .filter_map(|input| {
            let index = input.get("index")?.as_u64()? as u32;
            let properties = input.get("properties");
            let property = |key: &str| {
                properties
                    .and_then(|p| p.get(key))
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
            };
            let app = property("application.name")
                .or_else(|| property("application.process.binary"))
                .or_else(|| property("application.id"))
                .unwrap_or("Unknown application")
                .to_string();
            let media = property("media.name").unwrap_or("").to_string();
            let muted = input.get("mute").and_then(Value::as_bool).unwrap_or(false);
            let volume = input
                .get("volume")
                .and_then(Value::as_object)
                .and_then(|channels| channels.values().next())
                .and_then(|channel| channel.get("value"))
                .and_then(Value::as_f64)
                .map(|value| (value / 65_536.0) as f32)
                .unwrap_or(1.0);
            Some(SinkInput {
                index,
                app,
                media,
                volume,
                muted,
            })
        })
        .collect())
}

pub fn set_volume(index: u32, volume: f32) -> Result<(), String> {
    let percent = (volume.clamp(0.0, 1.5) * 100.0).round() as u32;
    let index = index.to_string();
    run(&["set-sink-input-volume", &index, &format!("{percent}%")]).map(|_| ())
}

pub fn set_mute(index: u32, muted: bool) -> Result<(), String> {
    let index = index.to_string();
    run(&["set-sink-input-mute", &index, if muted { "1" } else { "0" }]).map(|_| ())
}
