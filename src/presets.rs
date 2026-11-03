//! Presets ported one-to-one from eqMac's `AdvancedEqualizerDefaultPresets`.

use serde::{Deserialize, Serialize};

use crate::dsp::{Channel, ExpertBand, FilterType};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BasicPreset {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub peak_limiter: bool,
    pub bass: f64,
    pub mid: f64,
    pub treble: f64,
}

impl BasicPreset {
    pub fn flat() -> Self {
        Self {
            id: "flat".into(),
            name: "Flat".into(),
            is_default: true,
            peak_limiter: false,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
        }
    }

    pub fn basic_defaults() -> Vec<BasicPreset> {
        [
            ("Acoustic", 1.0, 2.0, 2.0),
            ("Classical", 1.0, 0.0, 2.0),
            ("Corsair HS55 (starting point)", 0.0, 1.0, -1.0),
            ("Electronic", 3.0, 0.0, 2.0),
            ("Hip-Hop", 4.0, 1.0, 2.0),
            ("Jazz", 2.0, 1.0, 2.0),
            ("Latin", 3.0, -1.0, 3.0),
            ("Pop", -1.0, 3.0, -1.0),
            ("Rock", 3.0, 1.0, 3.0),
            ("Vocal", -2.0, 3.0, 1.0),
        ]
        .into_iter()
        .map(|(name, bass, mid, treble)| BasicPreset {
            id: format!("builtin-{}", camel_case(name)),
            name: name.into(),
            is_default: true,
            peak_limiter: true,
            bass,
            mid,
            treble,
        })
        .collect()
    }

