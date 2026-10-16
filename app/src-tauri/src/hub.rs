//! Everything the app does, over a [`Hub`]: the open microphone, the
//! settings and PipeWire. The Tauri commands and the tray are thin wrappers,
//! so this is testable with the simulator and a fake PipeWire.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use pd200x_core::calibrate::{self, Proposal};
use pd200x_core::changes::{lighting_changes, tone_changes};
use pd200x_core::level::Level;
use pd200x_core::model::{Lighting, LiveState, Profile, Settings, Tone, EQ_BANDS};
use pd200x_core::pipewire::{self, Runner};
use pd200x_core::{presets, store, Change, Device, Error};

/// Error as the frontend sees it: a stable kind it translates, plus the
/// English text for details.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CmdError {
    pub kind: String,
    pub message: String,
}

impl From<Error> for CmdError {
    fn from(e: Error) -> Self {
        CmdError { kind: e.kind().to_string(), message: e.to_string() }
    }
}

impl CmdError {
    pub fn other(detail: impl Into<String>) -> Self {
        CmdError { kind: "other".into(), message: detail.into() }
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

/// What the window shows; polled about once a second.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// `None` while no microphone is open.
    pub live: Option<LiveState>,
    /// Why there is no microphone (an error kind), `None` when connected.
    pub problem: Option<String>,
    /// The software boost in dB; `None` when PipeWire has no source for it.
    pub boost_db: Option<f32>,
    /// EQ and dynamics as the microphone has them (read when it connects).
    pub tone: Option<Tone>,
    /// The ring light as the microphone has it.
    pub lighting: Option<Lighting>,
    pub settings: Settings,
}

/// A built-in EQ preset; the frontend shows a translated name for the id.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PresetInfo {
    pub id: String,
    pub eq: Vec<pd200x_core::model::EqBand>,
}

pub fn presets_info() -> Vec<PresetInfo> {
    presets::ALL.iter().map(|(id, eq)| PresetInfo { id: (*id).to_string(), eq: eq.to_vec() }).collect()
}

pub type Opener = Box<dyn Fn() -> pd200x_core::Result<Device> + Send + Sync>;

pub struct Hub {
    device: Mutex<Option<Arc<Device>>>,
    problem: Mutex<Option<String>>,
    settings: Mutex<Settings>,
    boost: Mutex<Option<f32>>,
    /// Tone and lighting on the device: read on connect, kept up to date by `apply`.
    onboard: Mutex<Option<(Tone, Lighting)>>,
    config_dir: PathBuf,
    runner: Box<dyn Runner>,
    open: Opener,
}

impl Hub {
    pub fn new(config_dir: PathBuf, runner: Box<dyn Runner>, open: Opener) -> Hub {
        let settings = store::load(&config_dir).unwrap_or_default();
        Hub {
            device: Mutex::new(None),
            problem: Mutex::new(None),
            settings: Mutex::new(settings),
            boost: Mutex::new(None),
            onboard: Mutex::new(None),
            config_dir,
            runner,
            open,
        }
    }

    /// Opens the microphone if it is not open (or was unplugged); after
    /// connecting, sends the saved tone and lighting when enabled. Called by
    /// the watcher thread every second.
    pub fn connect(&self) {
        let mut device = self.device.lock().unwrap();
        if device.as_ref().is_some_and(|d| d.is_alive()) {
            return;
        }
        *device = None;
        match (self.open)() {
            Ok(d) => {
                let settings = self.settings.lock().unwrap().clone();
                if settings.restore_on_connect {
                    // A failed restore is not worth losing the connection over.
                    let _ = d.apply_all(&saved_changes(&settings));
                }
                let linkage = settings.lighting.is_some_and(|l| l.linkage);
                let onboard = d.read_tone().and_then(|mut tone| {
                    tone.source = presets::detect(&tone.eq, &settings);
                    Ok((tone, d.read_lighting(linkage)?))
                });
                *self.onboard.lock().unwrap() = onboard.ok();
                *device = Some(Arc::new(d));
                *self.problem.lock().unwrap() = None;
            }
            Err(e) => *self.problem.lock().unwrap() = Some(e.kind().to_string()),
        }
    }

