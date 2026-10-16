//! Tauri commands: thin wrappers around [`Hub`]; work that waits on the
//! device runs off the main thread.

use std::sync::Arc;

use tauri::State;

use pd200x_core::calibrate::Proposal;
use pd200x_core::level::Level;
use pd200x_core::model::Profile;
use pd200x_core::Change;

use crate::hub::{CmdError, CmdResult, Hub, RecordingStatus, Snapshot};

pub type AppState = Arc<Hub>;

async fn blocking<T: Send + 'static>(
    hub: &AppState,
    f: impl FnOnce(&Hub) -> CmdResult<T> + Send + 'static,
) -> CmdResult<T> {
    let hub = hub.clone();
    tauri::async_runtime::spawn_blocking(move || f(&hub)).await.map_err(|e| CmdError::other(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn snapshot(state: State<'_, AppState>) -> CmdResult<Snapshot> {
    blocking(&state, |hub| Ok(hub.snapshot())).await
}

#[tauri::command]
#[specta::specta]
pub fn level(state: State<'_, AppState>) -> Level {
    state.level()
}

#[tauri::command]
#[specta::specta]
pub async fn apply_change(state: State<'_, AppState>, change: Change) -> CmdResult<()> {
    blocking(&state, move |hub| hub.apply(change)).await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_eq(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    blocking(&state, move |hub| hub.apply_eq(&id)).await
}

#[tauri::command]
#[specta::specta]
pub fn save_profile(state: State<'_, AppState>, name: String) -> CmdResult<Profile> {
    state.save_profile(&name)
}

#[tauri::command]
#[specta::specta]
pub fn rename_profile(state: State<'_, AppState>, id: String, name: String) -> CmdResult<()> {
    state.rename_profile(&id, &name)
}

#[tauri::command]
#[specta::specta]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.delete_profile(&id)
}

#[tauri::command]
#[specta::specta]
pub async fn set_boost(state: State<'_, AppState>, db: f32) -> CmdResult<()> {
    blocking(&state, move |hub| hub.set_boost(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn calibrate(state: State<'_, AppState>, seconds: u32) -> CmdResult<Proposal> {
    blocking(&state, move |hub| hub.calibrate(u64::from(seconds.clamp(2, 15)))).await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_calibration(state: State<'_, AppState>, gain: u8, boost_db: f32) -> CmdResult<()> {
    blocking(&state, move |hub| hub.apply_calibration(gain, boost_db)).await
}

#[tauri::command]
#[specta::specta]
pub fn set_restore_on_connect(state: State<'_, AppState>, on: bool) -> CmdResult<()> {
    state.set_restore_on_connect(on)
}

#[tauri::command]
#[specta::specta]
pub async fn set_protected(state: State<'_, AppState>, on: bool) -> CmdResult<()> {
    blocking(&state, move |hub| hub.set_protected(on)).await
}

#[tauri::command]
#[specta::specta]
pub async fn start_recording(state: State<'_, AppState>) -> CmdResult<()> {
    blocking(&state, |hub| hub.start_recording()).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_recording(state: State<'_, AppState>) -> CmdResult<RecordingStatus> {
    blocking(&state, |hub| hub.stop_recording()).await
}

#[tauri::command]
#[specta::specta]
pub async fn play_recording(state: State<'_, AppState>) -> CmdResult<()> {
    blocking(&state, |hub| hub.play_recording()).await
}

#[tauri::command]
#[specta::specta]
pub async fn recording_status(state: State<'_, AppState>) -> CmdResult<RecordingStatus> {
    blocking(&state, |hub| hub.recording_status()).await
}
