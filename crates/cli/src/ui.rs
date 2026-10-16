//! Text output of the CLI, kept pure so it can be tested.

use pd200x_core::calibrate::Proposal;
use pd200x_core::level::{Level, FLOOR_DB};
use pd200x_core::model::{LiveState, Settings};

fn on(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// A meter like `[#########-------|----]  -18.0 dB` with the peak as `|`.
pub fn meter(level: &Level, width: usize) -> String {
    let pos = |db: f32| (((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0) * width as f32).round() as usize;
    let filled = pos(level.current_db);
    let peak = pos(level.peak_db).min(width.saturating_sub(1));
    let bar: String = (0..width)
        .map(|i| {
            if i < filled {
                '#'
            } else if i == peak && level.peak_db > FLOOR_DB {
                '|'
            } else {
                '-'
            }
        })
        .collect();
    let clip = if level.clipping { "  CLIP" } else { "" };
    format!("[{bar}] {:>6.1} dB  peak {:>6.1} dB{clip}", level.current_db, level.peak_db)
}

pub fn status(live: Option<&LiveState>, boost: Option<f32>, settings: &Settings) -> String {
    let mut out = Vec::new();
    match live {
        Some(s) => {
            out.push("PD200X  connected".to_string());
            out.push(format!("  Mute          {}", on(s.mute)));
            out.push(format!("  Gain          {} / 100", s.gain));
            out.push(format!("  Monitor       {}", on(s.monitor)));
            out.push(format!("  Headphone     {} / 100", s.headphone));
            out.push(format!("  Noise cancel  {} (strength {} / 40)", on(s.noise_cancellation), s.nc_strength));
        }
        None => out.push("PD200X  not connected".to_string()),
    }
    out.push(match boost {
        Some(db) if db > 0.0 => format!("  Boost         +{db:.1} dB (PipeWire)"),
        Some(_) => "  Boost         off".to_string(),
        None => "  Boost         unavailable (no PipeWire source)".to_string(),
    });
    out.push(match &settings.tone {
        Some(t) => format!(
            "  Tone          {}, compressor {}, limiter {}",
            t.source.as_deref().unwrap_or("custom EQ"),
            dynamics(t.compressor.enabled, t.compressor.threshold_db),
            dynamics(t.limiter.enabled, t.limiter.threshold_db),
        ),
        None => "  Tone          not set in the app".to_string(),
    });
    out.push(match &settings.lighting {
        Some(l) if l.enabled => format!(
            "  Light         {}, {} %{}",
            format!("{:?}", l.color).to_lowercase(),
            l.brightness,
            if l.linkage { ", off while muted" } else { "" }
        ),
        Some(_) => "  Light         off".to_string(),
        None => "  Light         not set in the app".to_string(),
    });
    out.join("\n")
}

fn dynamics(enabled: bool, db: f32) -> String {
    if enabled {
        format!("{db:.0} dB")
    } else {
        "off".into()
    }
}

pub fn proposal(p: &Proposal) -> String {
    format!(
        "Speech peaks at {:.1} dB.\nProposal: gain {} / 100, boost +{:.1} dB → peaks at about {:.1} dB.",
        p.measured_peak_db, p.gain, p.boost_db, p.expected_peak_db
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd200x_core::model::{Lighting, RgbColor, Tone};

    #[test]
    fn meter_shows_level_and_peak() {
        let l = Level { current_db: -30.0, peak_db: -12.0, clipping: false };
        assert_eq!(meter(&l, 10), "[#####---|-]  -30.0 dB  peak  -12.0 dB");
        let silent = meter(&Level::SILENT, 4);
        assert!(silent.starts_with("[----]"));
        assert!(meter(&Level { current_db: 0.0, peak_db: 0.0, clipping: true }, 4).ends_with("CLIP"));
    }

    #[test]
    fn status_lists_everything() {
        let live = LiveState {
            mute: false,
            gain: 100,
            monitor: true,
            headphone: 41,
            noise_cancellation: false,
            nc_strength: 34,
        };
        let settings = Settings {
            tone: Some(Tone::default()),
            lighting: Some(Lighting { color: RgbColor::Cyan, brightness: 62, enabled: true, linkage: true }),
            ..Settings::default()
        };
        let text = status(Some(&live), Some(14.6), &settings);
        assert!(text.contains("Gain          100 / 100"));
        assert!(text.contains("Boost         +14.6 dB"));
        assert!(text.contains("Tone          flat, compressor off, limiter off"));
        assert!(text.contains("Light         cyan, 62 %, off while muted"));
    }

    #[test]
    fn status_without_device_and_pipewire() {
        let text = status(None, None, &Settings::default());
        assert!(text.contains("not connected"));
        assert!(text.contains("unavailable"));
        assert!(text.contains("not set in the app"));
    }
}
