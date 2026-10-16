//! The live level the microphone reports (register 0x2034, about 18 times a
//! second): current value, a peak that holds for a moment, and recording of
//! a stretch of values for the calibration.

use serde::Serialize;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Below this the meter shows silence.
pub const FLOOR_DB: f32 = -60.0;
/// How long the peak marker stays.
const PEAK_HOLD: Duration = Duration::from_millis(1500);
/// A level older than this counts as no level (device gone or busy).
const STALE: Duration = Duration::from_millis(600);

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Level {
    /// dBFS, [`FLOOR_DB`] … 0.
    pub current_db: f32,
    pub peak_db: f32,
    /// A value at or above −1 dBFS within the peak hold time.
    pub clipping: bool,
}

impl Level {
    pub const SILENT: Level = Level { current_db: FLOOR_DB, peak_db: FLOOR_DB, clipping: false };
}

#[derive(Debug, Default)]
pub struct LevelMeter {
    recent: VecDeque<(Instant, f32)>,
    capture: Option<Vec<f32>>,
}

impl LevelMeter {
    pub fn push(&mut self, db: f32, now: Instant) {
        let db = db.clamp(FLOOR_DB, 0.0);
        self.recent.push_back((now, db));
        while self.recent.front().is_some_and(|(t, _)| now.duration_since(*t) > PEAK_HOLD) {
            self.recent.pop_front();
        }
        if let Some(c) = self.capture.as_mut() {
            c.push(db);
        }
    }

    pub fn snapshot(&self, now: Instant) -> Level {
        let Some(&(last_at, current)) = self.recent.back() else { return Level::SILENT };
        if now.duration_since(last_at) > STALE {
            return Level::SILENT;
        }
        let held = self.recent.iter().filter(|(t, _)| now.duration_since(*t) <= PEAK_HOLD).map(|(_, db)| *db);
        let peak = held.clone().fold(FLOOR_DB, f32::max);
        Level { current_db: current, peak_db: peak, clipping: held.into_iter().any(|db| db >= -1.0) }
    }

    /// Starts recording every value until [`LevelMeter::take_capture`].
    pub fn start_capture(&mut self) {
        self.capture = Some(Vec::new());
    }

    pub fn take_capture(&mut self) -> Vec<f32> {
        self.capture.take().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_holds_then_falls() {
        let t0 = Instant::now();
        let mut m = LevelMeter::default();
        m.push(-10.0, t0);
        m.push(-30.0, t0 + Duration::from_millis(100));
        let l = m.snapshot(t0 + Duration::from_millis(150));
        assert_eq!((l.current_db, l.peak_db, l.clipping), (-30.0, -10.0, false));

        // −10 dB is older than the hold time now, −30 dB is not
        m.push(-35.0, t0 + Duration::from_millis(1550));
        let l = m.snapshot(t0 + Duration::from_millis(1560));
        assert_eq!(l.peak_db, -30.0);
    }

    #[test]
    fn clipping_and_clamping() {
        let t0 = Instant::now();
        let mut m = LevelMeter::default();
        m.push(3.0, t0);
        m.push(-90.0, t0 + Duration::from_millis(50));
        let l = m.snapshot(t0 + Duration::from_millis(60));
        assert_eq!((l.current_db, l.peak_db, l.clipping), (FLOOR_DB, 0.0, true));
    }

    #[test]
    fn silent_when_nothing_arrives() {
        let t0 = Instant::now();
        let mut m = LevelMeter::default();
        assert_eq!(m.snapshot(t0), Level::SILENT);
        m.push(-12.0, t0);
        assert_eq!(m.snapshot(t0 + Duration::from_secs(1)), Level::SILENT);
    }

    #[test]
    fn capture_records_only_while_on() {
        let t0 = Instant::now();
        let mut m = LevelMeter::default();
        m.push(-20.0, t0);
        m.start_capture();
        m.push(-12.0, t0);
        m.push(-15.0, t0);
        assert_eq!(m.take_capture(), vec![-12.0, -15.0]);
        m.push(-11.0, t0);
        assert!(m.take_capture().is_empty());
    }
}
