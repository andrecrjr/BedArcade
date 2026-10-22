#[cfg(target_os = "linux")]
use gamepad_desktop::{keyboard, placement};
#[cfg(target_os = "linux")]
#[path = "../src/wayland_keyboard.rs"]
mod wayland_keyboard;

/// Measure the real Wayland overlay without injecting mouse or keyboard input.
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    let mut arguments = std::env::args().skip(1);
    let scenario = arguments.next().unwrap_or_else(|| "closed".to_owned());
    if !matches!(scenario.as_str(), "closed" | "open") {
        return Err("usage: overlay_benchmark [closed|open] [seconds]".into());
    }
    let seconds = arguments
        .next()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(60);
    if seconds == 0 {
        return Err("seconds must be greater than zero".into());
    }
    let mut overlay = wayland_keyboard::WaylandKeyboardOverlay::new()?;
    let mut session = keyboard::KeyboardSession::default();
    session.open = scenario == "open";
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_secs(2) {
        overlay.update(&session)?;
        thread::sleep(Duration::from_millis(8));
    }
    let started = Instant::now();
    let mut samples = Vec::new();
    while started.elapsed() < Duration::from_secs(seconds) {
        let update = Instant::now();
        overlay.update(&session)?;
        samples.push(update.elapsed().as_nanos() as u64);
        thread::sleep(Duration::from_millis(8));
    }
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({
            "scenario": scenario,
            "elapsed_seconds": started.elapsed().as_secs_f64(),
            "updates": samples.len(),
            "update_mean_us": samples.iter().sum::<u64>() as f64 / samples.len() as f64 / 1000.0,
            "update_p50_us": samples[samples.len() / 2] as f64 / 1000.0,
            "update_p95_us": samples[(samples.len() - 1) * 95 / 100] as f64 / 1000.0,
        })
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This benchmark requires Linux and a compositor supporting wlr-layer-shell.");
    std::process::exit(1);
}
