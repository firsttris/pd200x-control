//! The subcommands. Each opens what it needs: the microphone, PipeWire, the
//! settings (shared with the app, so both show the same tone and lighting).

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use pd200x_core::changes::{lighting_changes, tone_changes};
use pd200x_core::model::{Dynamics, Lighting, RgbColor, Settings, Tone, COMPRESSOR_MIN_DB, LIMITER_MIN_DB};
use pd200x_core::pipewire::{self, SystemRunner};
use pd200x_core::{calibrate, presets, sim, store, Change, Device};

use crate::{ui, Cmd, Toggle};

/// `PD200X_SIMULATOR=1` uses a simulated microphone (tests, trying without hardware).
fn open_device() -> Result<Device> {
    if std::env::var("PD200X_SIMULATOR").is_ok_and(|v| v == "1") {
        return Ok(Device::new(Box::new(sim::Simulator::with_defaults())));
    }
    Ok(Device::open_system()?)
}

fn config_dir() -> Result<PathBuf> {
    store::system_config_dir().context("neither XDG_CONFIG_HOME nor HOME is set")
}

fn update_settings(f: impl FnOnce(&mut Settings)) -> Result<Settings> {
    let dir = config_dir()?;
    let mut s = store::load(&dir)?;
    f(&mut s);
    store::save(&dir, &s)?;
    Ok(s)
}

fn apply_and_save_tone(device: &Device, change: impl FnOnce(&mut Tone)) -> Result<()> {
    let s = update_settings(|s| change(s.tone.get_or_insert_with(Tone::default)))?;
    device.apply_all(&tone_changes(s.tone.as_ref().expect("just set")))?;
    Ok(())
}

fn apply_and_save_light(device: &Device, change: impl FnOnce(&mut Lighting)) -> Result<()> {
    let s = update_settings(|s| change(s.lighting.get_or_insert_with(Lighting::default)))?;
    device.apply_all(&lighting_changes(s.lighting.as_ref().expect("just set")))?;
    Ok(())
}

/// `off` or a threshold in dB.
fn parse_dynamics(value: &str, min_db: f32) -> Result<Dynamics> {
    if value == "off" {
        return Ok(Dynamics { enabled: false, threshold_db: min_db / 2.0 });
    }
    let db: f32 = value.parse().map_err(|_| anyhow!("expected `off` or a threshold in dB, got `{value}`"))?;
    if !(min_db..=0.0).contains(&db) {
        bail!("the threshold must be between {min_db} and 0 dB");
    }
    Ok(Dynamics { enabled: true, threshold_db: db })
}

fn parse_color(name: &str) -> Option<RgbColor> {
    RgbColor::ALL.into_iter().find(|c| format!("{c:?}").eq_ignore_ascii_case(name))
}

fn light(device: &Device, args: &[String]) -> Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["on"] => apply_and_save_light(device, |l| l.enabled = true),
        ["off"] => apply_and_save_light(device, |l| l.enabled = false),
        ["brightness", v] => {
            let v: u8 = v.parse().ok().filter(|v| *v <= 100).context("brightness is 0–100")?;
            apply_and_save_light(device, |l| l.brightness = v)
        }
        ["linkage", v] => {
            let on = match *v {
                "on" => true,
                "off" => false,
                _ => bail!("linkage takes `on` or `off`"),
            };
            apply_and_save_light(device, |l| l.linkage = on)
        }
        [name] if parse_color(name).is_some() => {
            let c = parse_color(name).expect("checked");
            apply_and_save_light(device, |l| {
                l.color = c;
                l.enabled = true;
            })
        }
        _ => bail!(
            "usage: pd200x light on|off|<colour>|brightness <0–100>|linkage on|off\ncolours: white, red, orange, yellow, green, cyan, blue, purple, rainbow"
        ),
    }
}

fn eq(device: &Device, name: &str) -> Result<()> {
    let settings = store::load(&config_dir()?)?;
    if name == "list" {
        println!("Presets: {}", presets::ALL.map(|(id, _)| id).join(", "));
        if !settings.profiles.is_empty() {
            let names: Vec<_> = settings.profiles.iter().map(|p| p.name.as_str()).collect();
            println!("Profiles: {}", names.join(", "));
        }
        return Ok(());
    }
    let (source, bands) = if let Some(eq) = presets::find(name) {
        (name.to_string(), eq.to_vec())
    } else if let Some(p) = settings.profiles.iter().find(|p| p.name.eq_ignore_ascii_case(name) || p.id == name) {
        (p.id.clone(), p.eq.clone())
    } else {
        bail!("no preset or profile `{name}`; see `pd200x eq list`");
    };
    apply_and_save_tone(device, |t| {
        t.source = Some(source);
        t.eq = bands;
    })
}

fn level(device: &Device, seconds: u64) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(seconds);
    let mut out = std::io::stdout();
    while Instant::now() < end {
        write!(out, "\r{}", ui::meter(&device.level(), 40))?;
        out.flush()?;
        std::thread::sleep(Duration::from_millis(60));
    }
    writeln!(out)?;
    Ok(())
}