    pub fn manual() -> Self {
        Self {
            id: "manual".into(),
            name: "Manual".into(),
            is_default: true,
            peak_limiter: false,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdvancedPreset {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub global: f64,
    pub bands: [f64; 10],
}

impl AdvancedPreset {
    pub fn flat() -> Self {
        Self {
            id: "flat".into(),
            name: "Flat".into(),
            is_default: true,
            global: 0.0,
            bands: [0.0; 10],
        }
    }

    pub fn manual() -> Self {
        Self {
            id: "manual".into(),
            name: "Manual".into(),
            is_default: true,
            global: 0.0,
            bands: [0.0; 10],
        }
    }
}

/// eqMac's `ADVANCED_EQUALIZER_DEFAULT_PRESETS`, in the same order as the
/// original dictionary (alphabetical by name).
pub const ADVANCED_DEFAULTS: &[(&str, [f64; 10])] = &[
    ("Flat", [0.0; 10]),
    (
        "Acoustic",
        [-8.3, 9.8, -15.68, 2.1, 18.22, 3.5, 7.0, 8.2, 7.1, 4.3],
    ),
    (
        "Bass Booster",
        [11.0, 8.5, 7.0, 5.0, 2.5, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "Bass Reducer",
        [-11.0, -8.5, -7.0, -5.0, -2.5, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "Classic",
        [9.5, 7.5, 6.0, 5.0, -3.0, -3.0, 0.0, 4.5, 6.5, 7.5],
    ),
    (
        "Corsair HS55 (starting point)",
        [-2.0, -1.0, 1.0, 1.0, 0.0, 0.0, 2.0, 2.0, -2.0, -2.0],
    ),
    (
        "Dance",
        [7.14, 13.1, 9.98, 0.0, 3.84, 7.3, 10.3, 9.08, 7.18, 0.0],
    ),
    (
        "Deep",
        [9.9, 7.1, 3.5, 2.0, 5.7, 5.0, 2.9, -4.3, -7.1, -9.2],
    ),
    (
        "Electronic",
        [8.5, 7.6, 2.4, 0.0, -4.3, 4.5, 1.7, 2.5, 7.9, 9.6],
    ),
    (
        "Hip-Hop",
        [10.0, 8.5, 3.0, 6.0, -2.0, -2.0, 3.0, -1.0, 4.0, 6.0],
    ),
    ("Jazz", [8.0, 6.0, 3.0, 4.5, -3.0, -3.0, 0.0, 3.0, 6.0, 7.5]),
    (
        "Latin",
        [9.0, 6.0, 0.0, 0.0, -3.0, -3.0, -3.0, 0.0, 6.0, 9.0],
    ),
    (
        "Loudness",
        [12.0, 8.0, 0.0, 0.0, -4.0, 0.0, -2.0, -10.0, 10.0, 2.0],
    ),
    (
        "Lounge",
        [-6.0, -3.0, -1.0, 3.0, 8.0, 5.0, 0.0, -3.0, 4.0, 2.0],
    ),
    ("Piano", [6.0, 4.0, 0.0, 5.0, 6.0, 3.0, 7.0, 9.0, 6.0, 7.0]),
    (
        "Pop",
        [-3.0, -2.0, 0.0, 4.0, 8.0, 8.0, 4.0, 0.0, -2.0, -3.0],
    ),
    (
        "RnB",
        [5.24, 13.84, 11.3, 2.66, -4.38, -3.0, 4.64, 5.3, 6.0, 7.5],
    ),
    (
        "Rock",
        [10.0, 8.0, 6.0, 3.0, -1.0, -2.0, 1.0, 5.0, 7.0, 9.0],
    ),
    (
        "Small Speakers",
        [11.0, 8.5, 7.0, 5.0, 2.5, 0.0, -2.5, -5.0, -7.0, -8.5],
    ),
    (
        "Spoken Word",
        [-6.92, -0.94, 0.0, 1.38, 6.92, 9.22, 9.68, 8.56, 5.08, 0.0],
    ),
    (
        "Treble Booster",
        [0.0, 0.0, 0.0, 0.0, 0.0, 2.5, 5.0, 7.0, 8.5, 11.0],
    ),
    (
        "Treble Reducer",
        [0.0, 0.0, 0.0, 0.0, 0.0, -2.5, -5.0, -7.0, -8.5, -11.0],
    ),
    (
        "Vocal Booster",
        [-3.0, -6.0, -6.0, 3.0, 7.5, 7.5, 6.0, 3.0, 0.0, -3.0],
    ),
];

pub fn camel_case(name: &str) -> String {
    let mut out = String::new();
    for (i, word) in name.split_whitespace().enumerate() {
        if i == 0 {
            out.push_str(&word.to_lowercase());
        } else {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(&chars.as_str().to_lowercase());
            }
        }
    }
    if out.is_empty() {
        "preset".into()
    } else {
        out
    }
}

pub fn advanced_defaults() -> Vec<AdvancedPreset> {
    ADVANCED_DEFAULTS
        .iter()
        .map(|(name, bands)| AdvancedPreset {
            id: camel_case(name),
            name: (*name).to_string(),
            is_default: true,
            global: 0.0,
            bands: *bands,
        })
        .collect()
}

/// Same ordering as the eqMac UI: Manual, Flat, then the rest alphabetically.
pub fn ordered_basic(user: &[BasicPreset]) -> Vec<BasicPreset> {
    let mut rest: Vec<BasicPreset> = BasicPreset::basic_defaults();
    rest.extend(
        user.iter()
            .filter(|p| p.id != "manual" && p.id != "flat")
            .cloned(),
    );
    rest.sort_by_key(|p| p.name.to_lowercase());

    let mut out = vec![BasicPreset::manual(), BasicPreset::flat()];
    out.extend(rest);
    out
}

pub fn display_name(name: &str, spanish: bool) -> &str {
    if !spanish {
        return name;
    }

    match name {
        "Acoustic" => "Acústica",
        "Classical" => "Clásica",
        "Electronic" => "Electrónica",
        "Hip-Hop" => "Hip-hop",
        "Jazz" => "Jazz",
        "Latin" => "Latina",
        "Pop" => "Pop",
        "Rock" => "Rock",
        "Vocal" => "Vocal",
        "Vocal Booster" => "Realce vocal",
        "Flat" => "Plana",
        "Manual" => "Manual",
        "Corsair HS55 (starting point)" => "Corsair HS55 (ajuste inicial)",
        _ => name,
    }
}

pub fn ordered_advanced(user: &[AdvancedPreset], show_defaults: bool) -> Vec<AdvancedPreset> {
    let mut rest: Vec<AdvancedPreset> = user
        .iter()
        .filter(|p| p.id != "manual" && p.id != "flat")
        .cloned()
        .collect();

    if show_defaults {
        rest.extend(advanced_defaults().into_iter().filter(|p| p.id != "flat"));
    }

    rest.sort_by_key(|p| p.name.to_lowercase());

    let mut out = vec![AdvancedPreset::manual(), AdvancedPreset::flat()];
    out.extend(rest);
    out
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExpertPreset {
    pub id: String,
    pub name: String,
    pub bands: Vec<ExpertBand>,
    pub global: f64,
    pub peak_limiter: bool,
}

impl ExpertPreset {
    pub fn flat() -> Self {
        Self {
            id: "flat".into(),
            name: "Flat".into(),
            bands: Vec::new(),
            global: 0.0,
            peak_limiter: true,
        }
    }

    pub fn manual() -> Self {
        Self {
            id: "manual".into(),
            name: "Manual".into(),
            bands: Vec::new(),
            global: 0.0,
            peak_limiter: true,
        }
    }
}

pub fn expert_defaults() -> Vec<ExpertPreset> {
    let harman = vec![
        (105.0, 5.5, 0.7),
        (1_000.0, -1.5, 1.2),
        (2_800.0, 3.0, 1.4),
        (6_000.0, -2.0, 1.8),
        (10_000.0, 2.5, 1.0),
    ]
    .into_iter()
    .map(|(frequency, gain, q)| ExpertBand {
        enabled: true,
        filter_type: FilterType::Peaking,
        frequency,
        gain,
        q,
        channel: Channel::Both,
    })
    .collect();

    vec![
        ExpertPreset::flat(),
        ExpertPreset {
            id: "harman".into(),
            name: "Harman Target".into(),
            bands: harman,
            global: 0.0,
            peak_limiter: true,
        },
        ExpertPreset {
            id: "bass-lift".into(),
            name: "Bass Lift".into(),
            bands: vec![ExpertBand {
                frequency: 70.0,
                gain: 6.0,
                q: 0.8,
                ..Default::default()
            }],
            global: 0.0,
            peak_limiter: true,
        },
    ]
}

/// Same ordering as the eqMac UI: Manual, Flat, then the rest alphabetically.
pub fn ordered_expert(user: &[ExpertPreset], show_defaults: bool) -> Vec<ExpertPreset> {
    let mut rest: Vec<ExpertPreset> = user
        .iter()
        .filter(|preset| preset.id != "manual" && preset.id != "flat")
        .cloned()
        .collect();
    if show_defaults {
        rest.extend(
            expert_defaults()
                .into_iter()
                .filter(|preset| preset.id != "manual" && preset.id != "flat"),
        );
    }
    rest.sort_by_key(|preset| preset.name.to_lowercase());
    let mut out = vec![ExpertPreset::manual()];
    let flat = user
        .iter()
        .find(|preset| preset.id == "flat")
        .cloned()
        .unwrap_or_else(ExpertPreset::flat);
    out.push(flat);
    out.extend(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_presets_include_requested_genres_and_headset() {
        let list = ordered_basic(&[]);
        for name in [
            "Vocal",
            "Rock",
            "Pop",
            "Latin",
            "Corsair HS55 (starting point)",
        ] {
            assert!(list.iter().any(|preset| preset.name == name));
        }
    }

    #[test]
    fn advanced_presets_include_corsair_headset_profile() {
        let list = ordered_advanced(&[], true);
        for name in ["Vocal Booster", "Rock", "Pop", "Latin"] {
            assert!(list.iter().any(|preset| preset.name == name));
        }
        let profile = list
            .into_iter()
            .find(|preset| preset.name == "Corsair HS55 (starting point)")
            .expect("built-in headset preset");
        assert_eq!(profile.bands.len(), 10);
        assert_eq!(profile.bands[6], 2.0);
    }
}
