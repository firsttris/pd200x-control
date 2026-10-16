//! The microphone's USB HID protocol: registers, packets and value encodings.
//!
//! Everything here is pure. The layout follows pd200x by advnpzn
//! (`src/protocol.rs`, `src/packet.rs`, MIT), which reverse-engineered it
//! from USB captures of the MAONO Link app.

/// USB vendor and product id of the PD200X and PD200XS (same firmware).
pub const VENDOR_ID: u16 = 0x352F;
pub const PRODUCT_ID: u16 = 0x0104;

/// Register numbers. A command packet writes one, a query packet asks the
/// device to answer with its value.
pub mod reg {
    pub const MUTE: u16 = 0x2022;
    pub const GAIN: u16 = 0x2023;
    pub const COMPRESSOR_ENABLE: u16 = 0x2026;
    pub const COMPRESSOR_THRESHOLD: u16 = 0x2027;
    pub const LIMITER_ENABLE: u16 = 0x202C;
    pub const LIMITER_THRESHOLD: u16 = 0x202D;
    pub const HEADPHONE_VOLUME: u16 = 0x2030;
    /// Written together with [`MONITOR`]: 2000 with monitoring on, 3000 off
    /// (pd200x `set_mic_monitor`). Its own meaning is unclear, so the app
    /// never writes it alone.
    pub const NOISE_GATE: u16 = 0x2031;
    /// Sent by the device on its own, about 18 times a second.
    pub const LEVEL: u16 = 0x2034;
    pub const RGB_ENABLE: u16 = 0x2036;
    pub const RGB_BRIGHTNESS: u16 = 0x2037;
    pub const RGB_COLOR: u16 = 0x2038;
    pub const LIGHTING_LINKAGE: u16 = 0x2039;
    pub const NC_ENABLE: u16 = 0x203A;
    pub const NC_STRENGTH: u16 = 0x203B;
    /// First register of EQ band 1; each band has five (enable, type,
    /// frequency, gain, Q), the seven bands follow each other.
    pub const EQ_BASE: u16 = 0x204D;
    /// 0 = monitoring on, 1 = off.
    pub const MONITOR: u16 = 0x2070;
}

/// The host sends 65 bytes; the first one is the HID report id.
pub type Packet = [u8; 65];
/// The device answers with 64 bytes (no report id echo).
pub type Response = [u8; 64];

/// `-sum(bytes[1..end]) & 0xFFFF`.
fn checksum(bytes: &[u8], end: usize) -> u16 {
    let sum: u32 = bytes[1..end].iter().map(|&b| u32::from(b)).sum();
    (!sum).wrapping_add(1) as u16
}

fn header(len: u8, kind: u8) -> Packet {
    let mut p = [0u8; 65];
    p[0..6].copy_from_slice(&[0x4B, 0xC4, len, 0x00, 0x00, kind]);
    p
}

/// Writes `value` to `register`. The device applies it at once, without an answer.
pub fn command(register: u16, value: u16) -> Packet {
    let mut p = header(0x0B, 0x03);
    p[6..8].copy_from_slice(&register.to_le_bytes());
    p[8..10].copy_from_slice(&value.to_le_bytes());
    let cs = checksum(&p, 10);
    p[10..12].copy_from_slice(&cs.to_le_bytes());
    p
}

/// Asks for the value of `register`; the device answers with a response packet.
pub fn query(register: u16) -> Packet {
    let mut p = header(0x09, 0x04);
    p[6..8].copy_from_slice(&register.to_le_bytes());
    let cs = checksum(&p, 8);
    p[8..10].copy_from_slice(&cs.to_le_bytes());
    p
}

