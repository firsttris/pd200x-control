//! Test recordings: `pw-record` takes the microphone's PipeWire source (so
//! the system volume is in it, unlike the hardware monitoring), `pw-play`
//! plays it back. The analysis of the WAV file is pure.

use serde::Serialize;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::{Error, Result};

/// A recording stops by itself after this long.
pub const MAX_SECONDS: u32 = 30;
pub const SAMPLE_RATE: u32 = 48_000;
/// Bars in the waveform.
pub const WAVEFORM_POINTS: usize = 120;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    pub duration_ms: u32,
    /// dBFS of the loudest sample.
    pub peak_db: f32,
    /// dBFS of the RMS over the whole recording.
    pub mean_db: f32,
    /// [`WAVEFORM_POINTS`] values 0…1: the loudest sample in each slice.
    pub waveform: Vec<f32>,
}

/// The samples of a 16-bit PCM WAV file (mono, or the first channel).
pub fn parse_wav(bytes: &[u8]) -> Option<(u32, Vec<i16>)> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let (mut channels, mut rate, mut bits) = (1u16, SAMPLE_RATE, 16u16);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8;
        match id {
            b"fmt " if body + 16 <= bytes.len() => {
                channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
                rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().ok()?);
                bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            }
            b"data" => {
                if bits != 16 || channels == 0 {
                    return None;
                }
                // pw-record leaves the length at its maximum while it writes; take what is there.
                let end = body.saturating_add(len).min(bytes.len());
                let frame = usize::from(channels) * 2;
                let samples = bytes[body..end].chunks_exact(frame).map(|f| i16::from_le_bytes([f[0], f[1]])).collect();
                return Some((rate, samples));
            }
            _ => {}
        }
        at = body + len + (len & 1);
    }
    None
}

fn db(value: f32) -> f32 {
    if value <= 0.0 {
        return -96.0;
    }
    (20.0 * value.log10()).max(-96.0)
}

pub fn analyze(rate: u32, samples: &[i16]) -> RecordingInfo {
    let norm = |s: i16| f32::from(s).abs() / 32768.0;
    let peak = samples.iter().map(|&s| norm(s)).fold(0.0, f32::max);
    let rms = if samples.is_empty() {
        0.0
    } else {
        (samples.iter().map(|&s| (f64::from(s) / 32768.0).powi(2)).sum::<f64>() / samples.len() as f64).sqrt() as f32
    };
    let slice = samples.len().div_ceil(WAVEFORM_POINTS).max(1);
    let mut waveform: Vec<f32> =
        samples.chunks(slice).map(|c| c.iter().map(|&s| norm(s)).fold(0.0, f32::max)).collect();
    waveform.resize(WAVEFORM_POINTS, 0.0);
    RecordingInfo {
        duration_ms: (samples.len() as u64 * 1000 / u64::from(rate.max(1))) as u32,
        peak_db: (db(peak) * 10.0).round() / 10.0,
        mean_db: (db(rms) * 10.0).round() / 10.0,
        waveform: waveform.into_iter().map(|v| (v * 1000.0).round() / 1000.0).collect(),
    }
}

pub fn analyze_file(path: &Path) -> Result<RecordingInfo> {
    let bytes = std::fs::read(path)?;
    let (rate, samples) = parse_wav(&bytes)
        .ok_or_else(|| Error::ToolFailed { tool: "pw-record".into(), message: "unreadable recording".into() })?;
    Ok(analyze(rate, &samples))
}

/// A running `pw-record` or `pw-play`.
pub trait Process: Send {
    /// Asks it to finish (SIGINT: pw-record then completes the WAV header).
    fn interrupt(&mut self);
    /// `true` once it has exited.
    fn finished(&mut self) -> bool;
    /// Waits until it has exited.
    fn wait(&mut self);
}

/// Starts long-running programs; tests pass a fake.
pub trait Spawner: Send + Sync {
    fn spawn(&self, program: &str, args: &[String]) -> io::Result<Box<dyn Process>>;
}

