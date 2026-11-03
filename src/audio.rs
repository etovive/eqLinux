//! Real-time audio engine: capture the virtual sink monitor, process it and
//! play it back to the selected physical output.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use arc_swap::ArcSwap;
use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::dsp::{
    gain_factors, EqChain, EqParams, SpatialMode, SpatialProcessor, SpectrumAnalyzer,
};
use crate::recorder::Recorder;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u8 = 2;
const FRAMES: usize = 480; // 10 ms

#[derive(Clone, Debug)]
pub struct AudioParams {
    pub enabled: bool,
    pub eq_enabled: bool,
    pub eq: EqParams,
    pub volume: f32,
    pub balance: f32,
    pub muted: bool,
}

impl Default for AudioParams {
    fn default() -> Self {
        Self {
            enabled: true,
            eq_enabled: true,
            eq: EqParams::default(),
            volume: 0.5,
            balance: 0.0,
            muted: false,
        }
    }
}

pub struct EngineHandle {
    shared: Arc<ArcSwap<AudioParams>>,
    spectrum: Arc<ArcSwap<[f32; 32]>>,
    status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    recorder: Arc<Mutex<Option<Recorder>>>,
    record_path: Arc<Mutex<Option<PathBuf>>>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn start(monitor: String, sink: String, params: AudioParams) -> Self {
        let shared = Arc::new(ArcSwap::from_pointee(params));
        let spectrum = Arc::new(ArcSwap::from_pointee([0.0f32; 32]));
        let status = Arc::new(Mutex::new(String::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let recorder = Arc::new(Mutex::new(None));
        let record_path = Arc::new(Mutex::new(None));

        let thread = {
            let shared = Arc::clone(&shared);
            let spectrum = Arc::clone(&spectrum);
            let status = Arc::clone(&status);
            let stop = Arc::clone(&stop);
            let recorder = Arc::clone(&recorder);

            thread::Builder::new()
                .name("eqlinux-audio".into())
                .spawn(move || audio_loop(monitor, sink, shared, spectrum, status, stop, recorder))
                .ok()
        };

        Self {
            shared,
            spectrum,
            status,
            stop,
            recorder,
            record_path,
            thread,
        }
    }

    pub fn update(&self, params: AudioParams) {
        self.shared.store(Arc::new(params));
    }

    pub fn spectrum(&self) -> [f32; 32] {
        **self.spectrum.load()
    }

    pub fn status(&self) -> String {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn start_recording(&self) -> Result<PathBuf, String> {
        self.stop_recording();
        let recorder = Recorder::new()?;
        let path = recorder.path().clone();
        *self.recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        *self.record_path.lock().map_err(|e| e.to_string())? = Some(path.clone());
        Ok(path)
    }

    pub fn stop_recording(&self) -> Option<PathBuf> {
        let recorder = self.recorder.lock().ok().and_then(|mut guard| guard.take());
        let path = self
            .record_path
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        if let Some(recorder) = recorder {
            match recorder.finish() {
                Ok(path) => return Some(path),
                Err(err) => {
                    set_status(&self.status, format!("Recording error: {err}"));
                }
            }
        }
        path
    }

    pub fn is_recording(&self) -> bool {
        self.recorder
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.stop_recording();
        self.stop();
    }
}

fn audio_loop(
    monitor: String,
    sink: String,
    shared: Arc<ArcSwap<AudioParams>>,
    spectrum: Arc<ArcSwap<[f32; 32]>>,
    status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    recorder: Arc<Mutex<Option<Recorder>>>,
) {
    let spec = Spec {
        format: Format::F32le,
        rate: SAMPLE_RATE,
        channels: CHANNELS,
    };

    let bytes_per_frame = CHANNELS as u32 * std::mem::size_of::<f32>() as u32;
    let frag_bytes = FRAMES as u32 * bytes_per_frame;

    let record_attr = BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: frag_bytes,
    };
    let playback_attr = BufferAttr {
        maxlength: u32::MAX,
        tlength: frag_bytes * 2,
        prebuf: frag_bytes,
        minreq: frag_bytes,
        fragsize: u32::MAX,
    };

    let capture = match Simple::new(
        None,
        "eqLinux",
        Direction::Record,
        Some(&monitor),
        "eqLinux system capture",
        &spec,
        None,
        Some(&record_attr),
    ) {
        Ok(capture) => capture,
        Err(err) => {
            set_status(&status, format!("Could not open capture stream: {err}"));
            return;
        }
    };

    let playback = match Simple::new(
        None,
        "eqLinux",
        Direction::Playback,
        Some(&sink),
        "eqLinux output",
        &spec,
        None,
        Some(&playback_attr),
    ) {
        Ok(playback) => playback,
        Err(err) => {
            set_status(
                &status,
                format!("Could not open playback stream on '{sink}': {err}"),
            );
            return;
        }
    };

    set_status(&status, format!("Running · {monitor} → {sink}"));

    let mut samples = vec![0.0f32; FRAMES * CHANNELS as usize];
    let mut chain: Option<EqChain> = None;
    let mut last_eq: Option<EqParams> = None;
    let mut spatial = SpatialProcessor::new(SAMPLE_RATE as f32, SpatialMode::Off, 0.5);
    let mut last_spatial: Option<(SpatialMode, f32)> = None;
    let mut analyzer = SpectrumAnalyzer::new(SAMPLE_RATE as f32, 2048);

    while !stop.load(Ordering::Relaxed) {
        let bytes = bytemuck::cast_slice_mut::<f32, u8>(&mut samples);
        if let Err(err) = capture.read(bytes) {
            set_status(&status, format!("Capture error: {err}"));
            break;
        }

        let params = shared.load_full();

        if params.enabled {
            if last_spatial != Some((params.eq.spatial, params.eq.spatial_mix)) {
                spatial.update(SAMPLE_RATE as f32, params.eq.spatial, params.eq.spatial_mix);
                last_spatial = Some((params.eq.spatial, params.eq.spatial_mix));
            }
            if params.eq_enabled {
                match chain.as_mut() {
                    Some(eq) => {
                        if last_eq.as_ref() != Some(&params.eq) {
                            eq.update(SAMPLE_RATE as f64, &params.eq);
                            last_eq = Some(params.eq.clone());
                        }
                    }
                    None => {
                        chain = Some(EqChain::new(SAMPLE_RATE as f64, &params.eq));
                        last_eq = Some(params.eq.clone());
                    }
                }
            }

            let (left_gain, right_gain) = gain_factors(params.volume, params.balance, params.muted);
            let eq_enabled = params.eq_enabled;

            for frame in samples.as_chunks_mut::<2>().0 {
                let (mut left, mut right) = (frame[0], frame[1]);
                if eq_enabled {
                    if let Some(eq) = chain.as_mut() {
                        let processed = eq.process(left, right);
                        left = processed.0;
                        right = processed.1;
                    }
                }
                let processed = spatial.process(left, right);
                left = processed.0;
                right = processed.1;
                frame[0] = left * left_gain;
                frame[1] = right * right_gain;
            }
        }

        analyzer.push_stereo(&samples);
        spectrum.store(Arc::new(analyzer.smoothed_bands()));

        if let Ok(mut guard) = recorder.lock() {
            if let Some(rec) = guard.as_mut() {
                if let Err(err) = rec.write(&samples) {
                    set_status(&status, format!("Recording error: {err}"));
                    *guard = None;
                }
            }
        }

        if let Err(err) = playback.write(bytemuck::cast_slice(&samples)) {
            set_status(&status, format!("Playback error: {err}"));
            break;
        }
    }
}

fn set_status(status: &Mutex<String>, message: String) {
    if let Ok(mut guard) = status.lock() {
        *guard = message;
    }
}
