//! "Pegel einmessen": from a few seconds of speech, the gain and software
//! boost that put the voice's peaks near the target. Pure.

use serde::Serialize;

use crate::model::round1;
use crate::{Error, Result};

/// Where the peaks of normal speech should land.
pub const TARGET_PEAK_DB: f32 = -9.0;
/// The hardware preamp spans about 42 dB over its 100 steps (MAONO: 0 … +42 dB).
pub const DB_PER_GAIN_STEP: f32 = 0.42;
pub const MAX_BOOST_DB: f32 = 18.0;
/// Peaks below this mean nobody spoke.
const VOICE_FLOOR_DB: f32 = -50.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    /// The speech peak during the measurement (device level, before the boost).
    pub measured_peak_db: f32,
    pub gain: u8,
    pub boost_db: f32,
    /// Where the peaks land with the proposal (after the boost).
    pub expected_peak_db: f32,
}

/// A robust peak: the 98th percentile, so a single bump or click does not count.
pub fn speech_peak(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    let idx = ((sorted.len() - 1) as f32 * 0.98).round() as usize;
    Some(sorted[idx])
}

/// `values` were measured at `gain`; `boost_db` was active in PipeWire but is
/// not part of the device level. Raises the hardware gain first (better
/// signal-to-noise), the rest goes to the software boost; lowering takes
/// the boost first.
pub fn propose(values: &[f32], gain: u8, boost_db: f32) -> Result<Proposal> {
    let peak = speech_peak(values).filter(|p| *p > VOICE_FLOOR_DB).ok_or(Error::NoVoice)?;
    let gain = gain.min(100);
    let current_total = peak + boost_db;
    let mut needed = TARGET_PEAK_DB - current_total;

    let mut new_boost = boost_db;
    let mut new_gain = f32::from(gain);
    if needed > 0.0 {
        let gain_room = (100.0 - new_gain) * DB_PER_GAIN_STEP;
        let from_gain = needed.min(gain_room);
        new_gain += from_gain / DB_PER_GAIN_STEP;
        needed -= from_gain;
        new_boost += needed;
    } else {
        let from_boost = (-needed).min(new_boost);
        new_boost -= from_boost;
        needed += from_boost;
        new_gain += needed / DB_PER_GAIN_STEP;
    }
    let new_gain = new_gain.round().clamp(0.0, 100.0) as u8;
    let new_boost = round1(new_boost.clamp(0.0, MAX_BOOST_DB));
    let gain_change = (f32::from(new_gain) - f32::from(gain)) * DB_PER_GAIN_STEP;
    Ok(Proposal {
        measured_peak_db: round1(peak),
        gain: new_gain,
        boost_db: new_boost,
        expected_peak_db: round1(peak + gain_change + new_boost),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speech(peak: f32) -> Vec<f32> {
        let mut v: Vec<f32> = (0..90).map(|i| peak - 20.0 + (i % 10) as f32).collect();
        v.extend([peak; 5]);
        v
    }

    #[test]
    fn todays_case_gain_at_max_goes_to_boost() {
        // Gain 100, peaks at −21.5 dB, no boost: about +12.5 dB boost
        let p = propose(&speech(-21.5), 100, 0.0).unwrap();
        assert_eq!((p.gain, p.boost_db), (100, 12.5));
        assert_eq!(p.expected_peak_db, TARGET_PEAK_DB);
    }

    #[test]
    fn raises_the_gain_before_the_boost() {
        let p = propose(&speech(-25.0), 80, 0.0).unwrap();
        assert_eq!(p.gain, 100);
        assert_eq!(p.boost_db, 7.6);
        let p = propose(&speech(-12.0), 70, 0.0).unwrap();
        assert_eq!((p.gain, p.boost_db), (77, 0.0));
    }

    #[test]
    fn too_loud_lowers_the_boost_first_then_the_gain() {
        let p = propose(&speech(-12.0), 100, 10.0).unwrap();
        assert_eq!((p.gain, p.boost_db), (100, 3.0));
        let p = propose(&speech(-2.0), 100, 2.0).unwrap();
        assert_eq!(p.boost_db, 0.0);
        assert_eq!(p.gain, 83);
    }

    #[test]
    fn the_boost_has_a_ceiling() {
        let p = propose(&speech(-45.0), 100, 0.0).unwrap();
        assert_eq!(p.boost_db, MAX_BOOST_DB);
        assert!(p.expected_peak_db < TARGET_PEAK_DB);
    }

    #[test]
    fn silence_is_an_error() {
        assert!(matches!(propose(&[-60.0; 50], 100, 0.0), Err(Error::NoVoice)));
        assert!(matches!(propose(&[], 100, 0.0), Err(Error::NoVoice)));
    }

    #[test]
    fn a_single_click_does_not_count_as_peak() {
        let mut v = vec![-30.0; 200];
        v[100] = -2.0;
        assert_eq!(speech_peak(&v), Some(-30.0));
    }
}