fn boost(value: Option<&str>) -> Result<()> {
    match value {
        None => println!("Boost: +{:.1} dB", pipewire::get_boost(&SystemRunner)?),
        Some("off") => pipewire::set_boost(&SystemRunner, 0.0)?,
        Some(v) => {
            let db: f32 = v.trim_start_matches('+').parse().map_err(|_| anyhow!("expected dB or `off`, got `{v}`"))?;
            pipewire::set_boost(&SystemRunner, db)?;
            update_settings(|s| s.boost_db = db)?;
        }
    }
    Ok(())
}

fn calibrate_cmd(device: &Device, seconds: u64, apply: bool) -> Result<()> {
    let live = device.read_live()?;
    let boost = pipewire::get_boost(&SystemRunner).unwrap_or(0.0);
    println!("Speak normally into the microphone for {seconds} seconds …");
    let values = device.capture(Duration::from_secs(seconds))?;
    let p = calibrate::propose(&values, live.gain, boost)?;
    println!("{}", ui::proposal(&p));
    if apply {
        device.apply(&Change::Gain(p.gain))?;
        pipewire::set_boost(&SystemRunner, p.boost_db)?;
        update_settings(|s| s.boost_db = p.boost_db)?;
        println!("Applied.");
    } else {
        println!("Run with --apply to use it.");
    }
    Ok(())
}

fn restore(device: &Device) -> Result<()> {
    let s = store::load(&config_dir()?)?;
    let mut changes = Vec::new();
    if let Some(t) = &s.tone {
        changes.extend(tone_changes(t));
    }
    if let Some(l) = &s.lighting {
        changes.extend(lighting_changes(l));
    }
    if changes.is_empty() {
        println!("Nothing saved yet.");
        return Ok(());
    }
    device.apply_all(&changes)?;
    println!("Restored tone and lighting.");
    Ok(())
}

pub fn run(cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::Status { json } => {
            let device = open_device();
            let live = device.as_ref().ok().map(|d| d.read_live()).transpose()?;
            let boost = pipewire::get_boost(&SystemRunner).ok();
            let settings = store::load(&config_dir()?)?;
            if json {
                let v = serde_json::json!({ "live": live, "boostDb": boost, "settings": settings });
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("{}", ui::status(live.as_ref(), boost, &settings));
            }
            Ok(())
        }
        Cmd::Boost { value } => boost(value.as_deref()),
        cmd => {
            let d = open_device()?;
            match cmd {
                Cmd::Level { seconds } => level(&d, seconds),
                Cmd::Gain { value } => Ok(d.apply(&Change::Gain(value))?),
                Cmd::Mute { state } => {
                    let on = match state {
                        Toggle::On => true,
                        Toggle::Off => false,
                        Toggle::Toggle => !d.read_live()?.mute,
                    };
                    Ok(d.apply(&Change::Mute(on))?)
                }
                Cmd::Monitor { state } => Ok(d.apply(&Change::Monitor(state.is_on()))?),
                Cmd::Headphone { value } => Ok(d.apply(&Change::Headphone(value))?),
                Cmd::Nc { value } => match value.as_str() {
                    "on" => Ok(d.apply(&Change::NoiseCancellation(true))?),
                    "off" => Ok(d.apply(&Change::NoiseCancellation(false))?),
                    v => {
                        let s: u8 = v.parse().map_err(|_| anyhow!("expected on, off or a strength 0–40"))?;
                        d.apply(&Change::NoiseCancellation(true))?;
                        Ok(d.apply(&Change::NcStrength(s))?)
                    }
                },
                Cmd::Eq { name } => eq(&d, &name),
                Cmd::Compressor { value } => {
                    let c = parse_dynamics(&value, COMPRESSOR_MIN_DB)?;
                    apply_and_save_tone(&d, |t| t.compressor = c)
                }
                Cmd::Limiter { value } => {
                    let l = parse_dynamics(&value, LIMITER_MIN_DB)?;
                    apply_and_save_tone(&d, |t| t.limiter = l)
                }
                Cmd::Light { args } => light(&d, &args),
                Cmd::Calibrate { seconds, apply } => calibrate_cmd(&d, seconds, apply),
                Cmd::Restore => restore(&d),
                Cmd::Status { .. } | Cmd::Boost { .. } => unreachable!("handled above"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamics_values() {
        assert_eq!(parse_dynamics("-9", COMPRESSOR_MIN_DB).unwrap(), Dynamics { enabled: true, threshold_db: -9.0 });
        assert!(!parse_dynamics("off", LIMITER_MIN_DB).unwrap().enabled);
        assert!(parse_dynamics("-50", COMPRESSOR_MIN_DB).is_err());
        assert!(parse_dynamics("loud", COMPRESSOR_MIN_DB).is_err());
    }

    #[test]
    fn colours_by_name() {
        assert_eq!(parse_color("Cyan"), Some(RgbColor::Cyan));
        assert_eq!(parse_color("rainbow"), Some(RgbColor::Rainbow));
        assert_eq!(parse_color("pink"), None);
    }
}
