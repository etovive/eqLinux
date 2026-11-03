//! Persistent application settings (stored as JSON under XDG config).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::dsp::{EqMode, EqParams, ExpertBand, SpatialMode};
use crate::i18n::Language;
use crate::presets::{AdvancedPreset, BasicPreset, ExpertPreset};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Green,
    Blue,
    Purple,
    Orange,
    Pink,
    Graphite,
}

impl Accent {
    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Green, false) => "eqMac green",
            (Self::Green, true) => "Verde eqMac",
            (Self::Blue, false) => "Ocean blue",
            (Self::Blue, true) => "Azul océano",
            (Self::Purple, false) => "Violet",
            (Self::Purple, true) => "Violeta",
            (Self::Orange, false) => "Sunset",
            (Self::Orange, true) => "Atardecer",
            (Self::Pink, false) => "Rose",
            (Self::Pink, true) => "Rosa",
            (Self::Graphite, false) => "Graphite",
            (Self::Graphite, true) => "Grafito",
        }
    }

    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Green => (79, 141, 113),
            Self::Blue => (72, 126, 176),
            Self::Purple => (132, 105, 173),
            Self::Orange => (190, 121, 70),
            Self::Pink => (183, 96, 120),
            Self::Graphite => (132, 136, 140),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpertConfig {
    pub global: f64,
    pub bands: Vec<ExpertBand>,
    pub peak_limiter: bool,
    pub selected_preset: String,
    pub show_default_presets: bool,
}