    /// Reads the boost from PipeWire again (it can change in KDE's settings too).
    pub fn refresh_boost(&self) {
        let boost = pipewire::get_boost(self.runner.as_ref()).ok();
        *self.boost.lock().unwrap() = boost;
    }

    /// The open microphone; the lock is released right away, so a long
    /// capture does not block the level or the snapshot.
    fn current(&self) -> CmdResult<Arc<Device>> {
        self.device.lock().unwrap().clone().ok_or_else(|| {
            let kind = self.problem.lock().unwrap().clone().unwrap_or_else(|| "notFound".into());
            CmdError { kind, message: "no microphone".into() }
        })
    }

    fn with_device<T>(&self, f: impl FnOnce(&Device) -> pd200x_core::Result<T>) -> CmdResult<T> {
        let device = self.current()?;
        f(&device).map_err(CmdError::from)
    }

    fn change_settings<T>(&self, f: impl FnOnce(&mut Settings) -> CmdResult<T>) -> CmdResult<T> {
        let mut settings = self.settings.lock().unwrap();
        let mut next = settings.clone();
        let out = f(&mut next)?;
        store::save(&self.config_dir, &next)?;
        *settings = next;
        Ok(out)
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn snapshot(&self) -> Snapshot {
        let live = self.with_device(Device::read_live).ok();
        let onboard = self.onboard.lock().unwrap().clone();
        Snapshot {
            live,
            problem: self.problem.lock().unwrap().clone(),
            boost_db: *self.boost.lock().unwrap(),
            tone: onboard.as_ref().map(|(t, _)| t.clone()),
            lighting: onboard.map(|(_, l)| l),
            settings: self.settings(),
        }
    }

    pub fn level(&self) -> Level {
        self.current().map_or(Level::SILENT, |d| d.level())
    }

    /// Applies a change and remembers what the device cannot report.
    pub fn apply(&self, change: Change) -> CmdResult<()> {
        self.with_device(|d| d.apply(&change))?;
        if let Some((tone, lighting)) = self.onboard.lock().unwrap().as_mut() {
            let mut s = Settings { tone: Some(tone.clone()), lighting: Some(*lighting), ..Settings::default() };
            remember(&mut s, &change);
            *tone = s.tone.unwrap_or_default();
            *lighting = s.lighting.unwrap_or_default();
        }
        self.change_settings(|s| {
            remember(s, &change);
            Ok(())
        })
    }

    /// The tone as it is now: on the device, else as saved, else the default.
    fn current_tone(&self) -> Tone {
        let onboard = self.onboard.lock().unwrap().as_ref().map(|(t, _)| t.clone());
        onboard.or_else(|| self.settings().tone).unwrap_or_default()
    }

    /// Mute on or off, whatever it is now (tray menu).
    pub fn toggle_mute(&self) -> CmdResult<bool> {
        let mute = !self.with_device(Device::read_live)?.mute;
        self.apply(Change::Mute(mute))?;
        Ok(mute)
    }

    /// A built-in preset (`flat`, `natural`, …) or the id of an own profile.
    pub fn apply_eq(&self, id: &str) -> CmdResult<()> {
        let eq = match presets::find(id) {
            Some(eq) => eq.to_vec(),
            None => self
                .settings()
                .profiles
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.eq.clone())
                .ok_or_else(|| CmdError::from(Error::InvalidValue(format!("profile {id}"))))?,
        };
        let tone = Tone { source: Some(id.to_string()), eq, ..self.current_tone() };
        self.with_device(|d| d.apply_all(&tone_changes(&tone)))?;
        if let Some((t, _)) = self.onboard.lock().unwrap().as_mut() {
            *t = tone.clone();
        }
        self.change_settings(|s| {
            s.tone = Some(tone);
            Ok(())
        })
    }

