//! What can be changed on the microphone, and the packets each change needs.
//! Pure: [`steps`] turns a change into packets and pauses, the device sends them.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::model::{
    Dynamics, EqBand, Lighting, RgbColor, Tone, COMPRESSOR_MIN_DB, EQ_BANDS, HPF_INDEX, LIMITER_MIN_DB, LPF_INDEX,
};
use crate::protocol::{self, reg, Packet};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum Change {
    Mute(bool),
    Gain(u8),
    Monitor(bool),
    Headphone(u8),
    NoiseCancellation(bool),
    NcStrength(u8),
    EqBand { index: u8, band: EqBand },
    Compressor(Dynamics),
    Limiter(Dynamics),
    RgbEnabled(bool),
    RgbBrightness(u8),
    RgbColor(RgbColor),
    Linkage(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Send(Packet),
    Pause(Duration),
}

fn set(register: u16, value: u16) -> Step {
    Step::Send(protocol::command(register, value))
}

fn flag(on: bool) -> u16 {
    u16::from(on)
}

/// Rejects values outside the device's ranges instead of sending them.
pub fn validate(change: &Change) -> Result<()> {
    let bad = |what: &str| Err(Error::InvalidValue(what.into()));
    match change {
        Change::Gain(v) | Change::Headphone(v) | Change::RgbBrightness(v) if *v > 100 => bad("0–100"),
        Change::NcStrength(v) if *v > 40 => bad("noise cancellation strength 0–40"),
        Change::EqBand { index, .. } if usize::from(*index) >= EQ_BANDS => bad("EQ band 0–6"),
        _ => Ok(()),
    }
}

/// The packets for one change. Values are clamped to the firmware's ranges.
pub fn steps(change: &Change) -> Vec<Step> {
    match *change {
        Change::Mute(on) => vec![set(reg::MUTE, flag(on))],
        Change::Gain(v) => vec![set(reg::GAIN, u16::from(v.min(100)))],
        // MAONO Link writes both registers with a pause in between (pd200x `set_mic_monitor`).
        Change::Monitor(on) => vec![
            set(reg::NOISE_GATE, if on { 2000 } else { 3000 }),
            Step::Pause(Duration::from_millis(100)),
            set(reg::MONITOR, flag(!on)),
            Step::Pause(Duration::from_millis(100)),
        ],
        Change::Headphone(v) => vec![set(reg::HEADPHONE_VOLUME, u16::from(v.min(100)))],
        Change::NoiseCancellation(on) => vec![set(reg::NC_ENABLE, flag(on))],
        Change::NcStrength(v) => vec![set(reg::NC_STRENGTH, protocol::encode_nc_strength(v))],
        Change::EqBand { index, band } => vec![Step::Send(eq_packet(usize::from(index), band))],
        Change::Compressor(d) => {
            dynamics(reg::COMPRESSOR_ENABLE, reg::COMPRESSOR_THRESHOLD, d.clamped(COMPRESSOR_MIN_DB))
        }
        Change::Limiter(d) => dynamics(reg::LIMITER_ENABLE, reg::LIMITER_THRESHOLD, d.clamped(LIMITER_MIN_DB)),
        Change::RgbEnabled(on) => vec![set(reg::RGB_ENABLE, flag(on))],
        Change::RgbBrightness(v) => vec![set(reg::RGB_BRIGHTNESS, u16::from(v.min(100)))],
        Change::RgbColor(c) => vec![set(reg::RGB_COLOR, c.code())],
        Change::Linkage(on) => vec![set(reg::LIGHTING_LINKAGE, flag(on))],
    }
}

fn dynamics(enable: u16, threshold: u16, d: Dynamics) -> Vec<Step> {
    vec![set(enable, flag(d.enabled)), set(threshold, protocol::encode_db(d.threshold_db))]
}

fn eq_packet(index: usize, band: EqBand) -> Packet {
    let band = band.clamped();
    let base = protocol::reg::EQ_BASE + (index as u16) * 5;
    // HPF and LPF take fixed gain and Q values (pd200x `eq_pairs`).
    let (gain, q) = if index == HPF_INDEX || index == LPF_INDEX {
        (2000, 100)
    } else {
        (protocol::encode_db(band.gain_db), protocol::encode_q(band.q))
    };
    protocol::command5([
        (base, flag(band.enabled)),
        (base + 1, band.filter.code()),
        (base + 2, band.frequency),
        (base + 3, gain),
        (base + 4, q),
    ])
}

/// All changes that bring the tone onto the device.
pub fn tone_changes(tone: &Tone) -> Vec<Change> {
    let mut out: Vec<Change> =
        tone.eq.iter().take(EQ_BANDS).enumerate().map(|(i, &band)| Change::EqBand { index: i as u8, band }).collect();
    out.push(Change::Compressor(tone.compressor));
    out.push(Change::Limiter(tone.limiter));
    out
}

/// All changes that bring the lighting onto the device.
pub fn lighting_changes(l: &Lighting) -> Vec<Change> {
    vec![
        Change::RgbEnabled(l.enabled),
        Change::RgbColor(l.color),
        Change::RgbBrightness(l.brightness),
        Change::Linkage(l.linkage),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FilterType;
    use crate::presets;

    fn sent(steps: &[Step]) -> Vec<(u16, u16)> {
        steps
            .iter()
            .filter_map(|s| match s {
                Step::Send(p) => Some((u16::from_le_bytes([p[6], p[7]]), u16::from_le_bytes([p[8], p[9]]))),
                Step::Pause(_) => None,
            })
            .collect()
    }

    #[test]
    fn simple_registers() {
        assert_eq!(sent(&steps(&Change::Gain(80))), [(reg::GAIN, 80)]);
        assert_eq!(sent(&steps(&Change::Gain(250))), [(reg::GAIN, 100)]);
        assert_eq!(sent(&steps(&Change::Mute(true))), [(reg::MUTE, 1)]);
        assert_eq!(sent(&steps(&Change::NcStrength(21))), [(reg::NC_STRENGTH, 1410)]);
        assert_eq!(sent(&steps(&Change::RgbColor(RgbColor::Blue))), [(reg::RGB_COLOR, 6)]);
    }

    #[test]
    fn monitor_writes_both_registers_with_pauses() {
        let on = steps(&Change::Monitor(true));
        assert_eq!(sent(&on), [(reg::NOISE_GATE, 2000), (reg::MONITOR, 0)]);
        assert_eq!(on.iter().filter(|s| matches!(s, Step::Pause(_))).count(), 2);
        assert_eq!(sent(&steps(&Change::Monitor(false))), [(reg::NOISE_GATE, 3000), (reg::MONITOR, 1)]);
    }

    #[test]
    fn dynamics_encode_the_threshold_in_db() {
        let s = steps(&Change::Compressor(Dynamics { enabled: true, threshold_db: -9.0 }));
        assert_eq!(sent(&s), [(reg::COMPRESSOR_ENABLE, 1), (reg::COMPRESSOR_THRESHOLD, 1910)]);
        let s = steps(&Change::Compressor(Dynamics { enabled: true, threshold_db: -70.0 }));
        assert_eq!(sent(&s)[1], (reg::COMPRESSOR_THRESHOLD, 1600));
        let s = steps(&Change::Limiter(Dynamics { enabled: false, threshold_db: -70.0 }));
        assert_eq!(sent(&s), [(reg::LIMITER_ENABLE, 0), (reg::LIMITER_THRESHOLD, 1300)]);
    }

    #[test]
    fn eq_band_packet_carries_five_registers() {
        let band = EqBand { enabled: true, filter: FilterType::Peaking, frequency: 1320, gain_db: 2.5, q: 1.0 };
        let Step::Send(p) = &steps(&Change::EqBand { index: 2, band })[0] else { panic!() };
        let pairs: Vec<(u16, u16)> = (0..5)
            .map(|i| {
                let at = 6 + i * 4;
                (u16::from_le_bytes([p[at], p[at + 1]]), u16::from_le_bytes([p[at + 2], p[at + 3]]))
            })
            .collect();
        assert_eq!(pairs, [(0x2057, 1), (0x2058, 0), (0x2059, 1320), (0x205A, 2025), (0x205B, 100)]);
    }

    #[test]
    fn hpf_sends_firmware_constants_for_gain_and_q() {
        let Step::Send(p) = &steps(&Change::EqBand { index: 5, band: presets::NATURAL[5] })[0] else { panic!() };
        assert_eq!(u16::from_le_bytes([p[6], p[7]]), 0x2066);
        assert_eq!(u16::from_le_bytes([p[20], p[21]]), 2000);
        assert_eq!(u16::from_le_bytes([p[24], p[25]]), 100);
    }

    #[test]
    fn validate_rejects_out_of_range() {
        assert!(validate(&Change::Gain(101)).is_err());
        assert!(validate(&Change::NcStrength(41)).is_err());
        assert!(validate(&Change::EqBand { index: 7, band: presets::FLAT[0] }).is_err());
        assert!(validate(&Change::Gain(100)).is_ok());
    }

    #[test]
    fn tone_and_lighting_expand_to_all_registers() {
        assert_eq!(tone_changes(&Tone::default()).len(), 9);
        assert_eq!(lighting_changes(&Lighting::default()).len(), 4);
    }

    #[test]
    fn change_serializes_tagged() {
        let json = serde_json::to_string(&Change::Gain(80)).unwrap();
        assert_eq!(json, r#"{"kind":"gain","value":80}"#);
        let back: Change = serde_json::from_str(r#"{"kind":"rgbColor","value":"cyan"}"#).unwrap();
        assert_eq!(back, Change::RgbColor(RgbColor::Cyan));
    }
}