impl Default for ExpertConfig {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl ExpertConfig {
    pub fn with_defaults() -> Self {
        Self {
            global: 0.0,
            bands: Vec::new(),
            peak_limiter: true,
            selected_preset: "flat".into(),
            show_default_presets: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuperPreset {
    pub id: String,
    pub name: String,
    pub output_sink: String,
    pub mode: EqMode,
    pub preset_id: String,
}

impl Default for SuperPreset {
    fn default() -> Self {
        Self {
            id: "super".into(),
            name: "Super preset".into(),
            output_sink: String::new(),
            mode: EqMode::Basic,
            preset_id: "flat".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub show_volume: bool,
    pub show_balance: bool,
    pub show_equalizer: bool,
    pub show_outputs: bool,
    pub use_sliders: bool,
    pub equalizer_collapsed: bool,
    pub spectrum: bool,
    pub spectrum_style: SpectrumStyle,
    pub show_app_mixer: bool,
    pub revision: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpectrumStyle {
    #[default]
    Default,
    Waveform,
    Mirrored,
    Filled,
    Rounded,
    Blocks,
    Dots,
}

impl SpectrumStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Waveform,
            Self::Waveform => Self::Mirrored,
            Self::Mirrored => Self::Filled,
            Self::Filled => Self::Rounded,
            Self::Rounded => Self::Blocks,
            Self::Blocks => Self::Dots,
            Self::Dots => Self::Default,
        }
    }

    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Default, false) => "Default",
            (Self::Default, true) => "Predeterminado",
            (Self::Waveform, false) => "Waveform",
            (Self::Waveform, true) => "Onda",
            (Self::Mirrored, false) => "Mirrored bars",
            (Self::Mirrored, true) => "Barras espejo",
            (Self::Filled, false) => "Filled waveform",
            (Self::Filled, true) => "Onda rellena",
            (Self::Rounded, false) => "Rounded bars",
            (Self::Rounded, true) => "Barras redondeadas",
            (Self::Blocks, false) => "Blocks",
            (Self::Blocks, true) => "Bloques",
            (Self::Dots, false) => "Dots",
            (Self::Dots, true) => "Puntos",
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_volume: true,
            show_balance: true,
            show_equalizer: true,
            show_outputs: true,
            use_sliders: false,
            equalizer_collapsed: false,
            spectrum: false,
            spectrum_style: SpectrumStyle::default(),
            show_app_mixer: false,
            revision: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BasicConfig {
    pub bass: f64,
    pub mid: f64,
    pub treble: f64,
    pub peak_limiter: bool,
    pub selected_preset: String,
}

impl Default for BasicConfig {
    fn default() -> Self {
        Self {
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            peak_limiter: false,
            selected_preset: "flat".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedConfig {
    pub global: f64,
    pub bands: [f64; 10],
    pub selected_preset: String,
    pub show_default_presets: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            global: 0.0,
            bands: [0.0; 10],
            selected_preset: "flat".into(),
            show_default_presets: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub app_enabled: bool,
    pub eq_enabled: bool,
    pub start_minimized_on_startup: bool,
    pub eq_mode: EqMode,
    pub basic: BasicConfig,
    pub advanced: AdvancedConfig,
    pub expert: ExpertConfig,
    pub language: Language,
    pub volume: f32,
    pub balance: f32,
    pub muted: bool,
    pub boost_enabled: bool,
    pub output_sink: Option<String>,
    pub virtual_sink: String,
    pub routing_enabled: bool,
    pub spatial: SpatialMode,
    pub spatial_mix: f32,
    pub left_trim: f64,
    pub right_trim: f64,
    pub accent: Accent,
    pub super_presets_enabled: bool,
    pub super_presets: Vec<SuperPreset>,
    pub ui: UiSettings,
    pub user_basic_presets: Vec<BasicPreset>,
    pub user_advanced_presets: Vec<AdvancedPreset>,
    pub user_expert_presets: Vec<ExpertPreset>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_enabled: true,
            eq_enabled: true,
            start_minimized_on_startup: false,
            eq_mode: EqMode::Basic,
            basic: BasicConfig::default(),
            advanced: AdvancedConfig::default(),
            expert: ExpertConfig::with_defaults(),
            language: Language::detect(),
            volume: 0.5,
            balance: 0.0,
            muted: false,
            boost_enabled: true,
            output_sink: None,
            virtual_sink: "eqlinux_sink".into(),
            routing_enabled: true,
            spatial: SpatialMode::Off,
            spatial_mix: 0.5,
            left_trim: 0.0,
            right_trim: 0.0,
            accent: Accent::Green,
            super_presets_enabled: true,
            super_presets: Vec::new(),
            ui: UiSettings::default(),
            user_basic_presets: Vec::new(),
            user_advanced_presets: Vec::new(),
            user_expert_presets: Vec::new(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("eqlinux").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(text) => {
                let mut config: Self = serde_json::from_str(&text).unwrap_or_else(|err| {
                    eprintln!("eqlinux: could not parse {}: {err}", path.display());
                    Self::default()
                });
                if config.migrate() {
                    let _ = config.save();
                }
                config
            }
            Err(_) => Self::default(),
        }
    }

    fn migrate(&mut self) -> bool {
        let mut changed = false;
        if self.ui.revision < 1 {
            self.ui.use_sliders = false;
            self.ui.revision = 1;
            changed = true;
        }
        changed
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, text).map_err(|e| e.to_string())
    }

    pub fn eq_params(&self) -> EqParams {
        EqParams {
            mode: self.eq_mode,
            bass: self.basic.bass,
            mid: self.basic.mid,
            treble: self.basic.treble,
            peak_limiter: self.basic.peak_limiter,
            advanced_global: self.advanced.global,
            advanced_bands: self.advanced.bands,
            expert_global: self.expert.global,
            expert_peak_limiter: self.expert.peak_limiter,
            expert_bands: self.expert.bands.clone(),
            left_trim: self.left_trim,
            right_trim: self.right_trim,
            spatial: self.spatial,
            spatial_mix: self.spatial_mix,
        }
    }

    pub fn effective_volume(&self) -> f32 {
        if self.boost_enabled {
            self.volume
        } else {
            self.volume.min(1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{Channel, FilterType};

    #[test]
    fn round_trips_pro_settings() {
        let mut config = Config {
            language: Language::Spanish,
            accent: Accent::Purple,
            spatial: SpatialMode::LargeHall,
            spatial_mix: 0.8,
            left_trim: -2.5,
            right_trim: 1.5,
            start_minimized_on_startup: true,
            ..Config::default()
        };
        config.ui.spectrum_style = SpectrumStyle::Dots;
        config.expert.bands.push(ExpertBand {
            enabled: true,
            filter_type: FilterType::LowShelf,
            frequency: 120.0,
            gain: 4.0,
            q: 0.7,
            channel: Channel::Both,
        });
        config.super_presets.push(SuperPreset {
            id: "super-1".into(),
            name: "Headphones".into(),
            output_sink: "bluez_output.test".into(),
            mode: EqMode::Expert,
            preset_id: "manual".into(),
        });

        let text = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded.language, Language::Spanish);
        assert_eq!(decoded.accent, Accent::Purple);
        assert!(decoded.start_minimized_on_startup);
        assert_eq!(decoded.ui.spectrum_style, SpectrumStyle::Dots);
        assert_eq!(decoded.spatial, SpatialMode::LargeHall);
        assert_eq!(decoded.expert.bands.len(), 1);
        assert_eq!(decoded.super_presets.len(), 1);
        assert_eq!(decoded.eq_params().spatial_mix, 0.8);

        let mut old_config = serde_json::to_value(Config::default()).unwrap();
        old_config["ui"]
            .as_object_mut()
            .unwrap()
            .remove("spectrum_style");
        let old_config: Config = serde_json::from_value(old_config).unwrap();
        assert_eq!(old_config.ui.spectrum_style, SpectrumStyle::Default);
    }
}