    /// Saves the current EQ under `name`.
    pub fn save_profile(&self, name: &str) -> CmdResult<Profile> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::InvalidValue("empty name".into()).into());
        }
        let tone = self.current_tone();
        let id = self.change_settings(|s| {
            let id = unique_id(name, &s.profiles);
            s.profiles.push(Profile { id: id.clone(), name: name.to_string(), eq: tone.eq.clone() });
            s.tone = Some(Tone { source: Some(id.clone()), ..tone.clone() });
            Ok(id)
        })?;
        if let Some((t, _)) = self.onboard.lock().unwrap().as_mut() {
            t.source = Some(id.clone());
        }
        Ok(Profile { id, name: name.to_string(), eq: tone.eq })
    }

    pub fn rename_profile(&self, id: &str, name: &str) -> CmdResult<()> {
        let name = name.trim().to_string();
        self.change_settings(|s| {
            let p = s.profiles.iter_mut().find(|p| p.id == id);
            match p {
                Some(p) if !name.is_empty() => {
                    p.name = name;
                    Ok(())
                }
                _ => Err(Error::InvalidValue(format!("profile {id}")).into()),
            }
        })
    }

    pub fn delete_profile(&self, id: &str) -> CmdResult<()> {
        self.change_settings(|s| {
            s.profiles.retain(|p| p.id != id);
            if let Some(t) = s.tone.as_mut().filter(|t| t.source.as_deref() == Some(id)) {
                t.source = None;
            }
            Ok(())
        })
    }

    /// Sets the software boost; values above 0 are remembered for switching it on again.
    pub fn set_boost(&self, db: f32) -> CmdResult<()> {
        pipewire::set_boost(self.runner.as_ref(), db)?;
        *self.boost.lock().unwrap() = Some(db);
        if db > 0.0 {
            self.change_settings(|s| {
                s.boost_db = db;
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Records `seconds` of speech and proposes gain and boost; changes nothing.
    pub fn calibrate(&self, seconds: u64) -> CmdResult<Proposal> {
        let device = self.current()?;
        let gain = device.read_live()?.gain;
        let boost = self.boost.lock().unwrap().unwrap_or(0.0);
        let values = device.capture(Duration::from_secs(seconds))?;
        Ok(calibrate::propose(&values, gain, boost)?)
    }

    pub fn apply_calibration(&self, gain: u8, boost_db: f32) -> CmdResult<()> {
        self.apply(Change::Gain(gain))?;
        self.set_boost(boost_db)
    }

    pub fn set_restore_on_connect(&self, on: bool) -> CmdResult<()> {
        self.change_settings(|s| {
            s.restore_on_connect = on;
            Ok(())
        })
    }
}

/// Tone and lighting as saved, as device changes.
fn saved_changes(s: &Settings) -> Vec<Change> {
    let mut out = Vec::new();
    if let Some(t) = &s.tone {
        out.extend(tone_changes(t));
    }
    if let Some(l) = &s.lighting {
        out.extend(lighting_changes(l));
    }
    out
}

/// Keeps tone and lighting in the settings.
fn remember(s: &mut Settings, change: &Change) {
    match *change {
        Change::EqBand { index, band } => {
            let tone = s.tone.get_or_insert_with(Tone::default);
            tone.eq.resize(EQ_BANDS, band);
            tone.eq[usize::from(index)] = band.clamped();
            tone.source = None;
        }
        Change::Compressor(d) => s.tone.get_or_insert_with(Tone::default).compressor = d,
        Change::Limiter(d) => s.tone.get_or_insert_with(Tone::default).limiter = d,
        Change::RgbEnabled(on) => s.lighting.get_or_insert_with(Default::default).enabled = on,
        Change::RgbBrightness(v) => s.lighting.get_or_insert_with(Default::default).brightness = v,
        Change::RgbColor(c) => s.lighting.get_or_insert_with(Default::default).color = c,
        Change::Linkage(on) => s.lighting.get_or_insert_with(Default::default).linkage = on,
        _ => {}
    }
}

/// `podcast`, `podcast-2`, … from the name.
fn unique_id(name: &str, profiles: &[Profile]) -> String {
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() || presets::find(&base).is_some() { format!("profile-{base}") } else { base };
    let taken = |id: &str| profiles.iter().any(|p| p.id == id);
    if !taken(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|id| !taken(id)).expect("some number is free")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd200x_core::model::{Dynamics, RgbColor};
    use pd200x_core::protocol::reg;
    use pd200x_core::sim::Simulator;

    #[test]
    fn ids_from_names() {
        let p = |id: &str| Profile { id: id.into(), name: id.into(), eq: vec![] };
        assert_eq!(unique_id("Podcast (eigen)", &[]), "podcast-eigen");
        assert_eq!(unique_id("Podcast", &[p("podcast")]), "podcast-2");
        assert_eq!(unique_id("Natural", &[]), "profile-natural");
        assert_eq!(unique_id("!!!", &[]), "profile-");
    }

    #[test]
    fn remember_keeps_tone_and_lighting() {
        let mut s = Settings::default();
        remember(&mut s, &Change::RgbColor(RgbColor::Blue));
        remember(&mut s, &Change::Compressor(Dynamics { enabled: true, threshold_db: -9.0 }));
        let band = presets::NATURAL[2];
        remember(&mut s, &Change::EqBand { index: 2, band });
        remember(&mut s, &Change::Gain(10));
        assert_eq!(s.lighting.unwrap().color, RgbColor::Blue);
        let tone = s.tone.unwrap();
        assert_eq!(tone.compressor.threshold_db, -9.0);
        assert_eq!(tone.eq[2], band);
        assert_eq!(tone.source, None);
    }

    #[test]
    fn reads_the_device_state_on_connect() {
        let tmp = tempfile::tempdir().unwrap();
        let sim = Simulator::new();
        sim.set(reg::RGB_COLOR, RgbColor::Cyan.code());
        sim.set(reg::RGB_BRIGHTNESS, 70);
        let s2 = sim.clone();
        let hub = Hub::new(
            tmp.path().to_path_buf(),
            Box::new(pipewire::SystemRunner),
            Box::new(move || Ok(Device::new(Box::new(s2.clone())))),
        );
        hub.connect();
        let snap = hub.snapshot();
        let light = snap.lighting.unwrap();
        assert_eq!((light.color, light.brightness), (RgbColor::Cyan, 70));
        assert_eq!(snap.tone.unwrap().source.as_deref(), Some("flat"));
        hub.apply(Change::RgbColor(RgbColor::Red)).unwrap();
        assert_eq!(hub.snapshot().lighting.unwrap().color, RgbColor::Red);
        hub.apply_eq("deep").unwrap();
        assert_eq!(hub.snapshot().tone.unwrap().source.as_deref(), Some("deep"));
    }

    #[test]
    fn restores_saved_settings_on_connect() {
        let tmp = tempfile::tempdir().unwrap();
        let mut settings = Settings { restore_on_connect: true, ..Settings::default() };
        remember(&mut settings, &Change::RgbColor(RgbColor::Green));
        store::save(tmp.path(), &settings).unwrap();
        let sim = Simulator::new();
        let s2 = sim.clone();
        let hub = Hub::new(
            tmp.path().to_path_buf(),
            Box::new(pipewire::SystemRunner),
            Box::new(move || Ok(Device::new(Box::new(s2.clone())))),
        );
        hub.connect();
        assert_eq!(sim.get(reg::RGB_COLOR), RgbColor::Green.code());
    }
}
