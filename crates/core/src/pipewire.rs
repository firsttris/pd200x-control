//! The software boost: the volume of the microphone's PipeWire source above
//! 100 %. WirePlumber keeps the volume across restarts by itself.
//!
//! `wpctl` volumes are cubic: 1.75 means an amplitude of 1.75³ ≈ 5.36, that is
//! +14.6 dB (`pw-dump` shows the cube as `channelVolumes`).

use std::io;
use std::process::Command;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program; tests pass a fake.
pub trait Runner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<Output>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<Output> {
        let out = Command::new(program).args(args).output()?;
        Ok(Output {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// The boost never goes beyond this (2.0 = +18 dB).
pub const MAX_BOOST_DB: f32 = 18.0;

pub fn volume_to_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        return f32::NEG_INFINITY;
    }
    60.0 * volume.log10()
}

pub fn db_to_volume(db: f32) -> f32 {
    10f32.powf(db / 60.0)
}

/// The id of the microphone's capture node in `pw-dump` output.
pub fn find_source(pw_dump: &str) -> Option<u32> {
    let objects: Vec<serde_json::Value> = serde_json::from_str(pw_dump).ok()?;
    objects.iter().find_map(|o| {
        let props = o.pointer("/info/props")?;
        let class = props.get("media.class")?.as_str()?;
        let name = props.get("node.name")?.as_str()?.to_ascii_lowercase();
        if class != "Audio/Source" || !name.contains("352f") || !name.contains("pd200x") {
            return None;
        }
        u32::try_from(o.get("id")?.as_u64()?).ok()
    })
}

/// The id from `wpctl status -n`, for systems without `pw-dump`:
/// ` │  *  105. alsa_input.usb-352f_PD200X_…mono-fallback [vol: 1.75]`.
pub fn find_source_in_status(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        let name_at = lower.find("alsa_input.")?;
        if !(lower[name_at..].contains("352f") && lower[name_at..].contains("pd200x")) {
            return None;
        }
        let before = line[..name_at].trim_end().strip_suffix('.')?;
        let digits: String = before.chars().rev().take_while(char::is_ascii_digit).collect();
        digits.chars().rev().collect::<String>().parse().ok()
    })
}

/// `Volume: 1.75` or `Volume: 1.75 [MUTED]`.
pub fn parse_volume(wpctl: &str) -> Option<f32> {
    wpctl.split_whitespace().skip_while(|w| *w != "Volume:").nth(1)?.parse().ok()
}

fn run(runner: &dyn Runner, program: &str, args: &[&str]) -> Result<String> {
    match runner.run(program, args) {
        Ok(out) if out.success => Ok(out.stdout),
        Ok(out) => Err(Error::ToolFailed { tool: program.into(), message: out.stderr.trim().to_string() }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Error::ToolMissing(program.into())),
        Err(e) => Err(e.into()),
    }
}

/// From `pw-dump`, or from `wpctl status -n` where `pw-dump` is not installed.
pub fn source_id(runner: &dyn Runner) -> Result<u32> {
    match run(runner, "pw-dump", &[]) {
        Ok(dump) => find_source(&dump).ok_or(Error::NoAudioSource),
        Err(Error::ToolMissing(_)) => {
            find_source_in_status(&run(runner, "wpctl", &["status", "-n"])?).ok_or(Error::NoAudioSource)
        }
        Err(e) => Err(e),
    }
}

/// The current boost in dB (0 when the volume is at or below 100 %).
pub fn get_boost(runner: &dyn Runner) -> Result<f32> {
    let id = source_id(runner)?.to_string();
    let out = run(runner, "wpctl", &["get-volume", &id])?;
    let volume = parse_volume(&out)
        .ok_or_else(|| Error::ToolFailed { tool: "wpctl".into(), message: format!("unexpected output: {out}") })?;
    Ok((volume_to_db(volume).max(0.0) * 10.0).round() / 10.0)
}

