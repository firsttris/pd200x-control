//! A simulated microphone behind the [`Transport`] trait: it keeps registers,
//! answers queries and sends level reports like the real device. Used by the
//! tests and by `PD200X_SIMULATOR=1` (CLI and app without hardware).

use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::protocol::{self, reg, Packet};
use crate::transport::Transport;

/// The device reports its level about every 55 ms.
const LEVEL_EVERY: Duration = Duration::from_millis(55);

struct State {
    registers: HashMap<u16, u16>,
    outbox: VecDeque<Vec<u8>>,
    level_db: f32,
    next_level: Instant,
    plugged: bool,
    answering: bool,
    sent: Vec<Packet>,
}

#[derive(Clone)]
pub struct Simulator {
    state: Arc<Mutex<State>>,
}

impl Default for Simulator {
    fn default() -> Self {
        Simulator::new()
    }
}

impl Simulator {
    /// All registers zero, silence.
    pub fn new() -> Simulator {
        Simulator {
            state: Arc::new(Mutex::new(State {
                registers: HashMap::new(),
                outbox: VecDeque::new(),
                level_db: -60.0,
                next_level: Instant::now(),
                plugged: true,
                answering: true,
                sent: Vec::new(),
            })),
        }
    }

    /// Registers as a microphone fresh out of the box, someone speaking softly.
    pub fn with_defaults() -> Simulator {
        let sim = Simulator::new();
        for (r, v) in [
            (reg::GAIN, 70),
            (reg::MONITOR, 1),
            (reg::HEADPHONE_VOLUME, 50),
            (reg::NC_STRENGTH, protocol::encode_nc_strength(20)),
            (reg::NOISE_GATE, 3000),
        ] {
            sim.set(r, v);
        }
        sim.speak(-24.0);
        sim
    }

    pub fn set(&self, register: u16, value: u16) {
        self.state.lock().unwrap().registers.insert(register, value);
    }

    pub fn get(&self, register: u16) -> u16 {
        self.state.lock().unwrap().registers.get(&register).copied().unwrap_or(0)
    }

    /// The level the device reports from now on.
    pub fn speak(&self, db: f32) {
        self.state.lock().unwrap().level_db = db;
    }

    pub fn unplug(&self) {
        self.state.lock().unwrap().plugged = false;
    }

    /// `false`: queries stay unanswered (a hung device).
    pub fn set_answering(&self, on: bool) {
        self.state.lock().unwrap().answering = on;
    }

    /// Every packet the host sent, oldest first.
    pub fn sent(&self) -> Vec<Packet> {
        self.state.lock().unwrap().sent.clone()
    }
}

fn report(register: u16, value: u16) -> Vec<u8> {
    protocol::command(register, value)[..64].to_vec()
}

fn u16_at(p: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([p[at], p[at + 1]])
}

impl Transport for Simulator {
    fn send(&self, packet: &Packet) -> io::Result<()> {
        let mut s = self.state.lock().unwrap();
        if !s.plugged {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "unplugged"));
        }
        s.sent.push(*packet);
        match (packet[2], packet[5]) {
            (0x09, 0x04) => {
                let register = u16_at(packet, 6);
                if s.answering {
                    let value = s.registers.get(&register).copied().unwrap_or(0);
                    s.outbox.push_back(report(register, value));
                }
            }
            (0x0B, 0x03) => {
                let (register, value) = (u16_at(packet, 6), u16_at(packet, 8));
                s.registers.insert(register, value);
            }
            (0x1B, 0x03) => {
                for i in 0..5 {
                    let at = 6 + i * 4;
                    let (register, value) = (u16_at(packet, at), u16_at(packet, at + 2));
                    s.registers.insert(register, value);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn recv(&self, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let mut s = self.state.lock().unwrap();
                if !s.plugged {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "unplugged"));
                }
                if let Some(p) = s.outbox.pop_front() {
                    return Ok(Some(p));
                }
                let now = Instant::now();
                if now >= s.next_level {
                    s.next_level = now + LEVEL_EVERY;
                    return Ok(Some(report(reg::LEVEL, protocol::encode_db(s.level_db))));
                }
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}
