//! Offline WAV rendering with the current equalizer, spatial effect and volume.

use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::dsp::{gain_factors, EqChain, EqParams, SpatialProcessor};

pub fn render_file(
    input: &Path,
    output: &Path,
    params: &EqParams,
    volume: f32,
    balance: f32,
) -> Result<(), String> {
    let mut reader = WavReader::open(input).map_err(|e| e.to_string())?;
    let source = reader.spec();
    let channels = source.channels as usize;
    if channels == 0 {
        return Err("input has no channels".into());
    }

    let samples: Vec<f32> = match source.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?,
        SampleFormat::Int if source.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / 32_768.0)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, _>>()?,
        SampleFormat::Int => reader
            .samples::<i32>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / 2_147_483_648.0)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, _>>()?,
    };

    let spec = WavSpec {
        channels: 2,
        sample_rate: source.sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(output, spec).map_err(|e| e.to_string())?;
    let mut chain = EqChain::new(source.sample_rate as f64, params);
    let mut spatial = SpatialProcessor::new(
        source.sample_rate as f32,
        params.spatial,
        params.spatial_mix,
    );
    let (left_gain, right_gain) = gain_factors(volume, balance, false);

    for frame in samples.chunks(channels) {
        let left = frame[0];
        let right = if channels > 1 { frame[1] } else { frame[0] };
        let (left, right) = chain.process(left, right);
        let (left, right) = spatial.process(left, right);
        writer
            .write_sample((left * left_gain).clamp(-1.0, 1.0))
            .map_err(|e| e.to_string())?;
        writer
            .write_sample((right * right_gain).clamp(-1.0, 1.0))
            .map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}
