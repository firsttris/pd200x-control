//! The CLI end to end against the simulated microphone, with its own config
//! folder and no PipeWire tools on PATH.

use assert_cmd::Command;
use predicates::str::contains;
use std::path::Path;

fn pd200x(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("pd200x").unwrap();
    cmd.env("PD200X_SIMULATOR", "1").env("XDG_CONFIG_HOME", config).env("PATH", config);
    cmd
}

fn settings(config: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(config.join("pd200x-control/settings.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn status_shows_the_simulated_microphone() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(contains("PD200X  connected"))
        .stdout(contains("Gain          70 / 100"))
        .stdout(contains("Boost         unavailable"))
        .stdout(contains("Tone          flat, compressor off, limiter off"))
        .stdout(contains("Light         off"));
}

#[test]
fn status_as_json() {
    let tmp = tempfile::tempdir().unwrap();
    let out = pd200x(tmp.path()).args(["status", "--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["live"]["gain"], 70);
    assert_eq!(v["live"]["headphone"], 50);
    assert!(v["boostDb"].is_null());
}

#[test]
fn eq_preset_and_dynamics_are_saved() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path()).args(["eq", "natural"]).assert().success();
    pd200x(tmp.path()).args(["compressor", "-9"]).assert().success();
    pd200x(tmp.path()).args(["limiter", "off"]).assert().success();
    let s = settings(tmp.path());
    assert_eq!(s["tone"]["source"], "natural");
    assert_eq!(s["tone"]["eq"][2]["frequency"], 1320);
    assert_eq!(s["tone"]["compressor"]["thresholdDb"], -9.0);
    assert_eq!(s["tone"]["limiter"]["enabled"], false);
    pd200x(tmp.path()).arg("restore").assert().success().stdout(contains("Restored"));
}

#[test]
fn light_settings_are_saved() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path()).args(["light", "cyan"]).assert().success();
    pd200x(tmp.path()).args(["light", "brightness", "80"]).assert().success();
    pd200x(tmp.path()).args(["light", "linkage", "on"]).assert().success();
    let s = settings(tmp.path());
    assert_eq!(s["lighting"]["color"], "cyan");
    assert_eq!(s["lighting"]["brightness"], 80);
    assert_eq!(s["lighting"]["linkage"], true);
}

#[test]
fn simple_device_commands() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [&["gain", "80"][..], &["mute", "toggle"], &["monitor", "on"], &["headphone", "30"], &["nc", "25"]] {
        pd200x(tmp.path()).args(args).assert().success();
    }
}

#[test]
fn bad_input_is_explained() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path()).args(["gain", "150"]).assert().failure().stderr(contains("0–100"));
    pd200x(tmp.path()).args(["eq", "loud"]).assert().failure().stderr(contains("pd200x eq list"));
    pd200x(tmp.path()).args(["light", "pink"]).assert().failure().stderr(contains("colours"));
    pd200x(tmp.path()).args(["compressor", "-60"]).assert().failure().stderr(contains("between -40 and 0"));
    pd200x(tmp.path()).arg("boost").assert().failure().stderr(contains("wpctl is not installed"));
}

#[test]
fn restore_without_saved_settings() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path()).arg("restore").assert().success().stdout(contains("Nothing saved yet"));
    pd200x(tmp.path()).args(["eq", "list"]).assert().success().stdout(contains("low_cut_mid_boost"));
}

#[test]
fn protect_shows_and_writes_the_drop_in() {
    let tmp = tempfile::tempdir().unwrap();
    pd200x(tmp.path()).arg("protect").assert().success().stdout(contains("Protected from apps: off"));
    // No systemctl on PATH: the file is written, the restart reports the missing tool
    pd200x(tmp.path()).args(["protect", "on"]).assert().failure().stderr(contains("systemctl is not installed"));
    assert!(tmp.path().join("pipewire/pipewire-pulse.conf.d/90-pd200x-control.conf").exists());
    pd200x(tmp.path()).arg("protect").assert().success().stdout(contains("Protected from apps: on"));
}

#[test]
fn protect_in_the_flatpak_writes_the_hosts_drop_in_and_names_the_restart() {
    let sandbox = tempfile::tempdir().unwrap();
    let host = tempfile::tempdir().unwrap();
    pd200x(sandbox.path())
        .env("FLATPAK_ID", "io.github.firsttris.pd200x-control")
        .env("HOST_XDG_CONFIG_HOME", host.path())
        .args(["protect", "on"])
        .assert()
        .success()
        .stdout(contains("systemctl --user restart pipewire-pulse"));
    assert!(host.path().join("pipewire/pipewire-pulse.conf.d/90-pd200x-control.conf").exists());
    assert!(!sandbox.path().join("pipewire").exists());
}
