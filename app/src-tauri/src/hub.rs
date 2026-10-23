//! Everything the app does, over a [`Hub`]: the open microphone, the
//! settings and PipeWire. The Tauri commands and the tray are thin wrappers,
//! so this is testable with the simulator and a fake PipeWire.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;

use pd200x_core::calibrate::{self, Proposal};
use pd200x_core::changes::{lighting_changes, tone_changes};
use pd200x_core::level::Level;
use pd200x_core::model::{Lighting, LiveState, Profile, Settings, Tone, EQ_BANDS};
use pd200x_core::pipewire::{self, Runner};
use pd200x_core::recording::{self, Process, RecordingInfo, Spawner};
use pd200x_core::{flatpak, presets, store, Change, Device, Error};

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
    /// Apps (Teams, Slack, browsers) may not change the microphone volume.
    pub protected: bool,
    /// The app runs in the Flatpak sandbox: the user restarts pipewire-pulse
    /// after switching `protected`.
    pub sandboxed: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RecordingState {
    Idle,
    Recording,
    Ready,
    Playing,
}

/// The test recording, polled while its card is on screen.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub state: RecordingState,
    /// Since the recording or the playback started.
    pub elapsed_ms: u32,
    /// The last finished recording.
    pub info: Option<RecordingInfo>,
}

enum Rec {
    Idle,
    Recording { process: Box<dyn Process>, started: Instant },
    Ready(RecordingInfo),
    Playing { process: Box<dyn Process>, started: Instant, info: RecordingInfo },
}

pub type Opener = Box<dyn Fn() -> pd200x_core::Result<Device> + Send + Sync>;

/// The system around the app: where PipeWire reads its drop-ins and whether
/// the app runs in the Flatpak sandbox.
#[derive(Debug, Clone)]
pub struct Host {
    /// The host's `~/.config`.
    pub config_home: Option<PathBuf>,
    pub sandboxed: bool,
}

impl Host {
    pub fn system() -> Host {
        Host { config_home: pipewire::system_config_home(), sandboxed: flatpak::is_sandboxed() }
    }
}

pub struct Hub {
    device: Mutex<Option<Arc<Device>>>,
    problem: Mutex<Option<String>>,
    settings: Mutex<Settings>,
    boost: Mutex<Option<f32>>,
    /// Tone and lighting on the device: read on connect, kept up to date by `apply`.
    onboard: Mutex<Option<(Tone, Lighting)>>,
    config_dir: PathBuf,
    host: Host,
    runner: Box<dyn Runner>,
    spawner: Box<dyn Spawner>,
    rec: Mutex<Rec>,
    rec_path: PathBuf,
    recording_dir: tempfile::TempDir,
    open: Opener,
}

