use forever_fishing::{
    Detector, FRAME,
    audio::{BUILTIN, decode},
};
fn main() {
    for (name, bytes) in BUILTIN {
        let samples = decode(bytes).unwrap();
        for noise in [0.0, 0.01, 0.03, 0.1] {
            let mut detector = Detector::new(&samples).unwrap();
            let mut seed = 1u32;
            let mixed: Vec<f32> = samples
                .iter()
                .map(|x| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    x * 0.2 + (seed as f32 / u32::MAX as f32 - 0.5) * noise
                })
                .collect();
            let best = mixed
                .as_chunks::<FRAME>()
                .0
                .iter()
                .take(30)
                .map(|frame| detector.score(frame))
                .fold(0.0_f32, f32::max);
            println!("{name}: background amplitude {noise:.3}, score {best:.4}");
        }
    }
}
