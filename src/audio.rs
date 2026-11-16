use crate::RATE;
use anyhow::{Result, ensure};
use std::io::Cursor;

pub fn decode(bytes: &[u8]) -> Result<Vec<f32>> {
    let (samples, channels, rate) = if bytes.starts_with(b"OggS") {
        let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes))?;
        let channels = reader.ident_hdr.audio_channels as usize;
        let rate = reader.ident_hdr.audio_sample_rate;
        ensure!(channels > 0 && rate > 0, "Invalid OGG format");
        let mut samples = Vec::new();
        while let Some(packet) = reader.read_dec_packet_itl()? {
            ensure!(
                samples.len() + packet.len() <= rate as usize * channels * 30,
                "Reference must be at most 30 seconds"
            );
            samples.extend(packet.into_iter().map(|v| v as f32 / 32768.0));
        }
        (samples, channels, rate)
    } else {
        let mut wav = hound::WavReader::new(Cursor::new(bytes))?;
        let spec = wav.spec();
        ensure!(
            spec.channels > 0 && spec.sample_rate > 0,
            "Invalid WAV format"
        );
        ensure!(
            wav.duration() <= spec.sample_rate * 30,
            "Reference must be at most 30 seconds"
        );
        let samples = if spec.sample_format == hound::SampleFormat::Float {
            wav.samples::<f32>()
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let scale = 2_f32.powi(spec.bits_per_sample as i32 - 1);
            wav.samples::<i32>()
                .map(|v| v.map(|x| x as f32 / scale))
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        (samples, spec.channels as usize, spec.sample_rate)
    };
    ensure!(
        !samples.is_empty() && samples.iter().all(|v| v.is_finite()),
        "Empty or non-finite audio"
    );
    let mono: Vec<f32> = samples
        .chunks_exact(channels)
        .map(|c| c.iter().sum::<f32>() / channels as f32)
        .collect();
    let len = mono.len() * RATE as usize / rate as usize;
    Ok((0..len)
        .map(|i| {
            let position = i as f64 * rate as f64 / RATE as f64;
            let a = position as usize;
            let b = (a + 1).min(mono.len() - 1);
            mono[a] + (mono[b] - mono[a]) * position.fract() as f32
        })
        .collect())
}

pub const BUILTIN: [(&str, &[u8]); 4] = [
    ("FishBite", include_bytes!("../assets/FishBite.ogg")),
    (
        "FishingBobber_ver2_1",
        include_bytes!("../assets/FishingBobber_ver2_1.ogg"),
    ),
    (
        "FishingBobber_ver2_2",
        include_bytes!("../assets/FishingBobber_ver2_2.ogg"),
    ),
    (
        "FishingBobber_ver2_3",
        include_bytes!("../assets/FishingBobber_ver2_3.ogg"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Detector, FRAME};
    #[test]
    fn rejects_unrelated_noise() {
        let mut seed = 1u32;
        for (_, bytes) in BUILTIN {
            let mut detector = Detector::new(&decode(bytes).unwrap()).unwrap();
            for _ in 0..200 {
                let frame = std::array::from_fn::<_, FRAME, _>(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    (seed as f32 / u32::MAX as f32 - 0.5) * 0.1
                });
                assert!(detector.score(&frame) < 0.96);
            }
        }
    }
    #[test]
    fn real_splashes_with_arbitrary_frame_alignment_and_volume() {
        for (name, bytes) in BUILTIN {
            let samples = decode(bytes).unwrap();
            let duration = Detector::new(&samples).unwrap().duration_ms;
            let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
            let onset =
                samples.iter().position(|s| s.abs() > peak * 0.08).unwrap() * 1000 / RATE as usize;
            let mut worst = 1.0_f32;
            let mut latest = 0;
            for offset in (0..FRAME).step_by(16) {
                let mut d = Detector::new(&samples).unwrap();
                let mut playback = vec![0.0; FRAME * 10 + offset];
                playback.extend(samples.iter().map(|x| x * 0.2));
                let mut best = 0.0_f32;
                let mut first_match = None;
                for (index, frame) in playback.as_chunks::<FRAME>().0.iter().enumerate() {
                    let score = d.score(frame);
                    if index < 30 {
                        best = best.max(score);
                    }
                    if score >= 0.96 && first_match.is_none() {
                        first_match = Some((index + 1) * 20 - 200);
                    }
                }
                worst = worst.min(best);
                latest = latest.max(first_match.unwrap_or(99999));
            }
            println!(
                "{name}: file={}ms, active={duration}ms, worst_score={worst:.4}, onset={onset}ms, latest_detection={latest}ms",
                samples.len() * 1000 / RATE as usize
            );
            assert!(worst >= 0.96, "{name}: score {worst}");
            assert!(
                latest <= onset + 180,
                "{name}: detected too late ({latest}ms)"
            );
        }
    }
}
