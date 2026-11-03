//! Digital signal processing for eqLinux.
//!
//! The original eqMac uses `AVAudioUnitEQ` with ten parametric bands at the
//! exact frequencies below and a constant bandwidth of 0.5 octaves.  The same
//! topology is reproduced here with RBJ peaking biquads.

use std::f64::consts::PI;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};

/// Centre frequencies used by eqMac (Basic and Advanced equalizers).
pub const FREQUENCIES: [f64; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

/// eqMac sets `bandwidth = 0.5` (octaves) on every band.
pub const BANDWIDTH: f64 = 0.5;

pub const MIN_GAIN: f64 = -24.0;
pub const MAX_GAIN: f64 = 24.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EqMode {
    #[default]
    Basic,
    Advanced,
    Expert,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    #[default]
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

impl FilterType {
    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Peaking, false) => "Peaking",
            (Self::Peaking, true) => "Campana",
            (Self::LowShelf, false) => "Low Shelf",
            (Self::LowShelf, true) => "Grave estante",
            (Self::HighShelf, false) => "High Shelf",
            (Self::HighShelf, true) => "Agudo estante",
            (Self::LowPass, false) => "Low Pass",
            (Self::LowPass, true) => "Paso bajo",
            (Self::HighPass, false) => "High Pass",
            (Self::HighPass, true) => "Paso alto",
            (Self::BandPass, false) => "Band Pass",
            (Self::BandPass, true) => "Paso banda",
            (Self::Notch, false) => "Notch",
            (Self::Notch, true) => "Muesca",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Both,
    Left,
    Right,
}

impl Channel {
    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Both, false) => "Stereo",
            (Self::Both, true) => "Estéreo",
            (Self::Left, false) => "Left",
            (Self::Left, true) => "Izq.",
            (Self::Right, false) => "Right",
            (Self::Right, true) => "Der.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpertBand {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency: f64,
    pub gain: f64,
    pub q: f64,
    pub channel: Channel,
}

