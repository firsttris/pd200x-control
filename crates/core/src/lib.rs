//! Core of pd200x-control: the USB HID protocol of the Maono PD200X and
//! PD200XS, the live level, the PipeWire software boost, calibration and the
//! settings the app remembers. The CLI and the desktop app are thin layers
//! over this crate.

pub mod calibrate;
pub mod changes;
pub mod device;
mod error;
pub mod level;
pub mod model;
pub mod pipewire;
pub mod presets;
pub mod protocol;
pub mod sim;
pub mod store;
pub mod transport;

pub use changes::Change;
pub use device::Device;
pub use error::{Error, Result};