/// Writes five registers at once; used for the five values of an EQ band.
pub fn command5(pairs: [(u16, u16); 5]) -> Packet {
    let mut p = header(0x1B, 0x03);
    for (i, (register, value)) in pairs.into_iter().enumerate() {
        let at = 6 + i * 4;
        p[at..at + 2].copy_from_slice(&register.to_le_bytes());
        p[at + 2..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    let cs = checksum(&p, 0x1A);
    p[0x1A..0x1C].copy_from_slice(&cs.to_le_bytes());
    p
}

/// A register value the device sent: an answer to a query or a level report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub register: u16,
    pub value: u16,
}

/// Parses a packet from the device; `None` for anything that is not a
/// well-formed register report.
pub fn parse(raw: &[u8]) -> Option<Report> {
    if raw.len() < 12 || raw[1] != 0xC4 {
        return None;
    }
    let stored = u16::from_le_bytes([raw[10], raw[11]]);
    if stored != checksum(raw, 10) {
        return None;
    }
    Some(Report { register: u16::from_le_bytes([raw[6], raw[7]]), value: u16::from_le_bytes([raw[8], raw[9]]) })
}

/// Thresholds, EQ gains and the level all use `2000 + dB × 10`.
pub fn encode_db(db: f32) -> u16 {
    (2000.0 + db * 10.0).round().clamp(0.0, f32::from(u16::MAX)) as u16
}

pub fn decode_db(raw: u16) -> f32 {
    (f32::from(raw) - 2000.0) / 10.0
}

pub fn encode_q(q: f32) -> u16 {
    (q * 100.0).round().clamp(0.0, f32::from(u16::MAX)) as u16
}

/// Noise cancellation strength: 0–40 in the UI, `(s + 120) × 10` on the wire.
pub fn encode_nc_strength(strength: u8) -> u16 {
    (u16::from(strength.min(40)) + 120) * 10
}

pub fn decode_nc_strength(raw: u16) -> u8 {
    (raw / 10).saturating_sub(120).min(40) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_of(packet: &Packet) -> Response {
        // Device reports share the command layout, 64 bytes long.
        let mut r = [0u8; 64];
        r.copy_from_slice(&packet[..64]);
        r
    }

    #[test]
    fn command_layout() {
        // Gain 50: checksum = -(0xC4 + 0x0B + 0x03 + 0x23 + 0x20 + 0x32) = 0xFEB9
        let p = command(reg::GAIN, 50);
        assert_eq!(&p[..12], &[0x4B, 0xC4, 0x0B, 0x00, 0x00, 0x03, 0x23, 0x20, 0x32, 0x00, 0xB9, 0xFE]);
        assert!(p[12..].iter().all(|&b| b == 0));
    }

    #[test]
    fn query_layout() {
        let p = query(reg::GAIN);
        assert_eq!(&p[..6], &[0x4B, 0xC4, 0x09, 0x00, 0x00, 0x04]);
        assert_eq!(u16::from_le_bytes([p[6], p[7]]), reg::GAIN);
        assert_eq!(u16::from_le_bytes([p[8], p[9]]), checksum(&p, 8));
    }

    #[test]
    fn command5_checksum_covers_all_pairs() {
        let p = command5([(0x204D, 1), (0x204E, 0), (0x204F, 110), (0x2050, 2030), (0x2051, 50)]);
        assert_eq!(p[2], 0x1B);
        assert_eq!(u16::from_le_bytes([p[0x1A], p[0x1B]]), checksum(&p, 0x1A));
        assert_eq!(u16::from_le_bytes([p[22], p[23]]), 0x2051);
    }

    #[test]
    fn parse_reads_a_report_and_rejects_garbage() {
        let raw = response_of(&command(reg::LEVEL, 1820));
        assert_eq!(parse(&raw), Some(Report { register: reg::LEVEL, value: 1820 }));

        let mut bad = raw;
        bad[10] ^= 0xFF;
        assert_eq!(parse(&bad), None);
        let mut magic = raw;
        magic[1] = 0;
        assert_eq!(parse(&magic), None);
        assert_eq!(parse(&raw[..8]), None);
    }

    #[test]
    fn db_round_trips() {
        assert_eq!(encode_db(0.0), 2000);
        assert_eq!(encode_db(-9.0), 1910);
        assert_eq!(encode_db(2.5), 2025);
        assert_eq!(decode_db(1400), -60.0);
        assert_eq!(decode_db(encode_db(-20.0)), -20.0);
        assert_eq!(encode_db(-1000.0), 0);
    }

    #[test]
    fn nc_strength_round_trips() {
        assert_eq!(encode_nc_strength(21), 1410);
        assert_eq!(decode_nc_strength(1540), 34);
        assert_eq!(encode_nc_strength(99), encode_nc_strength(40));
        for s in 0..=40 {
            assert_eq!(decode_nc_strength(encode_nc_strength(s)), s);
        }
    }

    #[test]
    fn q_encoding() {
        assert_eq!(encode_q(1.39), 139);
        assert_eq!(encode_q(0.69), 69);
    }
}
