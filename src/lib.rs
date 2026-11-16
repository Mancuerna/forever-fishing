use std::collections::VecDeque;

pub const RATE: u32 = 16000;
pub const FRAME: usize = 320; // 20 ms
const BANDS: usize = 20;
const PREFIX: usize = 6; // Recognize the first 120 ms, not the whole splash.
type Feature = [f32; BANDS];

fn feature(samples: &[f32]) -> Feature {
    let mut out = [0.0; BANDS];
    for (band, value) in out.iter_mut().enumerate() {
        let frequency = 180.0 * 1.19_f32.powi(band as i32);
        let coefficient = 2.0 * (std::f32::consts::TAU * frequency / RATE as f32).cos();
        let (mut a, mut b) = (0.0, 0.0);
        for (i, sample) in samples.iter().enumerate() {
            let window = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FRAME as f32).cos();
            let next = sample * window + coefficient * a - b;
            b = a;
            a = next;
        }
        *value = (a * a + b * b - coefficient * a * b).max(0.0).sqrt();
    }
    let norm = out.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
    out.iter_mut().for_each(|v| *v /= norm);
    out
}

pub struct Detector {
    reference: Vec<Feature>,
    history: VecDeque<Feature>,
    pub duration_ms: u64,
}
impl Detector {
    pub fn new(samples: &[f32]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            samples.iter().all(|x| x.is_finite()),
            "Non-finite audio samples"
        );
        let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        anyhow::ensure!(peak > 0.001, "Reference is silent");
        let first = samples.iter().position(|s| s.abs() > peak * 0.08).unwrap();
        let last = samples.iter().rposition(|s| s.abs() > peak * 0.08).unwrap();
        let sound = &samples[first..=last];
        let duration_ms = sound.len() as u64 * 1000 / RATE as u64;
        anyhow::ensure!(
            (250..=10000).contains(&duration_ms),
            "Use an isolated splash lasting 250 ms to 10 s"
        );
        Ok(Self {
            reference: sound
                .as_chunks::<FRAME>()
                .0
                .iter()
                .take(PREFIX)
                .map(|frame| feature(frame))
                .collect(),
            history: VecDeque::new(),
            duration_ms,
        })
    }
    pub fn reset(&mut self) {
        self.history.clear();
    }
    pub fn score(&mut self, frame: &[f32]) -> f32 {
        assert_eq!(frame.len(), FRAME);
        self.history.push_back(feature(frame));
        if self.history.len() > PREFIX {
            self.history.pop_front();
        }
        if self.history.len() != PREFIX {
            return 0.0;
        }
        self.reference
            .iter()
            .zip(&self.history)
            .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>())
            .sum::<f32>()
            / PREFIX as f32
    }
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Reel,
    Cast,
}
#[derive(Default)]
pub struct Cycle {
    pending: Option<(u64, u64, bool)>,
    blocked_until: u64,
}
impl Cycle {
    pub fn cancel(&mut self, now: u64) {
        self.pending = None;
        self.blocked_until = now + 1500;
    }
    pub fn detect(&mut self, now: u64, remaining: u64, reaction: u64, recast: u64) {
        if self.pending.is_none() && now >= self.blocked_until && remaining > 30 {
            let reel = now + reaction.min(remaining - 30);
            self.pending = Some((reel, recast.min(1000), false));
        }
    }
    pub fn tick(&mut self, now: u64) -> Option<Action> {
        let (due, delay, reeled) = self.pending?;
        if now < due {
            return None;
        }
        if now > due + 100 {
            self.cancel(now);
            return None;
        }
        if !reeled {
            self.pending = Some((now + delay + 25, 0, true));
            Some(Action::Reel)
        } else {
            self.pending = None;
            self.blocked_until = now + 2000;
            Some(Action::Cast)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timing_and_duplicate_suppression() {
        let mut c = Cycle::default();
        c.detect(0, 500, 100, 1000);
        c.detect(50, 500, 0, 0);
        assert_eq!(c.tick(99), None);
        assert_eq!(c.tick(100), Some(Action::Reel));
        assert_eq!(c.tick(1124), None);
        assert_eq!(c.tick(1125), Some(Action::Cast));
        c.detect(1200, 500, 0, 0);
        assert_eq!(c.tick(1200), None);
    }
    #[test]
    fn pause_cancels_recast_and_late_actions() {
        let mut c = Cycle::default();
        c.detect(0, 500, 0, 500);
        assert_eq!(c.tick(0), Some(Action::Reel));
        c.cancel(1);
        assert_eq!(c.tick(525), None);
        c.detect(2000, 500, 0, 0);
        assert_eq!(c.tick(2200), None);
    }
    #[test]
    fn recognizes_reference_but_not_silence_or_other_tone() {
        let samples: Vec<f32> = (0..8000).map(|i| (i as f32 * 0.3).sin()).collect();
        let mut d = Detector::new(&samples).unwrap();
        let start = samples.iter().position(|s| s.abs() > 0.08).unwrap();
        let mut score = 0.0;
        for frame in samples[start..].as_chunks::<FRAME>().0.iter().take(PREFIX) {
            score = d.score(frame);
        }
        assert!(score > 0.99);
        for _ in 0..PREFIX {
            score = d.score(&[0.0; FRAME]);
        }
        assert_eq!(score, 0.0);
        let other: Vec<f32> = (0..FRAME).map(|i| (i as f32 * 1.9).sin()).collect();
        for _ in 0..PREFIX {
            score = d.score(&other);
        }
        assert!(score < 0.8);
        assert!(Detector::new(&[0.0; 8000]).is_err());
    }
}