impl Hub {
    pub fn new(
        config_dir: PathBuf,
        host: Host,
        runner: Box<dyn Runner>,
        spawner: Box<dyn Spawner>,
        open: Opener,
    ) -> Hub {
        let settings = store::load(&config_dir).unwrap_or_default();
        let recording_dir = tempfile::Builder::new()
            .prefix("pd200x-control-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("private recording directory");
        Hub {
            device: Mutex::new(None),
            problem: Mutex::new(None),
            settings: Mutex::new(settings),
            boost: Mutex::new(None),
            onboard: Mutex::new(None),
            config_dir,
            host,
            runner,
            spawner,
            rec: Mutex::new(Rec::Idle),
            rec_path: recording_dir.path().join("recording.wav"),
            recording_dir,
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
            protected: self.host.config_home.as_deref().is_some_and(pipewire::is_protected),
            sandboxed: self.host.sandboxed,
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

    /// Keeps apps (their automatic gain control) from changing the microphone
    /// volume. The drop-in goes to the host's config folder, also from the
    /// Flatpak, whose own `~/.config` is not the one PipeWire reads.
    pub fn set_protected(&self, on: bool) -> CmdResult<()> {
        let home = self.host.config_home.as_deref().ok_or_else(|| CmdError::other("no config folder"))?;
        pipewire::set_protected(self.runner.as_ref(), home, on, !self.host.sandboxed)?;
        Ok(())
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

impl Hub {
    /// Starts a test recording from the microphone's PipeWire source; stops a playback first.
    pub fn start_recording(&self) -> CmdResult<()> {
        let mut rec = self.rec.lock().unwrap();
        finish(&mut rec);
        let source = pipewire::source_id(self.runner.as_ref())?;
        let _ = std::fs::remove_file(&self.rec_path);
        let process = recording::start_recording(self.spawner.as_ref(), source, &self.rec_path)?;
        *rec = Rec::Recording { process, started: Instant::now() };
        Ok(())
    }

    /// Stops the recording or the playback, whichever runs.
    pub fn stop_recording(&self) -> CmdResult<RecordingStatus> {
        let mut rec = self.rec.lock().unwrap();
        self.stop(&mut rec)?;
        Ok(status_of(&rec))
    }

    fn stop(&self, rec: &mut Rec) -> CmdResult<()> {
        match std::mem::replace(rec, Rec::Idle) {
            Rec::Recording { mut process, .. } => {
                process.interrupt();
                process.wait();
                *rec = Rec::Ready(recording::analyze_file(&self.rec_path)?);
            }
            Rec::Playing { mut process, info, .. } => {
                process.interrupt();
                process.wait();
                *rec = Rec::Ready(info);
            }
            other => *rec = other,
        }
        Ok(())
    }

    pub fn play_recording(&self) -> CmdResult<()> {
        let mut rec = self.rec.lock().unwrap();
        self.stop(&mut rec)?;
        let Rec::Ready(info) = &*rec else {
            return Err(Error::InvalidValue("no recording".into()).into());
        };
        let info = info.clone();
        let process = recording::start_playback(self.spawner.as_ref(), &self.rec_path)?;
        *rec = Rec::Playing { process, started: Instant::now(), info };
        Ok(())
    }

    /// Where the recording stands; stops it at the time limit and notices
    /// when a playback has ended.
    pub fn recording_status(&self) -> CmdResult<RecordingStatus> {
        let mut rec = self.rec.lock().unwrap();
        let over = match &mut *rec {
            Rec::Recording { started, .. } => started.elapsed() >= Duration::from_secs(recording::MAX_SECONDS.into()),
            Rec::Playing { process, .. } => process.finished(),
            _ => false,
        };
        if over {
            self.stop(&mut rec)?;
        }
        Ok(status_of(&rec))
    }

    /// Stops what runs and deletes the file (when the app quits).
    pub fn cleanup(&self) {
        let mut rec = self.rec.lock().unwrap();
        finish(&mut rec);
        let _ = std::fs::remove_file(&self.rec_path);
        let _ = std::fs::remove_dir(self.recording_dir.path());
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Ends a running process without analysing; a finished recording stays.
fn finish(rec: &mut Rec) {
    match std::mem::replace(rec, Rec::Idle) {
        Rec::Recording { mut process, .. } => {
            process.interrupt();
            process.wait();
        }
        Rec::Playing { mut process, info, .. } => {
            process.interrupt();
            process.wait();
            *rec = Rec::Ready(info);
        }
        other => *rec = other,
    }
}

fn status_of(rec: &Rec) -> RecordingStatus {
    let ms = |t: &Instant| t.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    match rec {
        Rec::Idle => RecordingStatus { state: RecordingState::Idle, elapsed_ms: 0, info: None },
        Rec::Recording { started, .. } => {
            RecordingStatus { state: RecordingState::Recording, elapsed_ms: ms(started), info: None }
        }
        Rec::Ready(info) => RecordingStatus { state: RecordingState::Ready, elapsed_ms: 0, info: Some(info.clone()) },
        Rec::Playing { started, info, .. } => {
            RecordingStatus { state: RecordingState::Playing, elapsed_ms: ms(started), info: Some(info.clone()) }
        }
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

    struct FakeProcess {
        done: bool,
    }

    impl Process for FakeProcess {
        fn interrupt(&mut self) {
            self.done = true;
        }
        fn finished(&mut self) -> bool {
            self.done
        }
        fn wait(&mut self) {}
    }

    /// pw-record writes a second of a −6 dB tone to its file; pw-play ends at once.
    struct FakeSpawner;

    impl Spawner for FakeSpawner {
        fn spawn(&self, program: &str, args: &[String]) -> std::io::Result<Box<dyn Process>> {
            if program == "pw-record" {
                let samples: Vec<i16> = (0..48_000).map(|i| ((i as f32 * 0.05).sin() * 16_384.0) as i16).collect();
                std::fs::write(args.last().unwrap(), recording::wav(48_000, &samples))?;
            }
            Ok(Box::new(FakeProcess { done: program == "pw-play" }))
        }
    }

    struct FakePipeWire;

    impl Runner for FakePipeWire {
        fn run(&self, program: &str, _: &[&str]) -> std::io::Result<pipewire::Output> {
            let stdout = if program == "pw-dump" {
                r#"[{"id": 105, "info": {"props": {"media.class": "Audio/Source", "node.name": "alsa_input.usb-352f_PD200X"}}}]"#
            } else {
                "Volume: 1.0"
            };
            Ok(pipewire::Output { success: true, stdout: stdout.into(), stderr: String::new() })
        }
    }

    fn native() -> Host {
        Host { config_home: None, sandboxed: false }
    }

    #[test]
    fn recording_directories_are_private_and_unique() {
        let config = tempfile::tempdir().unwrap();
        let make = || {
            Hub::new(
                config.path().to_path_buf(),
                native(),
                Box::new(NoPrograms),
                Box::new(FakeSpawner),
                Box::new(|| Err(Error::NotFound)),
            )
        };
        let first = make();
        let second = make();
        assert_ne!(first.rec_path.parent(), second.rec_path.parent());
        let mode = std::fs::metadata(first.rec_path.parent().unwrap()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn cleanup_removes_the_recording_directory_while_hub_is_shared() {
        let config = tempfile::tempdir().unwrap();
        let hub = Arc::new(Hub::new(
            config.path().to_path_buf(),
            native(),
            Box::new(FakePipeWire),
            Box::new(FakeSpawner),
            Box::new(|| Err(Error::NotFound)),
        ));
        let watcher = hub.clone();
        hub.start_recording().unwrap();
        let path = hub.rec_path.clone();
        let directory = path.parent().unwrap().to_path_buf();
        assert!(path.exists());
        hub.cleanup();
        assert_eq!(watcher.recording_status().unwrap().state, RecordingState::Idle);
        assert!(!path.exists());
        assert!(!directory.exists());
    }

    /// Every program fails, so a test notices when one is run.
    struct NoPrograms;

    impl Runner for NoPrograms {
        fn run(&self, program: &str, _: &[&str]) -> std::io::Result<pipewire::Output> {
            Err(std::io::Error::other(format!("{program} must not run")))
        }
    }

    #[test]
    fn protection_in_the_flatpak_writes_to_the_host_config_without_restarting() {
        let app_config = tempfile::tempdir().unwrap();
        let host_config = tempfile::tempdir().unwrap();
        let host = Host { config_home: Some(host_config.path().to_path_buf()), sandboxed: true };
        let hub = Hub::new(
            app_config.path().join("pd200x-control"),
            host,
            Box::new(NoPrograms),
            Box::new(recording::SystemSpawner),
            Box::new(|| Err(pd200x_core::Error::NotFound)),
        );
        hub.set_protected(true).unwrap();
        assert!(host_config.path().join(pipewire::PROTECT_FILE).exists());
        assert!(!app_config.path().join(pipewire::PROTECT_FILE).exists());
        let snap = hub.snapshot();
        assert!(snap.protected && snap.sandboxed);
    }

    #[test]
    fn protection_natively_restarts_pipewire_pulse() {
        let host_config = tempfile::tempdir().unwrap();
        let host = Host { config_home: Some(host_config.path().to_path_buf()), sandboxed: false };
        let hub = Hub::new(
            host_config.path().join("pd200x-control"),
            host,
            Box::new(NoPrograms),
            Box::new(recording::SystemSpawner),
            Box::new(|| Err(pd200x_core::Error::NotFound)),
        );
        // The file is written, then the restart is tried (and fails here).
        assert_eq!(hub.set_protected(true).unwrap_err().kind, "io");
        assert!(host_config.path().join(pipewire::PROTECT_FILE).exists());
    }

    #[test]
    fn records_analyses_and_plays_back() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = Hub::new(
            tmp.path().to_path_buf(),
            native(),
            Box::new(FakePipeWire),
            Box::new(FakeSpawner),
            Box::new(|| Ok(Device::new(Box::new(Simulator::new())))),
        );
        assert_eq!(hub.recording_status().unwrap().state, RecordingState::Idle);
        hub.start_recording().unwrap();
        assert_eq!(hub.recording_status().unwrap().state, RecordingState::Recording);
        let done = hub.stop_recording().unwrap();
        assert_eq!(done.state, RecordingState::Ready);
        let info = done.info.unwrap();
        assert_eq!(info.duration_ms, 1000);
        assert!((info.peak_db - -6.0).abs() < 0.2);

        hub.play_recording().unwrap();
        // The fake playback has ended by now; the status notices.
        assert_eq!(hub.recording_status().unwrap().state, RecordingState::Ready);
        let path = hub.rec_path.clone();
        let directory = path.parent().unwrap().to_path_buf();
        assert!(path.exists());
        drop(hub);
        assert!(!path.exists());
        assert!(!directory.exists());
    }

    #[test]
    fn playing_without_a_recording_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = Hub::new(
            tmp.path().to_path_buf(),
            native(),
            Box::new(FakePipeWire),
            Box::new(FakeSpawner),
            Box::new(|| Ok(Device::new(Box::new(Simulator::new())))),
        );
        assert_eq!(hub.play_recording().unwrap_err().kind, "invalidValue");
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
            native(),
            Box::new(pipewire::SystemRunner),
            Box::new(recording::SystemSpawner),
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
            native(),
            Box::new(pipewire::SystemRunner),
            Box::new(recording::SystemSpawner),
            Box::new(move || Ok(Device::new(Box::new(s2.clone())))),
        );
        hub.connect();
        assert_eq!(sim.get(reg::RGB_COLOR), RgbColor::Green.code());
    }
}