pub struct SystemSpawner;

struct SystemProcess(Child);

impl Process for SystemProcess {
    fn interrupt(&mut self) {
        // SAFETY: kill with a pid this process started and has not reaped.
        unsafe {
            libc::kill(self.0.id() as libc::pid_t, libc::SIGINT);
        }
    }

    fn finished(&mut self) -> bool {
        !matches!(self.0.try_wait(), Ok(None))
    }

    fn wait(&mut self) {
        let _ = self.0.wait();
    }
}

impl Spawner for SystemSpawner {
    fn spawn(&self, program: &str, args: &[String]) -> io::Result<Box<dyn Process>> {
        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Box::new(SystemProcess(child)))
    }
}

fn spawn(spawner: &dyn Spawner, program: &str, args: Vec<String>) -> Result<Box<dyn Process>> {
    spawner.spawn(program, &args).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => Error::ToolMissing(program.into()),
        _ => e.into(),
    })
}

/// Records the PipeWire node `source` into `path` (16-bit mono, 48 kHz).
pub fn start_recording(spawner: &dyn Spawner, source: u32, path: &Path) -> Result<Box<dyn Process>> {
    let args = vec![
        "--target".into(),
        source.to_string(),
        "--format".into(),
        "s16".into(),
        "--rate".into(),
        SAMPLE_RATE.to_string(),
        "--channels".into(),
        "1".into(),
        path.display().to_string(),
    ];
    spawn(spawner, "pw-record", args)
}

/// Plays `path` on the default output.
pub fn start_playback(spawner: &dyn Spawner, path: &Path) -> Result<Box<dyn Process>> {
    spawn(spawner, "pw-play", vec![path.display().to_string()])
}

/// A WAV file with 16-bit mono samples (tests and the simulator).
pub fn wav(rate: u32, samples: &[i16]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: f32, n: usize) -> Vec<i16> {
        (0..n).map(|i| ((i as f32 * 0.05).sin() * amplitude * 32767.0) as i16).collect()
    }

    #[test]
    fn parses_a_wav_file() {
        let samples = tone(0.5, 4800);
        let (rate, back) = parse_wav(&wav(48_000, &samples)).unwrap();
        assert_eq!(rate, 48_000);
        assert_eq!(back, samples);
        assert_eq!(parse_wav(b"nope"), None);
    }

    #[test]
    fn a_wav_still_being_written_is_read_as_far_as_it_goes() {
        let mut bytes = wav(48_000, &tone(0.5, 100));
        let len_at = bytes.len() - 200 - 4;
        bytes[len_at..len_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_wav(&bytes).unwrap().1.len(), 100);
    }

    #[test]
    fn analysis_in_db() {
        // Half scale: peak about −6 dB, a sine's RMS 3 dB lower
        let info = analyze(48_000, &tone(0.5, 48_000));
        assert_eq!(info.duration_ms, 1000);
        assert!((info.peak_db - -6.0).abs() < 0.2, "{info:?}");
        assert!((info.mean_db - -9.0).abs() < 0.3, "{info:?}");
        assert_eq!(info.waveform.len(), WAVEFORM_POINTS);
        assert!(info.waveform.iter().all(|v| (0.0..=0.51).contains(v)));
    }

    #[test]
    fn silence_and_empty() {
        let info = analyze(48_000, &[0; 480]);
        assert_eq!((info.peak_db, info.mean_db), (-96.0, -96.0));
        assert_eq!(analyze(48_000, &[]).duration_ms, 0);
    }

    #[test]
    fn missing_tools_are_named() {
        struct Missing;
        impl Spawner for Missing {
            fn spawn(&self, _: &str, _: &[String]) -> io::Result<Box<dyn Process>> {
                Err(io::ErrorKind::NotFound.into())
            }
        }
        let err = start_recording(&Missing, 105, Path::new("/tmp/x.wav")).err().unwrap();
        assert!(matches!(err, Error::ToolMissing(t) if t == "pw-record"));
    }
}
