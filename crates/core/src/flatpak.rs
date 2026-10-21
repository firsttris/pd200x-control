//! What changes inside the Flatpak sandbox (docs/installation.md#flatpak).
//! The native builds behave as before: everything here is `None` or `false`
//! outside a Flatpak.

use std::path::{Path, PathBuf};

/// The Flatpak's app ID from `FLATPAK_ID`, which Flatpak sets in every
/// sandbox; `None` outside one.
pub fn id_from(var: impl Fn(&str) -> Option<String>) -> Option<String> {
    var("FLATPAK_ID").filter(|id| !id.is_empty())
}

pub fn id() -> Option<String> {
    id_from(|k| std::env::var(k).ok())
}

pub fn is_sandboxed() -> bool {
    id().is_some()
}

/// The host's config folder, where PipeWire reads its drop-ins. Natively
/// `$XDG_CONFIG_HOME`; in a Flatpak that points into
/// `~/.var/app/<id>/config`, so it is `$HOST_XDG_CONFIG_HOME` there, which
/// Flatpak passes on when the host sets it. `~/.config` otherwise.
pub fn host_config_home_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let key = if id_from(&var).is_some() { "HOST_XDG_CONFIG_HOME" } else { "XDG_CONFIG_HOME" };
    match var(key).filter(|x| !x.is_empty()) {
        Some(x) => Some(x.into()),
        None => var("HOME").filter(|h| !h.is_empty()).map(|h| Path::new(&h).join(".config")),
    }
}

/// The folder the desktop's tray host can read the tray icon from: the one
/// folder with the same path inside the sandbox and on the host,
/// `$XDG_RUNTIME_DIR/app/<id>` (the sandbox's own `$XDG_RUNTIME_DIR` is
/// private). `None` outside a Flatpak.
pub fn shared_runtime_dir_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let id = id_from(&var)?;
    let runtime = var("XDG_RUNTIME_DIR").filter(|d| !d.is_empty())?;
    Some(Path::new(&runtime).join("app").join(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
    }

    const FLATPAK: &str = "io.github.firsttris.pd200x-control";

    #[test]
    fn id_only_inside_a_flatpak() {
        assert_eq!(id_from(env(&[("FLATPAK_ID", FLATPAK)])).as_deref(), Some(FLATPAK));
        assert_eq!(id_from(env(&[("FLATPAK_ID", "")])), None);
        assert_eq!(id_from(env(&[])), None);
    }

    #[test]
    fn host_config_natively_follows_xdg_config_home() {
        let native = env(&[("XDG_CONFIG_HOME", "/cfg"), ("HOST_XDG_CONFIG_HOME", "/host"), ("HOME", "/home/u")]);
        assert_eq!(host_config_home_from(native), Some(PathBuf::from("/cfg")));
        assert_eq!(host_config_home_from(env(&[("HOME", "/home/u")])), Some(PathBuf::from("/home/u/.config")));
        assert_eq!(host_config_home_from(env(&[])), None);
    }

    #[test]
    fn host_config_in_a_flatpak_is_the_hosts_not_the_sandboxes() {
        let sandbox = env(&[
            ("FLATPAK_ID", FLATPAK),
            ("XDG_CONFIG_HOME", "/home/u/.var/app/io.github.firsttris.pd200x-control/config"),
            ("HOST_XDG_CONFIG_HOME", "/home/u/cfg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(host_config_home_from(sandbox), Some(PathBuf::from("/home/u/cfg")));
        // The host has no XDG_CONFIG_HOME: Flatpak sets no HOST_ variable either.
        let sandbox = env(&[
            ("FLATPAK_ID", FLATPAK),
            ("XDG_CONFIG_HOME", "/home/u/.var/app/io.github.firsttris.pd200x-control/config"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(host_config_home_from(sandbox), Some(PathBuf::from("/home/u/.config")));
    }

    #[test]
    fn shared_runtime_dir_only_inside_a_flatpak() {
        let sandbox = env(&[("FLATPAK_ID", FLATPAK), ("XDG_RUNTIME_DIR", "/run/user/1000")]);
        assert_eq!(
            shared_runtime_dir_from(sandbox),
            Some(PathBuf::from("/run/user/1000/app/io.github.firsttris.pd200x-control"))
        );
        assert_eq!(shared_runtime_dir_from(env(&[("XDG_RUNTIME_DIR", "/run/user/1000")])), None);
    }
}
