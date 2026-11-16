use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use forever_fishing::{Cycle, Detector, FRAME, RATE};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
use wasapi::*;
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Record PC output to a WAV. Isolate/trim one splash before using it.
    Record {
        path: PathBuf,
        #[arg(long, default_value_t = 5)]
        seconds: u64,
    },
    /// Listen for a reference splash. Starts paused; F8 toggles, Esc exits.
    Run {
        reference: PathBuf,
        #[arg(long, default_value_t = 0.90)]
        threshold: f32,
        #[arg(long, default_value = "World of Warcraft")]
        window: String,
        #[arg(long)]
        dry_run: bool,
    },
}
fn down(key: u16) -> bool {
    unsafe { GetAsyncKeyState(key as i32) < 0 }
}
fn foreground_matches(expected: &str) -> bool {
    let mut title = [0u16; 512];
    unsafe {
        let hwnd = GetForegroundWindow();
        let length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        length > 0
            && String::from_utf16_lossy(&title[..length as usize])
                .to_lowercase()
                .contains(&expected.to_lowercase())
    }
}
fn space() -> Result<()> {
    let input = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_SPACE,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        ensure!(
            SendInput(1, &input(0), size_of::<INPUT>() as i32) == 1,
            "Space key-down failed"
        );
        std::thread::sleep(Duration::from_millis(20));
        let released = SendInput(1, &input(KEYEVENTF_KEYUP), size_of::<INPUT>() as i32);
        if released != 1 {
            SendInput(1, &input(KEYEVENTF_KEYUP), size_of::<INPUT>() as i32);
        }
        ensure!(released == 1, "Space key-up failed");
    }
    Ok(())
}
fn load(path: &PathBuf) -> Result<Vec<f32>> {
    let mut wav = hound::WavReader::open(path)?;
    let spec = wav.spec();
    ensure!(
        spec.channels > 0 && spec.sample_rate > 0,
        "Invalid WAV format"
    );
    ensure!(
        wav.duration() <= spec.sample_rate * 30,
        "Reference must be at most 30 seconds"
    );
    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        wav.samples::<f32>()
            .collect::<std::result::Result<_, _>>()?
    } else {
        let scale = 2_f32.powi(spec.bits_per_sample as i32 - 1);
        wav.samples::<i32>()
            .map(|v| v.map(|x| x as f32 / scale))
            .collect::<std::result::Result<_, _>>()?
    };
    let mono: Vec<f32> = samples
        .chunks_exact(spec.channels as usize)
        .map(|c| c.iter().sum::<f32>() / c.len() as f32)
        .collect();
    let len = mono.len() * RATE as usize / spec.sample_rate as usize;
    Ok((0..len)
        .map(|i| {
            let position = i as f64 * spec.sample_rate as f64 / RATE as f64;
            let a = position as usize;
            let b = (a + 1).min(mono.len() - 1);
            mono[a] + (mono[b] - mono[a]) * position.fract() as f32
        })
        .collect())
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    let (mut detector, threshold, window, dry_run, recording) = match cli.command {
        Command::Record { path, seconds } => {
            ensure!((1..=30).contains(&seconds), "seconds must be 1..30");
            (None, 0.9, String::new(), true, Some((path, seconds)))
        }
        Command::Run {
            reference,
            threshold,
            window,
            dry_run,
        } => {
            ensure!(
                threshold.is_finite() && (0.5..=1.0).contains(&threshold),
                "threshold must be 0.5..1"
            );
            ensure!(!window.trim().is_empty(), "Window title must not be empty");
            let d = Detector::new(&load(&reference)?)?;
            println!(
                "Reference: {} ms. F8: start/pause. Esc: exit. Dry run: {dry_run}",
                d.duration_ms
            );
            (Some(d), threshold, window, dry_run, None)
        }
    };
    initialize_mta().ok()?;
    let enumerator = DeviceEnumerator::new()?;
    let device = enumerator.get_default_device(Direction::Render)?;
    let mut client = device.get_iaudioclient()?;
    let format = WaveFormat::new(32, 32, SampleType::Float, RATE as usize, 1, None);
    client
        .initialize_client(
            &format,
            Direction::Capture,
            StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 200_000,
            },
            None,
        )
        .context("Cannot capture default Windows output device")?;
    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    let mut bytes = VecDeque::new();
    let mut writer = recording
        .as_ref()
        .map(|(path, _)| {
            hound::WavWriter::create(
                path,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: RATE,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
        })
        .transpose()?;
    client.start_stream()?;
    let start = Instant::now();
    let mut active = false;
    let mut previous_f8 = false;
    let mut cycle = Cycle::default();
    loop {
        let now = start.elapsed().as_millis() as u64;
        if down(VK_ESCAPE) {
            break;
        }
        if let Some((_, seconds)) = &recording
            && now >= seconds * 1000
        {
            break;
        }
        let f8 = down(VK_F8);
        if f8 && !previous_f8 && detector.is_some() {
            active = !active;
            cycle.cancel(now);
            println!("{}", if active { "Listening" } else { "Paused" });
        }
        previous_f8 = f8;
        let allowed = active && foreground_matches(&window);
        if active && !allowed {
            active = false;
            println!("Paused: game lost focus. F8 to resume.");
        }
        if !allowed {
            cycle.cancel(now);
        }
        // Poll briefly so silence never prevents pause/exit or scheduled actions.
        let _ = event.wait_for_event(10);
        capture.read_from_device_to_deque(&mut bytes)?;
        if writer.is_none() && bytes.len() > FRAME * 4 * 5 {
            bytes.clear();
            cycle.cancel(now);
            if let Some(d) = &mut detector {
                d.reset();
            }
        }
        while bytes.len() >= FRAME * 4 {
            let mut frame = [0.0; FRAME];
            for sample in &mut frame {
                let raw = std::array::from_fn(|_| bytes.pop_front().unwrap());
                *sample = f32::from_le_bytes(raw);
            }
            if let Some(w) = &mut writer {
                for sample in frame {
                    w.write_sample(sample)?;
                }
            }
            if let Some(d) = &mut detector {
                if !allowed {
                    d.reset();
                    continue;
                }
                let score = d.score(&frame);
                if score >= threshold {
                    let remaining = d.duration_ms.saturating_sub(120 + 100); // prefix + latency margin
                    let reaction = rand::random_range(0..=remaining.saturating_sub(30));
                    cycle.detect(now, remaining, reaction, rand::random_range(0..=1000));
                }
            }
        }
        if allowed
            && foreground_matches(&window)
            && !down(VK_ESCAPE)
            && !down(VK_F8)
            && let Some(action) = cycle.tick(start.elapsed().as_millis() as u64)
        {
            println!("{action:?}{}", if dry_run { " (dry run)" } else { "" });
            if !dry_run {
                space()?;
            }
        }
    }
    client.stop_stream()?;
    if let Some(w) = writer {
        w.finalize()?;
        println!("Recording saved");
    }
    Ok(())
}