impl Default for ExpertBand {
    fn default() -> Self {
        Self {
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency: 1_000.0,
            gain: 0.0,
            q: 1.0,
            channel: Channel::Both,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpatialMode {
    #[default]
    Off,
    SmallRoom,
    LargeHall,
    Concert,
    Headphones3D,
}

impl SpatialMode {
    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Off, false) => "Off",
            (Self::Off, true) => "Desactivado",
            (Self::SmallRoom, false) => "Small room",
            (Self::SmallRoom, true) => "Sala pequeña",
            (Self::LargeHall, false) => "Large hall",
            (Self::LargeHall, true) => "Gran sala",
            (Self::Concert, false) => "Concert",
            (Self::Concert, true) => "Concierto",
            (Self::Headphones3D, false) => "Headphone 3D (experimental)",
            (Self::Headphones3D, true) => "3D para auriculares (experimental)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterWidth {
    Octaves(f64),
    Q(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedBand {
    pub filter_type: FilterType,
    pub frequency: f64,
    pub gain: f64,
    pub width: FilterWidth,
    pub channel: Channel,
}

/// Everything the audio thread needs to know about the equalizer.
#[derive(Clone, Debug, PartialEq)]
pub struct EqParams {
    pub mode: EqMode,
    pub bass: f64,
    pub mid: f64,
    pub treble: f64,
    pub peak_limiter: bool,
    pub advanced_global: f64,
    pub advanced_bands: [f64; 10],
    pub expert_global: f64,
    pub expert_peak_limiter: bool,
    pub expert_bands: Vec<ExpertBand>,
    pub left_trim: f64,
    pub right_trim: f64,
    pub spatial: SpatialMode,
    pub spatial_mix: f32,
}

impl Default for EqParams {
    fn default() -> Self {
        Self {
            mode: EqMode::Basic,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            peak_limiter: false,
            advanced_global: 0.0,
            advanced_bands: [0.0; 10],
            expert_global: 0.0,
            expert_peak_limiter: true,
            expert_bands: Vec::new(),
            left_trim: 0.0,
            right_trim: 0.0,
            spatial: SpatialMode::Off,
            spatial_mix: 0.5,
        }
    }
}

impl EqParams {
    /// Expands the current mode into the filter bands plus the global gain
    /// (in dB), mirroring eqMac's Basic / Advanced / Expert equalizers.
    pub fn resolved(&self) -> (Vec<ResolvedBand>, f64) {
        match self.mode {
            EqMode::Basic => {
                let bands = [
                    self.bass,
                    self.bass,
                    self.bass,
                    self.mid,
                    self.mid,
                    self.mid,
                    self.mid,
                    self.treble,
                    self.treble,
                    self.treble,
                ];
                let global = if self.peak_limiter {
                    peak_offset(&bands)
                } else {
                    0.0
                };
                (
                    bands
                        .iter()
                        .enumerate()
                        .map(|(index, gain)| ResolvedBand {
                            filter_type: FilterType::Peaking,
                            frequency: FREQUENCIES[index],
                            gain: *gain,
                            width: FilterWidth::Octaves(BANDWIDTH),
                            channel: Channel::Both,
                        })
                        .collect(),
                    global,
                )
            }
            EqMode::Advanced => (
                self.advanced_bands
                    .iter()
                    .enumerate()
                    .map(|(index, gain)| ResolvedBand {
                        filter_type: FilterType::Peaking,
                        frequency: FREQUENCIES[index],
                        gain: *gain,
                        width: FilterWidth::Octaves(BANDWIDTH),
                        channel: Channel::Both,
                    })
                    .collect(),
                self.advanced_global,
            ),
            EqMode::Expert => {
                let mut gains = Vec::new();
                let bands = self
                    .expert_bands
                    .iter()
                    .filter(|band| band.enabled)
                    .map(|band| {
                        if band.gain > 0.0 {
                            gains.push(band.gain);
                        }
                        ResolvedBand {
                            filter_type: band.filter_type,
                            frequency: band.frequency.clamp(10.0, 22_000.0),
                            gain: band.gain,
                            width: FilterWidth::Q(band.q.clamp(0.05, 30.0)),
                            channel: band.channel,
                        }
                    })
                    .collect();
                let limiter = if self.expert_peak_limiter {
                    peak_offset(&gains)
                } else {
                    0.0
                };
                (bands, self.expert_global + limiter)
            }
        }
    }
}

fn peak_offset(gains: &[f64]) -> f64 {
    let highest = gains.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if highest > 0.0 {
        -highest
    } else {
        0.0
    }
}

/// A single transposed direct-form II biquad.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self::identity()
    }
}

impl Biquad {
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Updates the coefficients in place, keeping the filter history.  This
    /// avoids clicks while a knob is being dragged.
    pub fn set_filter(
        &mut self,
        sample_rate: f64,
        filter_type: FilterType,
        freq: f64,
        gain_db: f64,
        width: FilterWidth,
    ) {
        let freq = freq.clamp(10.0, sample_rate * 0.49);
        let q = match width {
            FilterWidth::Q(q) => q.max(0.05),
            FilterWidth::Octaves(octaves) => {
                let factor = 2f64.powf(octaves.max(0.01));
                factor.sqrt() / (factor - 1.0)
            }
        };
        // Gain-less filters are always active; peaking/shelves bypass at 0 dB.
        let bypass = matches!(
            filter_type,
            FilterType::Peaking | FilterType::LowShelf | FilterType::HighShelf
        ) && gain_db.abs() < 1e-9;
        if bypass {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::LowPass => (
                (1.0 - cos_w0) * 0.5,
                1.0 - cos_w0,
                (1.0 - cos_w0) * 0.5,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos_w0) * 0.5,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) * 0.5,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            FilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        };

        self.b0 = (b0 / a0) as f32;
        self.b1 = (b1 / a0) as f32;
        self.b2 = (b2 / a0) as f32;
        self.a1 = (a1 / a0) as f32;
        self.a2 = (a2 / a0) as f32;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// A stereo chain of parametric filters plus the global gain stage.
pub struct EqChain {
    left: Vec<Biquad>,
    right: Vec<Biquad>,
    global: f32,
    left_trim: f32,
    right_trim: f32,
}

impl EqChain {
    pub fn new(sample_rate: f64, params: &EqParams) -> Self {
        let mut chain = Self {
            left: Vec::new(),
            right: Vec::new(),
            global: 1.0,
            left_trim: 1.0,
            right_trim: 1.0,
        };
        chain.update(sample_rate, params);
        chain
    }

    pub fn update(&mut self, sample_rate: f64, params: &EqParams) {
        let (bands, global_db) = params.resolved();
        let left: Vec<&ResolvedBand> = bands
            .iter()
            .filter(|band| matches!(band.channel, Channel::Both | Channel::Left))
            .collect();
        let right: Vec<&ResolvedBand> = bands
            .iter()
            .filter(|band| matches!(band.channel, Channel::Both | Channel::Right))
            .collect();

        update_filters(&mut self.left, sample_rate, &left);
        update_filters(&mut self.right, sample_rate, &right);
        self.global = 10f32.powf(global_db as f32 / 20.0);
        self.left_trim = 10f32.powf(params.left_trim as f32 / 20.0);
        self.right_trim = 10f32.powf(params.right_trim as f32 / 20.0);
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let mut left = l;
        let mut right = r;
        for filter in &mut self.left {
            left = filter.process(left);
        }
        for filter in &mut self.right {
            right = filter.process(right);
        }
        (
            left * self.global * self.left_trim,
            right * self.global * self.right_trim,
        )
    }
}

fn update_filters(filters: &mut Vec<Biquad>, sample_rate: f64, bands: &[&ResolvedBand]) {
    for (index, band) in bands.iter().enumerate() {
        if index >= filters.len() {
            filters.push(Biquad::identity());
        }
        filters[index].set_filter(
            sample_rate,
            band.filter_type,
            band.frequency,
            band.gain,
            band.width,
        );
    }
    filters.truncate(bands.len());
}

/// A lightweight spatial effect based on short stereo delays and damping.
pub struct SpatialProcessor {
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    position: usize,
    delay_frames: usize,
    feedback: f32,
    cross: f32,
    mix: f32,
    mode: SpatialMode,
    damp_l: f32,
    damp_r: f32,
    crossfeed_l: Vec<f32>,
    crossfeed_r: Vec<f32>,
    crossfeed_position: usize,
    crossfeed_alpha: f32,
}

impl SpatialProcessor {
    pub fn new(sample_rate: f32, mode: SpatialMode, mix: f32) -> Self {
        let mut processor = Self {
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            position: 0,
            delay_frames: 0,
            feedback: 0.0,
            cross: 0.0,
            mix: 0.0,
            mode: SpatialMode::Off,
            damp_l: 0.0,
            damp_r: 0.0,
            crossfeed_l: Vec::new(),
            crossfeed_r: Vec::new(),
            crossfeed_position: 0,
            crossfeed_alpha: 0.0,
        };
        processor.update(sample_rate, mode, mix);
        processor
    }

    pub fn update(&mut self, sample_rate: f32, mode: SpatialMode, mix: f32) {
        self.mode = mode;
        let (ms, feedback, cross, base_mix) = match mode {
            SpatialMode::Off => (1.0, 0.0, 0.0, 0.0),
            SpatialMode::SmallRoom => (17.0, 0.28, 0.18, 0.20),
            SpatialMode::LargeHall => (37.0, 0.48, 0.28, 0.32),
            SpatialMode::Concert => (67.0, 0.62, 0.38, 0.45),
            SpatialMode::Headphones3D => (12.0, 0.0, 0.0, 1.0),
        };
        self.delay_frames = ((ms / 1000.0) * sample_rate).round() as usize + 1;
        if self.delay_l.len() != self.delay_frames {
            self.delay_l = vec![0.0; self.delay_frames];
            self.delay_r = vec![0.0; self.delay_frames];
            self.position = 0;
        }
        self.feedback = feedback;
        self.cross = cross;
        let mix = mix.clamp(0.0, 1.0);
        self.mix = if mode == SpatialMode::Headphones3D && mix > 0.0 {
            base_mix * (0.4 + mix * 0.6)
        } else {
            base_mix * mix
        };
        let crossfeed_frames = ((sample_rate * 0.0007).round() as usize).max(1);
        if self.crossfeed_l.len() != crossfeed_frames {
            self.crossfeed_l = vec![0.0; crossfeed_frames];
            self.crossfeed_r = vec![0.0; crossfeed_frames];
            self.crossfeed_position = 0;
        }
        self.crossfeed_alpha = 1.0 - (-2.0 * std::f32::consts::PI * 2_000.0 / sample_rate).exp();
        self.damp_l = 0.0;
        self.damp_r = 0.0;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.mode == SpatialMode::Off || self.delay_frames == 0 {
            return (l, r);
        }

        if self.mode == SpatialMode::Headphones3D {
            let cross_l = self.crossfeed_l[self.crossfeed_position];
            let cross_r = self.crossfeed_r[self.crossfeed_position];
            self.crossfeed_l[self.crossfeed_position] = l;
            self.crossfeed_r[self.crossfeed_position] = r;
            self.crossfeed_position = (self.crossfeed_position + 1) % self.crossfeed_l.len();

            self.damp_l += (cross_r - self.damp_l) * self.crossfeed_alpha;
            self.damp_r += (cross_l - self.damp_r) * self.crossfeed_alpha;

            let reflection_l_delay = ((self.delay_frames - 1) * 7 / 12).max(1);
            let reflection_r_delay = ((self.delay_frames - 1) * 11 / 12).max(1);
            let reflection_l = self.delay_r
                [(self.position + self.delay_frames - reflection_l_delay) % self.delay_frames];
            let reflection_r = self.delay_l
                [(self.position + self.delay_frames - reflection_r_delay) % self.delay_frames];
            self.delay_l[self.position] = l;
            self.delay_r[self.position] = r;
            self.position = (self.position + 1) % self.delay_frames;

            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5;
            let width = 1.0 + self.mix * 1.8;
            let wet_l = mid * 0.68
                + side * width
                + self.damp_l * self.mix * 0.28
                + reflection_l * self.mix * 0.38
                + reflection_r * self.mix * 0.12;
            let wet_r = mid * 0.68 - side * width
                + self.damp_r * self.mix * 0.28
                + reflection_r * self.mix * 0.38
                + reflection_l * self.mix * 0.12;
            return (l * (1.0 - self.mix) + wet_l, r * (1.0 - self.mix) + wet_r);
        }

        let read = (self.position + self.delay_frames - self.delay_frames) % self.delay_frames;
        let (delayed_l, delayed_r) = (self.delay_l[read], self.delay_r[read]);
        let wet_l = delayed_l + delayed_r * self.cross;
        let wet_r = delayed_r + delayed_l * self.cross;
        self.damp_l = self.damp_l * 0.62 + wet_l * 0.38;
        self.damp_r = self.damp_r * 0.62 + wet_r * 0.38;

        self.delay_l[self.position] = (l + self.damp_l * self.feedback).clamp(-1.0, 1.0);
        self.delay_r[self.position] = (r + self.damp_r * self.feedback).clamp(-1.0, 1.0);
        self.position = (self.position + 1) % self.delay_frames;

        let dry = 1.0 - self.mix;
        (l * dry + wet_l * self.mix, r * dry + wet_r * self.mix)
    }
}

/// Converts eqMac's 0..2 volume slider and -1..1 balance into per-channel
/// linear factors.  Above 1.0 the boost curve is the same as eqMac's
/// `remap(1..2 -> 1..6)`.
pub fn gain_factors(volume: f32, balance: f32, muted: bool) -> (f32, f32) {
    if muted {
        return (0.0, 0.0);
    }

    let amp = if volume <= 1.0 {
        volume.max(0.0)
    } else {
        1.0 + (volume - 1.0) * 5.0
    };

    let balance = balance.clamp(-1.0, 1.0);
    if balance > 0.0 {
        (amp * (1.0 - balance), amp)
    } else if balance < 0.0 {
        (amp, amp * (1.0 + balance))
    } else {
        (amp, amp)
    }
}

/// A small log-spaced spectrum analyser used by the optional visualiser.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    fft_len: usize,
    window: Vec<f32>,
    samples: Vec<f32>,
    position: usize,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    bands: [f32; 32],
    sample_rate: f32,
    smoothed: [f32; 32],
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: f32, fft_len: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_len);
        let scratch_len = fft.get_inplace_scratch_len();

        let window = (0..fft_len)
            .map(|n| {
                let x = 2.0 * std::f32::consts::PI * n as f32 / (fft_len - 1) as f32;
                0.5 * (1.0 - x.cos())
            })
            .collect();

        Self {
            fft,
            fft_len,
            window,
            samples: vec![0.0; fft_len],
            position: 0,
            buffer: vec![Complex::new(0.0, 0.0); fft_len],
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            bands: [0.0; 32],
            sample_rate,
            smoothed: [0.0; 32],
        }
    }

