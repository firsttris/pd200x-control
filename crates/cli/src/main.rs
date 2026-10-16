//! `pd200x`: the Maono PD200X/PD200XS from the command line.

mod commands;
mod ui;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "pd200x", version, about = "Control the Maono PD200X/PD200XS microphone on Linux")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    pub fn is_on(self) -> bool {
        matches!(self, OnOff::On)
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Toggle {
    On,
    Off,
    Toggle,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Show what the microphone reports, the software boost and the saved tone and lighting
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Show the live level for a few seconds
    Level {
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
    /// Hardware gain, 0–100 (about 0 … +42 dB)
    Gain { value: u8 },
    /// Mute the microphone
    Mute { state: Toggle },
    /// Hear yourself on the headphone output
    Monitor { state: OnOff },
    /// Headphone volume, 0–100
    Headphone { value: u8 },
    /// Noise cancellation on or off, or its strength 0–40
    Nc { value: String },
    /// Apply an EQ preset or a saved profile; `list` shows them
    Eq { name: String },
    /// Compressor threshold in dB (−40 … 0) or `off`
    Compressor {
        #[arg(allow_negative_numbers = true)]
        value: String,
    },
    /// Limiter threshold in dB (−80 … 0) or `off`
    Limiter {
        #[arg(allow_negative_numbers = true)]
        value: String,
    },
    /// Ring light: on, off, a colour, `brightness <0–100>` or `linkage on|off`
    Light { args: Vec<String> },
    /// Software boost in PipeWire, 0–18 dB, or `off`; without a value shows it
    Boost { value: Option<String> },
    /// Keep apps (Teams, Slack, browsers) from changing the microphone volume; without a value shows it
    Protect { state: Option<OnOff> },
    /// Speak for a few seconds; proposes gain and boost for peaks near −9 dB
    Calibrate {
        #[arg(long, default_value_t = 5)]
        seconds: u64,
        /// Apply the proposal right away
        #[arg(long)]
        apply: bool,
    },
    /// Send the saved tone and lighting to the microphone again
    Restore,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = commands::run(cli.command) {
        eprintln!("pd200x: {e}");
        std::process::exit(1);
    }
}