/// Sets the boost (0 = 100 %).
pub fn set_boost(runner: &dyn Runner, db: f32) -> Result<()> {
    if !(0.0..=MAX_BOOST_DB).contains(&db) {
        return Err(Error::InvalidValue(format!("boost 0–{MAX_BOOST_DB} dB")));
    }
    let id = source_id(runner)?.to_string();
    let volume = format!("{:.4}", db_to_volume(db));
    run(runner, "wpctl", &["set-volume", &id, &volume]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const DUMP: &str = r#"[
      {"id": 43, "type": "PipeWire:Interface:Node", "info": {"props": {"media.class": "Audio/Sink", "node.name": "alsa_output.pci"}}},
      {"id": 106, "type": "PipeWire:Interface:Node", "info": {"props": {"media.class": "Audio/Source", "node.name": "alsa_input.usb-046d_Logitech_StreamCam"}}},
      {"id": 105, "type": "PipeWire:Interface:Node", "info": {"props": {"media.class": "Audio/Source", "node.name": "alsa_input.usb-352f_PD200X_Podcast_Microphone_214b206000000178-00.mono-fallback"}}},
      {"id": 7, "type": "PipeWire:Interface:Core", "info": {}}
    ]"#;

    struct Fake {
        calls: Mutex<Vec<String>>,
        volume: &'static str,
    }

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[&str]) -> io::Result<Output> {
            self.calls.lock().unwrap().push(format!("{program} {}", args.join(" ")));
            let stdout = match program {
                "pw-dump" => DUMP.to_string(),
                "wpctl" if args[0] == "get-volume" => self.volume.to_string(),
                _ => String::new(),
            };
            Ok(Output { success: true, stdout, stderr: String::new() })
        }
    }

    fn fake(volume: &'static str) -> Fake {
        Fake { calls: Mutex::new(Vec::new()), volume }
    }

    #[test]
    fn cubic_volume_and_db() {
        assert!((volume_to_db(1.75) - 14.58).abs() < 0.01);
        assert!((db_to_volume(14.58) - 1.75).abs() < 0.001);
        assert_eq!(volume_to_db(1.0), 0.0);
        assert_eq!(volume_to_db(0.0), f32::NEG_INFINITY);
    }

    #[test]
    fn finds_the_pd200x_source() {
        assert_eq!(find_source(DUMP), Some(105));
        assert_eq!(find_source("[]"), None);
        assert_eq!(find_source("not json"), None);
    }

    #[test]
    fn finds_the_source_in_wpctl_status() {
        let status = " │     100. alsa_card.usb-352f_PD200X_Podcast_Microphone_214b206000000178-00 [alsa]\n \
                      │  *  105. alsa_input.usb-352f_PD200X_Podcast_Microphone_214b206000000178-00.mono-fallback [vol: 1.75]\n \
                      │     106. alsa_input.usb-046d_Logitech_StreamCam [vol: 1.00]\n";
        assert_eq!(find_source_in_status(status), Some(105));
        assert_eq!(find_source_in_status("nothing"), None);
    }

    #[test]
    fn parses_wpctl_volume() {
        assert_eq!(parse_volume("Volume: 1.75\n"), Some(1.75));
        assert_eq!(parse_volume("Volume: 0.40 [MUTED]"), Some(0.40));
        assert_eq!(parse_volume("garbage"), None);
    }

    #[test]
    fn reads_and_sets_the_boost() {
        let f = fake("Volume: 1.75\n");
        assert_eq!(get_boost(&f).unwrap(), 14.6);
        set_boost(&f, 12.0).unwrap();
        assert_eq!(f.calls.lock().unwrap().last().unwrap(), "wpctl set-volume 105 1.5849");
        assert!(set_boost(&f, 30.0).is_err());
    }

    #[test]
    fn quieter_than_100_percent_is_no_boost() {
        assert_eq!(get_boost(&fake("Volume: 0.80")).unwrap(), 0.0);
    }

    #[test]
    fn missing_tools_are_named() {
        struct Missing;
        impl Runner for Missing {
            fn run(&self, _: &str, _: &[&str]) -> io::Result<Output> {
                Err(io::ErrorKind::NotFound.into())
            }
        }
        assert!(matches!(get_boost(&Missing), Err(Error::ToolMissing(t)) if t == "wpctl"));
    }
}