    pub fn push_stereo(&mut self, frames: &[f32]) {
        for frame in frames.as_chunks::<2>().0 {
            self.samples[self.position] = (frame[0] + frame[1]) * 0.5;
            self.position += 1;
            if self.position == self.fft_len {
                self.position = 0;
                self.compute();
            }
        }
    }

    fn compute(&mut self) {
        for (i, sample) in self.samples.iter().enumerate() {
            self.buffer[i] = Complex::new(sample * self.window[i], 0.0);
        }

        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);

        let bin_hz = self.sample_rate / self.fft_len as f32;
        let min_hz = 20.0f32;
        let max_hz = 20_000.0f32;
        let log_min = min_hz.ln();
        let log_max = max_hz.ln();

        let band_count = self.bands.len() as f32;
        for (band, value) in self.bands.iter_mut().enumerate() {
            let t0 = band as f32 / band_count;
            let t1 = (band + 1) as f32 / band_count;
            let f0 = (log_min + (log_max - log_min) * t0).exp();
            let f1 = (log_min + (log_max - log_min) * t1).exp();
            let b0 = ((f0 / bin_hz).floor() as usize).max(1);
            let b1 = ((f1 / bin_hz).ceil() as usize).min(self.fft_len / 2);
            let mut peak = 0.0f32;
            for bin in b0..=b1.max(b0) {
                if bin < self.buffer.len() {
                    let c = self.buffer[bin];
                    peak = peak.max((c.re * c.re + c.im * c.im).sqrt());
                }
            }
            let magnitude = (peak * 2.0 / self.fft_len as f32).max(1e-9);
            let db = 20.0 * magnitude.log10();
            *value = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
        }

