//! The data the app and the CLI work with: what the microphone reports, what
//! it only accepts (EQ, dynamics, lighting) and what is remembered between runs.

use serde::{Deserialize, Serialize};

/// Shape of an EQ band. The numbers are the firmware's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum FilterType {
    Peaking,
    LowPass,
    HighPass,
    LowShelf,
    HighShelf,
}

impl FilterType {
    pub fn code(self) -> u16 {
        match self {
            FilterType::Peaking => 0,
            FilterType::LowPass => 1,
            FilterType::HighPass => 2,
            FilterType::LowShelf => 3,
            FilterType::HighShelf => 4,
        }
    }
}

/// One of the seven EQ bands: five parametric ones, then the high-pass and
/// the low-pass filter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub enabled: bool,
    pub filter: FilterType,
    /// Hz, 20–22000.
    pub frequency: u16,
    /// dB, −12 … +12; ignored by the firmware for HPF and LPF.
    pub gain_db: f32,
    /// 0.1 – 10.
    pub q: f32,
}

pub const EQ_BANDS: usize = 7;
pub const HPF_INDEX: usize = 5;
pub const LPF_INDEX: usize = 6;

impl EqBand {
    /// Keeps every value inside what the firmware accepts.
    pub fn clamped(self) -> EqBand {
        EqBand {
            frequency: self.frequency.clamp(20, 22_000),
            gain_db: round1(self.gain_db.clamp(-12.0, 12.0)),
            q: (self.q.clamp(0.1, 10.0) * 100.0).round() / 100.0,
            ..self
        }
    }
}

/// Compressor or limiter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Dynamics {
    pub enabled: bool,
    pub threshold_db: f32,
}

/// Lower ends of the thresholds, as MAONO Link offers them.
pub const COMPRESSOR_MIN_DB: f32 = -40.0;
pub const LIMITER_MIN_DB: f32 = -80.0;

impl Dynamics {
    pub fn clamped(self, min_db: f32) -> Dynamics {
        Dynamics { threshold_db: round1(self.threshold_db.clamp(min_db, 0.0)), ..self }
    }
}

/// The colours of the ring light, in firmware order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum RgbColor {
    White,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Rainbow,
}

impl RgbColor {
    pub const ALL: [RgbColor; 9] = [
        RgbColor::White,
        RgbColor::Red,
        RgbColor::Orange,
        RgbColor::Yellow,
        RgbColor::Green,
        RgbColor::Cyan,
        RgbColor::Blue,
        RgbColor::Purple,
        RgbColor::Rainbow,
    ];

    pub fn code(self) -> u16 {
        RgbColor::ALL.iter().position(|&c| c == self).unwrap_or(0) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Lighting {
    pub enabled: bool,
    /// 0–100.
    pub brightness: u8,
    pub color: RgbColor,
    /// The ring goes dark while the microphone is muted.
    pub linkage: bool,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting { enabled: true, brightness: 60, color: RgbColor::Rainbow, linkage: false }
    }
}

/// What the microphone reports when asked; the knob and the buttons on the
/// device change these too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LiveState {
    pub mute: bool,
    /// 0–100, the hardware preamp (0 … about +42 dB).
    pub gain: u8,
    /// Hear yourself on the headphone output.
    pub monitor: bool,
    /// 0–100.
    pub headphone: u8,
    pub noise_cancellation: bool,
    /// 0–40.
    pub nc_strength: u8,
}

/// EQ, compressor and limiter: the device accepts them but cannot be asked
/// for them, so the app remembers them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Tone {
    /// Id of the preset or own profile the EQ came from; `None` once changed by hand.
    pub source: Option<String>,
    /// Always [`EQ_BANDS`] entries.
    pub eq: Vec<EqBand>,
    pub compressor: Dynamics,
    pub limiter: Dynamics,
}

impl Default for Tone {
    fn default() -> Self {
        Tone {
            source: Some("flat".into()),
            eq: crate::presets::FLAT.to_vec(),
            compressor: Dynamics { enabled: false, threshold_db: -20.0 },
            limiter: Dynamics { enabled: false, threshold_db: -3.0 },
        }
    }
}

/// An EQ the user saved under a name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub eq: Vec<EqBand>,
}

/// Everything the app keeps in `settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `None` until the user changed the tone in the app; only then is it
    /// written to the microphone when it is plugged in.
    pub tone: Option<Tone>,
    pub lighting: Option<Lighting>,
    pub profiles: Vec<Profile>,
    /// Send `tone` and `lighting` to the microphone when it is plugged in.
    pub restore_on_connect: bool,
    /// The software boost to go back to when it is switched on again.
    pub boost_db: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { tone: None, lighting: None, profiles: Vec::new(), restore_on_connect: true, boost_db: 10.0 }
    }
}

pub(crate) fn round1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_band_is_clamped_to_the_firmware_range() {
        let b = EqBand { enabled: true, filter: FilterType::Peaking, frequency: 5, gain_db: 30.04, q: 0.001 }.clamped();
        assert_eq!((b.frequency, b.gain_db, b.q), (20, 12.0, 0.1));
        let b = EqBand { enabled: true, filter: FilterType::Peaking, frequency: 60_000, gain_db: -2.04, q: 1.394 }
            .clamped();
        assert_eq!((b.frequency, b.gain_db, b.q), (22_000, -2.0, 1.39));
    }

    #[test]
    fn dynamics_threshold_stays_between_min_and_zero() {
        let d = Dynamics { enabled: true, threshold_db: 3.0 }.clamped(COMPRESSOR_MIN_DB);
        assert_eq!(d.threshold_db, 0.0);
        let d = Dynamics { enabled: true, threshold_db: -99.0 }.clamped(LIMITER_MIN_DB);
        assert_eq!(d.threshold_db, -80.0);
    }

    #[test]
    fn color_codes_follow_firmware_order() {
        assert_eq!(RgbColor::White.code(), 0);
        assert_eq!(RgbColor::Cyan.code(), 5);
        assert_eq!(RgbColor::Rainbow.code(), 8);
    }

    #[test]
    fn settings_read_with_missing_fields() {
        let s: Settings = serde_json::from_str(r#"{"boostDb": 14.6}"#).unwrap();
        assert_eq!(s.boost_db, 14.6);
        assert!(s.restore_on_connect);
        assert_eq!(s.tone, None);
    }
}
