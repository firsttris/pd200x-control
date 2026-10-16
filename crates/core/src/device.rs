//! An open microphone: a reader thread takes every packet from the device,
//! feeds level reports into the [`LevelMeter`] and hands answers to waiting
//! queries; writes go straight to the transport.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::changes::{self, Change, Step};
use crate::level::{Level, LevelMeter};
use crate::model::LiveState;
use crate::protocol::{self, reg};
use crate::transport::Transport;
use crate::{Error, Result};

const QUERY_TIMEOUT: Duration = Duration::from_millis(1000);
const POLL: Duration = Duration::from_millis(100);

#[derive(Default)]
struct Answers {
    values: HashMap<u16, u16>,
}

struct Shared {
    transport: Box<dyn Transport>,
    answers: Mutex<Answers>,
    answered: Condvar,
    meter: Mutex<LevelMeter>,
    alive: AtomicBool,
    stop: AtomicBool,
    /// Keeps the packets of one change together (the monitor needs two writes).
    write: Mutex<()>,
}

pub struct Device {
    shared: Arc<Shared>,
    reader: Option<JoinHandle<()>>,
}

impl Device {
    pub fn new(transport: Box<dyn Transport>) -> Device {
        let shared = Arc::new(Shared {
            transport,
            answers: Mutex::new(Answers::default()),
            answered: Condvar::new(),
            meter: Mutex::new(LevelMeter::default()),
            alive: AtomicBool::new(true),
            stop: AtomicBool::new(false),
            write: Mutex::new(()),
        });
        let reader = {
            let shared = shared.clone();
            thread::Builder::new().name("pd200x-reader".into()).spawn(move || read_loop(&shared)).ok()
        };
        Device { shared, reader }
    }

    /// Opens the microphone on this system.
    pub fn open_system() -> Result<Device> {
        Ok(Device::new(Box::new(crate::transport::Hidraw::open_system()?)))
    }

    /// `false` once the device was unplugged; the handle is useless then.
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::SeqCst)
    }

    fn ensure_alive(&self) -> Result<()> {
        if self.is_alive() {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }

    pub fn query(&self, register: u16) -> Result<u16> {
        self.ensure_alive()?;
        let mut answers = self.shared.answers.lock().unwrap();
        answers.values.remove(&register);
        self.shared.transport.send(&protocol::query(register)).map_err(|_| Error::Disconnected)?;
        let deadline = Instant::now() + QUERY_TIMEOUT;
        loop {
            if let Some(v) = answers.values.remove(&register) {
                return Ok(v);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(Error::Timeout(register));
            }
            self.ensure_alive()?;
            answers = self.shared.answered.wait_timeout(answers, (deadline - now).min(POLL)).unwrap().0;
        }
    }

    pub fn apply(&self, change: &Change) -> Result<()> {
        changes::validate(change)?;
        self.ensure_alive()?;
        let _guard = self.shared.write.lock().unwrap();
        for step in changes::steps(change) {
            match step {
                Step::Send(p) => self.shared.transport.send(&p).map_err(|_| Error::Disconnected)?,
                Step::Pause(d) => thread::sleep(d),
            }
        }
        Ok(())
    }

    pub fn apply_all(&self, changes: &[Change]) -> Result<()> {
        changes.iter().try_for_each(|c| self.apply(c))
    }

    /// Everything the device can be asked for.
    pub fn read_live(&self) -> Result<LiveState> {
        Ok(LiveState {
            mute: self.query(reg::MUTE)? != 0,
            gain: self.query(reg::GAIN)?.min(100) as u8,
            monitor: self.query(reg::MONITOR)? == 0,
            headphone: self.query(reg::HEADPHONE_VOLUME)?.min(100) as u8,
            noise_cancellation: self.query(reg::NC_ENABLE)? != 0,
            nc_strength: protocol::decode_nc_strength(self.query(reg::NC_STRENGTH)?),
        })
    }

    pub fn level(&self) -> Level {
        if !self.is_alive() {
            return Level::SILENT;
        }
        self.shared.meter.lock().unwrap().snapshot(Instant::now())
    }

    /// Records the level for `duration` and returns every value (dBFS).
    pub fn capture(&self, duration: Duration) -> Result<Vec<f32>> {
        self.ensure_alive()?;
        self.shared.meter.lock().unwrap().start_capture();
        thread::sleep(duration);
        let values = self.shared.meter.lock().unwrap().take_capture();
        self.ensure_alive()?;
        Ok(values)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
    }
}

fn read_loop(shared: &Shared) {
    while !shared.stop.load(Ordering::SeqCst) {
        match shared.transport.recv(POLL) {
            Ok(Some(raw)) => {
                let Some(report) = protocol::parse(&raw) else { continue };
                if report.register == reg::LEVEL {
                    shared.meter.lock().unwrap().push(protocol::decode_db(report.value), Instant::now());
                } else {
                    shared.answers.lock().unwrap().values.insert(report.register, report.value);
                    shared.answered.notify_all();
                }
            }
            Ok(None) => {}
            Err(_) => {
                shared.alive.store(false, Ordering::SeqCst);
                shared.answered.notify_all();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RgbColor;
    use crate::sim::Simulator;

    fn open() -> (Device, Simulator) {
        let sim = Simulator::new();
        (Device::new(Box::new(sim.clone())), sim)
    }

    #[test]
    fn reads_the_live_state() {
        let (dev, sim) = open();
        sim.set(reg::GAIN, 100);
        sim.set(reg::MONITOR, 0);
        sim.set(reg::NC_STRENGTH, 1410);
        let s = dev.read_live().unwrap();
        assert_eq!((s.gain, s.monitor, s.nc_strength, s.mute), (100, true, 21, false));
    }

    #[test]
    fn applies_changes_to_registers() {
        let (dev, sim) = open();
        dev.apply(&Change::Gain(42)).unwrap();
        dev.apply(&Change::RgbColor(RgbColor::Cyan)).unwrap();
        assert_eq!(sim.get(reg::GAIN), 42);
        assert_eq!(sim.get(reg::RGB_COLOR), 5);
        assert!(dev.apply(&Change::Gain(150)).is_err());
    }

    #[test]
    fn level_reports_reach_the_meter() {
        let (dev, sim) = open();
        sim.speak(-12.0);
        thread::sleep(Duration::from_millis(300));
        let l = dev.level();
        assert!((l.peak_db - -12.0).abs() < 0.11, "{l:?}");
    }

    #[test]
    fn capture_collects_values() {
        let (dev, sim) = open();
        sim.speak(-20.0);
        let values = dev.capture(Duration::from_millis(250)).unwrap();
        assert!(!values.is_empty());
        assert!(values.iter().all(|v| (*v - -20.0).abs() < 0.11));
    }

    #[test]
    fn unplugging_ends_the_handle() {
        let (dev, sim) = open();
        sim.unplug();
        thread::sleep(Duration::from_millis(250));
        assert!(!dev.is_alive());
        assert!(matches!(dev.query(reg::GAIN), Err(Error::Disconnected)));
        assert!(matches!(dev.apply(&Change::Mute(true)), Err(Error::Disconnected)));
        assert_eq!(dev.level(), Level::SILENT);
    }

    #[test]
    fn a_silent_device_times_out() {
        let (dev, sim) = open();
        sim.set_answering(false);
        assert!(matches!(dev.query(reg::GAIN), Err(Error::Timeout(r)) if r == reg::GAIN));
    }
}
