use std::fmt;

/// Every error of the core. The app shows a translated text for
/// [`Error::kind`]; the CLI prints the English `Display`.
#[derive(Debug)]
pub enum Error {
    /// No PD200X on any USB port.
    NotFound,
    /// The device file exists but may not be opened; the udev rule is missing.
    PermissionDenied(String),
    /// The microphone was unplugged while in use.
    Disconnected,
    /// The microphone did not answer a query in time.
    Timeout(u16),
    /// PipeWire has no source for the microphone.
    NoAudioSource,
    /// A program the core runs (`wpctl`, `pw-dump`) is not installed.
    ToolMissing(String),
    ToolFailed {
        tool: String,
        message: String,
    },
    /// A value outside its range, or an unknown preset or profile.
    InvalidValue(String),
    /// The level stayed near silence during the calibration.
    NoVoice,
    Io(std::io::Error),
}

impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Error::NotFound => "notFound",
            Error::PermissionDenied(_) => "permissionDenied",
            Error::Disconnected => "disconnected",
            Error::Timeout(_) => "timeout",
            Error::NoAudioSource => "noAudioSource",
            Error::ToolMissing(_) => "toolMissing",
            Error::ToolFailed { .. } => "toolFailed",
            Error::InvalidValue(_) => "invalidValue",
            Error::NoVoice => "noVoice",
            Error::Io(_) => "io",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(f, "No Maono PD200X found. Is it plugged in over USB?"),
            Error::PermissionDenied(path) => write!(
                f,
                "No permission to open {path}. Install the udev rule (packaging/99-pd200x.rules) and plug the microphone in again."
            ),
            Error::Disconnected => write!(f, "The microphone was unplugged."),
            Error::Timeout(reg) => write!(f, "The microphone did not answer (register 0x{reg:04X})."),
            Error::NoAudioSource => write!(f, "PipeWire has no input for the PD200X."),
            Error::ToolMissing(tool) => write!(f, "{tool} is not installed."),
            Error::ToolFailed { tool, message } => write!(f, "{tool} failed: {message}"),
            Error::InvalidValue(what) => write!(f, "Invalid value: {what}"),
            Error::NoVoice => write!(f, "No voice heard. Speak normally into the microphone while measuring."),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
