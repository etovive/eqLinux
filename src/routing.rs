//! System-wide audio routing through a PulseAudio/PipeWire null sink.
//!
//! This mirrors eqMac's virtual driver: all applications are moved to a
//! virtual output, eqLinux captures its monitor, processes the stream and
//! plays it back to the real output device.

use std::process::Command;

use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct SinkInfo {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
struct RawSink {
    name: String,
    description: String,
}

fn run(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("could not run pactl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "pactl {} failed: {}",
            args.join(" "),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn list_sinks() -> Result<Vec<SinkInfo>, String> {
    let text = run(&["-f", "json", "list", "sinks"])?;
    let raw: Vec<RawSink> =
        serde_json::from_str(&text).map_err(|e| format!("could not parse sink list: {e}"))?;

    Ok(raw
        .into_iter()
        .map(|s| SinkInfo {
            name: s.name,
            description: s.description,
        })
        .collect())
}

pub fn default_sink() -> Option<String> {
    run(&["get-default-sink"]).ok().filter(|s| !s.is_empty())
}

/// True for sinks created by eqLinux, including stale ones from a crash.
pub fn is_virtual_sink(name: &str) -> bool {
    name == "eqlinux_sink" || name.starts_with("eqlinux_")
}

pub fn set_default_sink(name: &str) -> Result<(), String> {
    run(&["set-default-sink", name]).map(|_| ())
}

fn find_module(sink_name: &str) -> Option<u32> {
    let text = run(&["list", "short", "modules"]).ok()?;
    for line in text.lines() {
        let mut parts = line.split('\t');
        let Some(index) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let _name = parts.next();
        let args = parts.next().unwrap_or("");
        if args.contains(&format!("sink_name={sink_name}")) {
            return Some(index);
        }
    }
    None
}

fn unload_module(index: u32) {
    let _ = run(&["unload-module", &index.to_string()]);
}

fn load_null_sink(sink_name: &str) -> Result<u32, String> {
    let property = "sink_properties=device.description=eqLinux".to_string();
    let sink_arg = format!("sink_name={sink_name}");
    let text = run(&["load-module", "module-null-sink", &sink_arg, &property])?;
    text.parse::<u32>()
        .map_err(|e| format!("unexpected module id from pactl: {e}"))
}

pub fn move_sink_inputs(destination: &str) {
    let Ok(text) = run(&["-f", "json", "list", "short", "sink-inputs"]) else {
        return;
    };
    let Ok(inputs) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
        return;
    };

    for input in inputs {
        if let Some(index) = input.get("index").and_then(|v| v.as_u64()) {
            let _ = run(&["move-sink-input", &index.to_string(), destination]);
        }
    }
}

/// Owns the lifecycle of the virtual sink.  When dropped (or `stop`ped) the
/// user's previous default output is restored.
pub struct Routing {
    module_id: u32,
    original_default: Option<String>,
    virtual_sink: String,
    monitor_source: String,
    active: bool,
}

impl Routing {
    pub fn start(sink_name: &str) -> Result<Self, String> {
        let mut original = default_sink().filter(|s| s != sink_name && !is_virtual_sink(s));

        if original.is_none() {
            original = list_sinks().ok().and_then(|sinks| {
                sinks
                    .into_iter()
                    .find(|s| s.name != sink_name && !is_virtual_sink(&s.name))
                    .map(|s| s.name)
            });
        }

        // Clean up a stale virtual sink left behind by a previous crash.
        if let Some(module) = find_module(sink_name) {
            unload_module(module);
        }

        let module_id = load_null_sink(sink_name)?;
        if let Err(err) = set_default_sink(sink_name) {
            unload_module(module_id);
            return Err(err);
        }
        move_sink_inputs(sink_name);

        Ok(Self {
            module_id,
            original_default: original,
            virtual_sink: sink_name.to_string(),
            monitor_source: format!("{sink_name}.monitor"),
            active: true,
        })
    }

    pub fn monitor_source(&self) -> &str {
        &self.monitor_source
    }

    pub fn stop(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;

        if let Some(original) = &self.original_default {
            if original != &self.virtual_sink {
                let _ = set_default_sink(original);
            }
        }

        unload_module(self.module_id);
    }
}

impl Drop for Routing {
    fn drop(&mut self) {
        self.stop();
    }
}