        for i in 0..self.bands.len() {
            let target = self.bands[i];
            let current = self.smoothed[i];
            let coeff = if target > current { 0.65 } else { 0.16 };
            self.smoothed[i] = current + (target - current) * coeff;
        }
    }

    pub fn smoothed_bands(&self) -> [f32; 32] {
        self.smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_when_gain_is_zero() {
        let params = EqParams::default();
        let mut chain = EqChain::new(48_000.0, &params);
        let (l, r) = chain.process(0.25, -0.5);
        assert!((l - 0.25).abs() < 1e-6);
        assert!((r + 0.5).abs() < 1e-6);
    }

    #[test]
    fn boost_increases_low_frequency_energy() {
        let params = EqParams {
            bass: 12.0,
            ..Default::default()
        };
        let mut chain = EqChain::new(48_000.0, &params);

        let mut sum = 0.0;
        for n in 0..48_000 {
            let t = n as f32 / 48_000.0;
            let x = (2.0 * std::f32::consts::PI * 32.0 * t).sin() * 0.25;
            let (y, _) = chain.process(x, x);
            if n > 4_800 {
                sum += y.abs();
            }
        }
        assert!(sum > 0.25 * 43_200.0 * 0.5);
    }

    #[test]
    fn peak_limiter_offsets_positive_gain() {
        let params = EqParams {
            bass: 6.0,
            peak_limiter: true,
            ..Default::default()
        };
        let (bands, global) = params.resolved();
        assert_eq!(bands[0].gain, 6.0);
        assert_eq!(global, -6.0);
    }

    #[test]
    fn volume_boost_matches_eqmac_curve() {
        let (l, r) = gain_factors(1.0, 0.0, false);
        assert!((l - 1.0).abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
        let (l, r) = gain_factors(2.0, 0.0, false);
        assert!((l - 6.0).abs() < 1e-6 && (r - 6.0).abs() < 1e-6);
        let (l, r) = gain_factors(1.0, 1.0, false);
        assert!((l - 0.0).abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
    }

    #[test]
    fn headphones_3d_preserves_off_mode_and_produces_finite_output() {
        let mut off = SpatialProcessor::new(48_000.0, SpatialMode::Off, 1.0);
        assert_eq!(off.process(0.25, -0.5), (0.25, -0.5));

        let mut spatial = SpatialProcessor::new(48_000.0, SpatialMode::Headphones3D, 0.5);
        let mut max_delta = 0.0f32;
        for frame in 0..2_000 {
            let input = if frame == 0 { 0.5 } else { 0.0 };
            let (left, right) = spatial.process(input, input);
            assert!(left.is_finite() && right.is_finite());
            max_delta = max_delta
                .max((left - input).abs())
                .max((right - input).abs());
        }
        assert!(
            max_delta > 0.02,
            "3D mode should measurably change the signal"
        );
    }

    #[test]
    fn headphones_3d_keeps_state_across_processing_blocks() {
        let input: Vec<_> = (0..1_024)
            .map(|index| if index == 0 { 0.5 } else { 0.0 })
            .collect();
        let mut whole = SpatialProcessor::new(48_000.0, SpatialMode::Headphones3D, 1.0);
        let mut split = SpatialProcessor::new(48_000.0, SpatialMode::Headphones3D, 1.0);

        let whole_output: Vec<_> = input
            .iter()
            .map(|&sample| whole.process(sample, sample))
            .collect();
        let mut split_output = Vec::with_capacity(input.len());
        for block in input.chunks(128) {
            for &sample in block {
                split_output.push(split.process(sample, sample));
            }
        }

        assert_eq!(whole_output, split_output);
        assert!(whole_output
            .iter()
            .any(|(left, right)| (left - right).abs() > 1e-4));
    }

    #[test]
    fn headphones_3d_widens_stereo_and_adds_audible_mono_reflections() {
        let mut spatial = SpatialProcessor::new(48_000.0, SpatialMode::Headphones3D, 1.0);
        let mut stereo_width_sum = 0.0;
        let mut mono_difference_sum = 0.0;
        for frame in 0..4_000 {
            let left = (frame as f32 * 0.071).sin() * 0.2;
            let right = (frame as f32 * 0.113).sin() * 0.2;
            let (out_l, out_r) = spatial.process(left, right);
            stereo_width_sum += (out_l - out_r).abs();
        }

        let mut spatial = SpatialProcessor::new(48_000.0, SpatialMode::Headphones3D, 1.0);
        for frame in 0..2_000 {
            let input = if frame == 0 { 0.5 } else { 0.0 };
            let (out_l, out_r) = spatial.process(input, input);
            mono_difference_sum += (out_l - input).abs() + (out_r - input).abs();
        }

        assert!(stereo_width_sum > 1_000.0);
        assert!(mono_difference_sum > 0.5);
    }
}
