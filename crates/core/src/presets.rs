//! The eight EQ presets of the MAONO Link app, from USB captures by pd200x
//! (advnpzn, `src/presets.rs`, MIT). Bands 1–5, then HPF and LPF.

use crate::model::{EqBand, FilterType, EQ_BANDS};

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
    fn find_by_id() {
        assert_eq!(find("natural"), Some(&NATURAL));
        assert_eq!(find("nope"), None);
    }
}
