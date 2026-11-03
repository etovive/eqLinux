//! Import AutoEQ parametric profiles (ParametricEQ.txt / CSV).

use crate::dsp::{Channel, ExpertBand, FilterType};

pub fn parse(text: &str) -> Result<Vec<ExpertBand>, String> {
    let mut bands = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if line.to_ascii_lowercase().starts_with("filter") {
            if let Some(band) = parse_parametric_line(line) {
                bands.push(band);
            }
        }
    }

    if bands.is_empty() {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            if let Some(band) = parse_csv_line(line) {
                bands.push(band);
            }
        }
    }

    if bands.is_empty() {
        Err("no AutoEQ filters found".into())
    } else {
        Ok(bands)
    }
}

fn parse_parametric_line(line: &str) -> Option<ExpertBand> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    let enabled = !tokens.iter().any(|token| token.eq_ignore_ascii_case("OFF"));
    let filter_type = tokens
        .iter()
        .skip(3)
        .find(|value| {
            matches!(
                value.to_ascii_uppercase().as_str(),
                "PK" | "LS"
                    | "LSC"
                    | "HS"
                    | "HSC"
                    | "LP"
                    | "LPF"
                    | "HP"
                    | "HPF"
                    | "BP"
                    | "BPF"
                    | "NO"
                    | "NOTCH"
                    | "LOWSHELF"
                    | "HIGHSHELF"
                    | "LOWPASS"
                    | "HIGHPASS"
                    | "BANDPASS"
            )
        })
        .map(|value| filter_type(value))
        .unwrap_or_default();
    let frequency = token_after(&tokens, "Fc").unwrap_or(1_000.0);
    let gain = token_after(&tokens, "Gain").unwrap_or(0.0);
    let q = token_after(&tokens, "Q").unwrap_or(1.0);
    Some(ExpertBand {
        enabled,
        filter_type,
        frequency,
        gain,
        q,
        channel: Channel::Both,
    })
}

fn parse_csv_line(line: &str) -> Option<ExpertBand> {
    let columns: Vec<&str> = line.split(',').map(str::trim).collect();
    if columns.len() < 3 {
        return None;
    }
    let frequency = columns[0].parse::<f64>().ok()?;
    let gain = columns[1].parse::<f64>().ok()?;
    let q = columns[2].parse::<f64>().ok()?;
    let filter_type = columns
        .get(3)
        .map(|value| filter_type(value))
        .unwrap_or(FilterType::Peaking);
    Some(ExpertBand {
        enabled: true,
        filter_type,
        frequency,
        gain,
        q,
        channel: Channel::Both,
    })
}

fn token_after(tokens: &[&str], key: &str) -> Option<f64> {
    tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case(key))
        .and_then(|index| tokens.get(index + 1))
        .and_then(|value| value.parse::<f64>().ok())
}

fn filter_type(value: &str) -> FilterType {
    match value.to_ascii_uppercase().as_str() {
        "LS" | "LSC" | "LOWSHELF" => FilterType::LowShelf,
        "HS" | "HSC" | "HIGHSHELF" => FilterType::HighShelf,
        "LP" | "LPF" | "LOWPASS" => FilterType::LowPass,
        "HP" | "HPF" | "HIGHPASS" => FilterType::HighPass,
        "BP" | "BPF" | "BANDPASS" => FilterType::BandPass,
        "NO" | "NOTCH" => FilterType::Notch,
        _ => FilterType::Peaking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_parametric_profile() {
        let text = "Preamp: -6.0 dB\nFilter 1: ON PK Fc 105 Hz Gain -3.2 dB Q 0.70\n\
                    Filter 2: ON LS Fc 200 Hz Gain 4.0 dB Q 0.70\n\
                    Filter 3: OFF HS Fc 8000 Hz Gain -2.0 dB Q 0.70";
        let bands = parse(text).unwrap();
        assert_eq!(bands.len(), 3);
        assert_eq!(bands[0].filter_type, FilterType::Peaking);
        assert!((bands[0].frequency - 105.0).abs() < f64::EPSILON);
        assert!((bands[0].gain + 3.2).abs() < f64::EPSILON);
        assert_eq!(bands[1].filter_type, FilterType::LowShelf);
        assert_eq!(bands[2].filter_type, FilterType::HighShelf);
        assert!(!bands[2].enabled);
    }

    #[test]
    fn parses_csv_profile() {
        let bands = parse("100,3.5,0.7\n8000,-2.0,1.0,HS").unwrap();
        assert_eq!(bands.len(), 2);
        assert_eq!(bands[0].filter_type, FilterType::Peaking);
        assert_eq!(bands[1].filter_type, FilterType::HighShelf);
    }

    #[test]
    fn rejects_empty_input() {
        assert!(parse("# comment only").is_err());
    }
}
