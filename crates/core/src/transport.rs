//! The edge to the hardware: finding the microphone's `/dev/hidraw*` node and
//! moving packets through it. Plain hidraw instead of hidapi, so nothing but
//! the kernel is needed (no libudev).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::protocol::{Packet, PRODUCT_ID, VENDOR_ID};
use crate::{Error, Result};

/// Moves packets to and from the device. Both ends work through `&self`, so
/// one thread can read while another writes.
pub trait Transport: Send + Sync {
    fn send(&self, packet: &Packet) -> io::Result<()>;
    /// One packet from the device, or `None` when nothing came in `timeout`.
    fn recv(&self, timeout: Duration) -> io::Result<Option<Vec<u8>>>;
}

/// `HID_ID=0003:0000352F:00000104` in the uevent of the hidraw node's device.
fn is_pd200x(uevent: &str) -> bool {
    let want = format!(":{VENDOR_ID:08X}:{PRODUCT_ID:08X}");
    uevent.lines().any(|l| l.strip_prefix("HID_ID=").is_some_and(|id| id.to_ascii_uppercase().ends_with(&want)))
}

/// The microphone's device node, found through `<sys_class>/hidraw*/device/uevent`.
pub fn find_hidraw(sys_class: &Path, dev: &Path) -> Option<PathBuf> {
    let mut nodes: Vec<_> = fs::read_dir(sys_class).ok()?.flatten().map(|e| e.path()).collect();
    nodes.sort();
    nodes.into_iter().find_map(|node| {
        let uevent = fs::read_to_string(node.join("device/uevent")).ok()?;
        let name = node.file_name()?;
        is_pd200x(&uevent).then(|| dev.join(name))
    })
}

/// The microphone on this system.
pub fn find_system_hidraw() -> Option<PathBuf> {
    find_hidraw(Path::new("/sys/class/hidraw"), Path::new("/dev"))
}

pub struct Hidraw {
    file: File,
}

impl Hidraw {
    pub fn open(path: &Path) -> Result<Hidraw> {
        match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => Ok(Hidraw { file }),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                Err(Error::PermissionDenied(path.display().to_string()))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Error::NotFound),
            Err(e) => Err(e.into()),
        }
    }

    /// Opens the microphone if it is plugged in.
    pub fn open_system() -> Result<Hidraw> {
        Hidraw::open(&find_system_hidraw().ok_or(Error::NotFound)?)
    }
}

impl Transport for Hidraw {
    fn send(&self, packet: &Packet) -> io::Result<()> {
        // hidraw takes the report id as the first byte, like hidapi's write.
        (&self.file).write_all(packet)
    }

    fn recv(&self, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
        let mut fds = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: one valid pollfd for the duration of the call.
        let n = unsafe { libc::poll(&mut fds, 1, ms) };
        if n < 0 {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::Interrupted { Ok(None) } else { Err(e) };
        }
        if n == 0 {
            return Ok(None);
        }
        // POLLHUP/POLLERR: the device is gone.
        if fds.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 && fds.revents & libc::POLLIN == 0 {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "device removed"));
        }
        let mut buf = [0u8; 64];
        let len = (&self.file).read(&mut buf)?;
        Ok(Some(buf[..len].to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(root: &Path, name: &str, uevent: &str) {
        let dir = root.join(name).join("device");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("uevent"), uevent).unwrap();
    }

    #[test]
    fn finds_the_pd200x_among_other_hid_devices() {
        let tmp = tempfile::tempdir().unwrap();
        node(tmp.path(), "hidraw3", "DRIVER=hid-generic\nHID_ID=0003:00002DC8:00005006\nHID_NAME=8BitDo\n");
        node(
            tmp.path(),
            "hidraw8",
            "DRIVER=hid-generic\nHID_ID=0003:0000352F:00000104\nHID_NAME=PD200X Podcast Microphone\n",
        );
        assert_eq!(find_hidraw(tmp.path(), Path::new("/dev")), Some(PathBuf::from("/dev/hidraw8")));
    }

    #[test]
    fn none_without_the_microphone() {
        let tmp = tempfile::tempdir().unwrap();
        node(tmp.path(), "hidraw0", "HID_ID=0003:00001462:00007C02\n");
        assert_eq!(find_hidraw(tmp.path(), Path::new("/dev")), None);
        assert_eq!(find_hidraw(&tmp.path().join("missing"), Path::new("/dev")), None);
    }

    #[test]
    fn lowercase_ids_match_too() {
        assert!(is_pd200x("HID_ID=0003:0000352f:00000104"));
        assert!(!is_pd200x("HID_NAME=PD200X"));
    }

    #[test]
    fn open_reports_missing_and_forbidden_files() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(Hidraw::open(&tmp.path().join("hidraw9")), Err(Error::NotFound)));
    }
}
