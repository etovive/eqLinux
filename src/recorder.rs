//! WAV recording of the processed output.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::audio::{CHANNELS, SAMPLE_RATE};

pub struct Recorder {
    writer: WavWriter<BufWriter<File>>,
    path: PathBuf,
}

impl Recorder {
    pub fn new() -> Result<Self, String> {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("eqlinux")
            .join("recordings");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("eqlinux-{stamp}.wav"));
        let spec = WavSpec {
            channels: CHANNELS as u16,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let writer = WavWriter::create(&path, spec).map_err(|e| e.to_string())?;
        Ok(Self { writer, path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<(), String> {
        for sample in samples {
            self.writer
                .write_sample(*sample)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn finish(self) -> Result<PathBuf, String> {
        let path = self.path.clone();
        self.writer.finalize().map_err(|e| e.to_string())?;
        Ok(path)
    }
}
