//! The eight EQ presets of the MAONO Link app, from USB captures by pd200x
//! (advnpzn, `src/presets.rs`, MIT). Bands 1–5, then HPF and LPF.

use crate::model::{EqBand, FilterType, Settings, EQ_BANDS};

const fn band(enabled: bool, filter: FilterType, frequency: u16, gain_db: f32, q: f32) -> EqBand {
    EqBand { enabled, filter, frequency, gain_db, q }
}

use FilterType::{HighPass as Hp, LowPass as Lp, LowShelf as Ls, Peaking as Pk};

pub type PresetEq = [EqBand; EQ_BANDS];

pub const FLAT: PresetEq = [
    band(false, Pk, 3000, 3.0, 1.39),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 5000, 0.0, 1.0),
    band(false, Hp, 108, 0.0, 0.69),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const LOW_CUT: PresetEq = [
    band(false, Pk, 3000, 3.0, 1.39),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 5000, 0.0, 1.0),
    band(true, Hp, 108, 0.0, 0.69),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const MID_BOOST: PresetEq = [
    band(true, Pk, 3000, 3.0, 1.39),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 5000, 0.0, 1.0),
    band(false, Hp, 108, 0.0, 0.69),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const LOW_CUT_MID_BOOST: PresetEq = [
    band(true, Pk, 3000, 3.0, 1.39),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 1000, 0.0, 0.69),
    band(false, Pk, 5000, 0.0, 1.0),
    band(true, Hp, 108, 0.0, 0.69),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const BRIGHT: PresetEq = [
    band(true, Ls, 200, -5.0, 0.8),
    band(true, Pk, 366, -2.0, 0.5),
    band(true, Pk, 1000, 0.0, 1.0),
    band(true, Pk, 5000, 0.0, 1.0),
    band(true, Pk, 10000, 3.0, 0.5),
    band(true, Hp, 110, 0.0, 1.0),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const NATURAL: PresetEq = [
    band(true, Pk, 120, -3.0, 1.5),
    band(true, Pk, 500, 0.0, 1.0),
    band(true, Pk, 1320, 2.5, 1.0),
    band(true, Pk, 4310, 2.0, 1.5),
    band(true, Pk, 5000, 0.0, 1.0),
    band(true, Hp, 80, 0.0, 1.0),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const CLASSIC: PresetEq = [
    band(true, Pk, 400, -8.0, 3.0),
    band(true, Pk, 250, -3.0, 1.5),
    band(true, Pk, 1300, 2.5, 0.69),
    band(true, Pk, 10000, 0.0, 1.0),
    band(true, Pk, 5000, 0.0, 1.0),
    band(true, Hp, 50, 0.0, 1.0),
    band(false, Lp, 22000, 0.0, 1.0),
];

pub const DEEP: PresetEq = [
    band(true, Pk, 110, 3.0, 0.5),
    band(true, Pk, 500, 0.0, 1.0),
    band(true, Pk, 1000, 0.0, 1.0),
    band(true, Pk, 2000, 0.0, 1.0),
    band(true, Pk, 5000, 0.0, 1.0),
    band(false, Hp, 10, 0.0, 1.0),
    band(true, Lp, 7000, 0.0, 1.0),
];

/// Every preset with its id; the app shows translated names for the ids.
pub const ALL: [(&str, &PresetEq); 8] = [
    ("flat", &FLAT),
    ("low_cut", &LOW_CUT),
    ("mid_boost", &MID_BOOST),
    ("low_cut_mid_boost", &LOW_CUT_MID_BOOST),
    ("bright", &BRIGHT),
    ("natural", &NATURAL),
    ("classic", &CLASSIC),
    ("deep", &DEEP),
];

pub fn find(id: &str) -> Option<&'static PresetEq> {
    ALL.iter().find(|(i, _)| *i == id).map(|(_, eq)| *eq)
}

/// Two EQs sound the same: disabled bands do not count, values match within rounding.
pub fn same_eq(a: &[EqBand], b: &[EqBand]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.enabled == y.enabled
                && (!x.enabled
                    || (x.filter == y.filter
                        && x.frequency == y.frequency
                        && (x.gain_db - y.gain_db).abs() < 0.05
                        && (x.q - y.q).abs() < 0.011))
        })
}

/// Which preset or own profile an EQ read from the device is, if any. When
/// two sound the same, the one chosen last (`settings.tone.source`) wins.
pub fn detect(eq: &[EqBand], settings: &Settings) -> Option<String> {
    let saved = settings.tone.as_ref().and_then(|t| t.source.clone());
    let own = settings.profiles.iter().map(|p| (p.id.clone(), p.eq.clone()));
    let builtin = ALL.iter().map(|(id, eq)| ((*id).to_string(), eq.to_vec()));
    let mut candidates: Vec<(String, Vec<EqBand>)> = own.chain(builtin).collect();
    candidates.sort_by_key(|(id, _)| Some(id) != saved.as_ref());
    candidates.into_iter().find(|(_, c)| same_eq(eq, c)).map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HPF_INDEX, LPF_INDEX};

    #[test]
    fn every_preset_keeps_band_order() {
        for (id, eq) in ALL {
            assert_eq!(eq[HPF_INDEX].filter, FilterType::HighPass, "{id}");
            assert_eq!(eq[LPF_INDEX].filter, FilterType::LowPass, "{id}");
        }
    }

    #[test]
    fn detects_presets_and_profiles() {
        use crate::model::Profile;
        let s = Settings::default();
        assert_eq!(detect(&NATURAL, &s).as_deref(), Some("natural"));
        // Factory state: every band off at other frequencies – sounds like "flat"
        let mut factory = FLAT.to_vec();
        factory[0].frequency = 63;
        assert_eq!(detect(&factory, &s).as_deref(), Some("flat"));
        let mut custom = NATURAL.to_vec();
        custom[2].gain_db = 5.0;
        assert_eq!(detect(&custom, &s), None);
        let own = Settings {
            profiles: vec![Profile { id: "talk".into(), name: "Talk".into(), eq: custom.clone() }],
            ..Settings::default()
        };
        assert_eq!(detect(&custom, &own).as_deref(), Some("talk"));
    }

    #[test]
    fn find_by_id() {
        assert_eq!(find("natural"), Some(&NATURAL));
        assert_eq!(find("nope"), None);
    }
}
