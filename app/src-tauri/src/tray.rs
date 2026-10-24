//! The tray icon with its menu: mute, open the window, quit. A native menu,
//! because a popup window cannot be placed next to the tray on Wayland.

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use pd200x_core::flatpak;

use crate::hub::Hub;

struct Labels {
    mute: &'static str,
    open: &'static str,
    quit: &'static str,
}

/// German for a `de*` locale, English otherwise (the tray cannot ask the web view).
fn labels(locale: &str) -> Labels {
    if locale.to_ascii_lowercase().starts_with("de") {
        Labels { mute: "Stummschalten an/aus", open: "Fenster öffnen", quit: "Beenden" }
    } else {
        Labels { mute: "Toggle mute", open: "Open window", quit: "Quit" }
    }
}

fn system_locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
        .unwrap_or_default()
}

pub fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Whether a start asks to stay in the tray (`--hidden`, for a login autostart).
pub fn asks_hidden(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    args.into_iter().any(|arg| arg.as_ref() == "--hidden")
}

/// `--hidden` keeps the window closed only with a tray, or nothing would show the app.
pub fn shows_window_at_start(hidden: bool, has_tray: bool) -> bool {
    !hidden || !has_tray
}

/// The tray needs libayatana-appindicator (or libappindicator) at run time;
/// tauri panics when neither loads, so look before creating it.
#[cfg(target_os = "linux")]
fn appindicator_available() -> bool {
    ["libayatana-appindicator3.so.1", "libappindicator3.so.1", "libayatana-appindicator3.so", "libappindicator3.so"]
        .iter()
        .any(|name| {
            let c = std::ffi::CString::new(*name).expect("no NUL in library names");
            // SAFETY: dlopen with a valid C string; the handle is closed right away.
            unsafe {
                let handle = libc::dlopen(c.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
                if handle.is_null() {
                    false
                } else {
                    libc::dlclose(handle);
                    true
                }
            }
        })
}

#[cfg(not(target_os = "linux"))]
fn appindicator_available() -> bool {
    true
}

/// Creates the tray icon; `false` when the system has no tray library.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    if !appindicator_available() {
        return Ok(false);
    }
    let l = labels(&system_locale());
    let mute = MenuItem::with_id(app, "mute", l.mute, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&mute, &open, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut tray =
        TrayIconBuilder::with_id("main").tooltip("pd200x-control").menu(&menu).on_menu_event(|app, event| match event
            .id
            .as_ref()
        {
            "mute" => {
                let _ = app.state::<Arc<Hub>>().toggle_mute();
            }
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    // The tray host on the desktop reads the icon from a file by path; from the
    // Flatpak only this folder has the same path on the host.
    if let Some(dir) = flatpak::shared_runtime_dir_from(|k| std::env::var(k).ok()) {
        tray = tray.temp_dir_path(dir.join("tray-icon"));
    }
    tray.build(app)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_locale() {
        assert_eq!(labels("de_DE.UTF-8").quit, "Beenden");
        assert_eq!(labels("en_US.UTF-8").quit, "Quit");
        assert_eq!(labels("").open, "Open window");
    }

    #[test]
    fn hidden_is_asked_by_its_own_argument() {
        assert!(asks_hidden(["pd200x-control", "--hidden"]));
        assert!(!asks_hidden(["pd200x-control"]));
        assert!(!asks_hidden(vec!["pd200x-control".to_string(), "--hidden=false".to_string()]));
    }

    #[test]
    fn the_window_stays_closed_only_hidden_with_a_tray() {
        assert!(!shows_window_at_start(true, true));
        assert!(shows_window_at_start(true, false));
        assert!(shows_window_at_start(false, true));
        assert!(shows_window_at_start(false, false));
    }
}
