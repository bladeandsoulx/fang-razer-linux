#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod display;
mod panel;

use client::Client;
use fang_protocol::api::{Boost, Command, FanMode, GpuMode, KbdEffect, LogoMode, PerfMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct UiSettings {
    autostart: bool,
    close_to_tray: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings {
            autostart: false,
            close_to_tray: true,
        }
    }
}

struct Ui(Mutex<UiSettings>);

fn settings_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> UiSettings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Replace the settings file atomically so a crash cannot leave truncated
/// JSON that disagrees with the already-applied operating-system state.
fn save_settings(path: &Path, settings: &UiSettings) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("settings path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("create settings directory {}: {e}", parent.display()))?;

    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = std::path::PathBuf::from(temporary);
    let result = (|| {
        let bytes =
            serde_json::to_vec_pretty(settings).map_err(|e| format!("serialize settings: {e}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| format!("open {}: {e}", temporary.display()))?;
        file.write_all(&bytes)
            .map_err(|e| format!("write {}: {e}", temporary.display()))?;
        file.sync_all()
            .map_err(|e| format!("sync {}: {e}", temporary.display()))?;
        std::fs::rename(&temporary, path).map_err(|e| {
            format!(
                "replace settings {} with {}: {e}",
                path.display(),
                temporary.display()
            )
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Apply autostart, persist settings, then expose the new state. If the file
/// write fails after changing the OS entry, restore its exact previous state.
fn commit_ui_settings<A, S>(
    previous_autostart: bool,
    settings: UiSettings,
    mut apply_autostart: A,
    save: S,
) -> Result<UiSettings, String>
where
    A: FnMut(bool) -> Result<(), String>,
    S: FnOnce(&UiSettings) -> Result<(), String>,
{
    let autostart_changed = previous_autostart != settings.autostart;
    if autostart_changed {
        apply_autostart(settings.autostart)?;
    }

    if let Err(save_error) = save(&settings) {
        if autostart_changed {
            return match apply_autostart(previous_autostart) {
                Ok(()) => Err(format!(
                    "settings: {save_error}; restored previous autostart state"
                )),
                Err(rollback_error) => Err(format!(
                    "settings: {save_error}; restoring previous autostart state also failed: \
                     {rollback_error}"
                )),
            };
        }
        return Err(format!("settings: {save_error}"));
    }

    Ok(settings)
}

#[tauri::command]
fn daemon_connected(client: State<'_, Client>) -> bool {
    client.connected()
}

#[tauri::command]
fn get_version_info(client: State<'_, Client>) -> client::VersionInfo {
    client.version_info()
}

#[tauri::command]
async fn get_status(client: State<'_, Client>) -> Result<Value, String> {
    client.request(Command::GetStatus).await
}

#[tauri::command]
async fn set_perf_mode(
    client: State<'_, Client>,
    perf_mode: PerfMode,
    cpu_boost: Option<Boost>,
    gpu_boost: Option<Boost>,
) -> Result<Value, String> {
    client
        .request(Command::SetPerfMode {
            perf_mode,
            cpu_boost,
            gpu_boost,
        })
        .await
}

#[tauri::command]
async fn set_fan(client: State<'_, Client>, fan: FanMode) -> Result<Value, String> {
    client.request(Command::SetFan { fan }).await
}

#[tauri::command]
async fn set_gpu_mode(client: State<'_, Client>, gpu_mode: GpuMode) -> Result<Value, String> {
    client.request(Command::SetGpuMode { gpu_mode }).await
}

#[tauri::command]
async fn set_bho(client: State<'_, Client>, enabled: bool, threshold: u8) -> Result<Value, String> {
    client.request(Command::SetBho { enabled, threshold }).await
}

#[tauri::command]
async fn set_lighting(
    client: State<'_, Client>,
    brightness: Option<u8>,
    kbd_effect: Option<KbdEffect>,
    logo_led: Option<LogoMode>,
) -> Result<Value, String> {
    client
        .request(Command::SetLighting {
            brightness,
            kbd_effect,
            logo_led,
        })
        .await
}

/// Open an http(s) URL in the user's browser (credits / donation links).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("refusing to open non-http(s) url".into());
    }
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("xdg-open: {e}"))
}

#[tauri::command]
async fn get_display() -> Result<display::DisplayInfo, String> {
    tokio::task::spawn_blocking(display::get)
        .await
        .map_err(|e| format!("display worker failed: {e}"))
}

/// External-monitor color-temperature preset over DDC/CI (handled by fangd,
/// which owns i2c access). `value` is the VCP 0x14 code from Status.
#[tauri::command]
async fn set_color_preset(client: State<'_, Client>, value: u8) -> Result<Value, String> {
    client.request(Command::SetColorPreset { value }).await
}

/// External-monitor brightness over DDC/CI (VCP 0x10), also handled by fangd.
/// `value` is a 0..=100 percent of the monitor's luminance range.
#[tauri::command]
async fn set_monitor_brightness(client: State<'_, Client>, value: u8) -> Result<Value, String> {
    client
        .request(Command::SetMonitorBrightness { value })
        .await
}

/// Immediately retry external-monitor DDC/CI discovery.
#[tauri::command]
async fn rescan_ddc(client: State<'_, Client>) -> Result<Value, String> {
    client.request(Command::RescanDdc).await
}

/// Toggle AC/battery perf-profile automation and set the per-source profiles.
#[tauri::command]
async fn set_auto_power(
    client: State<'_, Client>,
    enabled: bool,
    ac_profile: PerfMode,
    battery_profile: PerfMode,
    ac_fan: FanMode,
    battery_fan: FanMode,
) -> Result<Value, String> {
    client
        .request(Command::SetAutoPower {
            enabled,
            ac_profile,
            battery_profile,
            ac_fan,
            battery_fan,
        })
        .await
}

#[tauri::command]
async fn set_refresh_rate(hz: u32) -> Result<display::DisplayInfo, String> {
    tokio::task::spawn_blocking(move || display::set(hz))
        .await
        .map_err(|e| format!("display worker failed: {e}"))?
}

#[tauri::command]
fn get_panel() -> panel::PanelInfo {
    panel::get()
}

#[tauri::command]
fn set_panel_brightness(percent: u8) -> Result<panel::PanelInfo, String> {
    panel::set(percent)
}

#[tauri::command]
fn get_ui_settings(ui: State<'_, Ui>) -> UiSettings {
    *ui.0.lock().unwrap()
}

#[tauri::command]
fn set_ui_settings(
    app: AppHandle,
    ui: State<'_, Ui>,
    settings: UiSettings,
) -> Result<UiSettings, String> {
    // Serialize concurrent UI saves and keep readers on the last fully
    // committed value until both the OS and disk operations succeed.
    let mut current =
        ui.0.lock()
            .map_err(|_| "UI settings lock is poisoned".to_string())?;
    let path = settings_path(&app).ok_or("application config directory is unavailable")?;
    let autolaunch = app.autolaunch();
    let previous_autostart = autolaunch
        .is_enabled()
        .map_err(|e| format!("read autostart state: {e}"))?;
    let confirmed = commit_ui_settings(
        previous_autostart,
        settings,
        |enabled| {
            let result = if enabled {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };
            result.map_err(|e| format!("autostart: {e}"))
        },
        |settings| save_settings(&path, settings),
    )?;
    *current = confirmed;
    Ok(confirmed)
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn notify_already_running(app: &AppHandle) {
    show_main_window(app);
    app.dialog()
        .message("Fang is already running. The existing window has been brought to the front.")
        .title("Fang is already open")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Fang", true, None::<&str>)?;
    let silent = MenuItem::with_id(app, "mode:silent", "Silent", true, None::<&str>)?;
    let balanced = MenuItem::with_id(app, "mode:balanced", "Balanced", true, None::<&str>)?;
    let gaming = MenuItem::with_id(app, "mode:gaming", "Gaming", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Fang", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &silent,
            &balanced,
            &gaming,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("fang")
        .icon(app.default_window_icon().expect("bundled icon").clone())
        .tooltip("Fang")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            if id == "open" {
                show_main_window(app);
            } else if id == "quit" {
                app.exit(0);
            } else if let Some(mode) = id.strip_prefix("mode:") {
                let Ok(perf_mode) = serde_json::from_value::<PerfMode>(Value::String(mode.into()))
                else {
                    return;
                };
                let client = app.state::<Client>().inner().clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    match client
                        .request(Command::SetPerfMode {
                            perf_mode,
                            cpu_boost: None,
                            gpu_boost: None,
                        })
                        .await
                    {
                        Ok(status) => {
                            use tauri::Emitter;
                            let _ = app.emit("fang://status", status);
                        }
                        Err(e) => log::warn!("tray mode switch failed: {e}"),
                    }
                });
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        // This must remain the first plugin so a second process exits before it
        // can initialize another Fang window, tray, or daemon client.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            notify_already_running(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let mut settings = load_settings(&handle);
            match handle.autolaunch().is_enabled() {
                Ok(actual) if actual != settings.autostart => {
                    log::warn!(
                        "saved autostart={} disagrees with OS autostart={actual}; using OS state",
                        settings.autostart
                    );
                    settings.autostart = actual;
                    if let Some(path) = settings_path(&handle) {
                        if let Err(error) = save_settings(&path, &settings) {
                            log::warn!("could not reconcile saved autostart state: {error}");
                        }
                    }
                }
                Ok(_) => {}
                Err(error) => log::warn!("could not read OS autostart state: {error}"),
            }
            app.manage(Ui(Mutex::new(settings)));
            app.manage(client::spawn(handle.clone()));
            build_tray(&handle)?;
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .state::<Ui>()
                    .0
                    .lock()
                    .unwrap()
                    .close_to_tray;
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            daemon_connected,
            get_version_info,
            get_status,
            set_perf_mode,
            set_fan,
            set_gpu_mode,
            set_bho,
            set_lighting,
            set_color_preset,
            set_monitor_brightness,
            rescan_ddc,
            set_auto_power,
            open_url,
            get_display,
            set_refresh_rate,
            get_panel,
            set_panel_brightness,
            get_ui_settings,
            set_ui_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fang");
}

#[cfg(test)]
mod tests {
    use super::{commit_ui_settings, UiSettings};
    use std::cell::{Cell, RefCell};

    fn settings(autostart: bool) -> UiSettings {
        UiSettings {
            autostart,
            close_to_tray: true,
        }
    }

    #[test]
    fn failed_os_change_is_never_saved() {
        let save_called = Cell::new(false);
        let error = commit_ui_settings(
            false,
            settings(true),
            |_| Err("plugin refused change".into()),
            |_| {
                save_called.set(true);
                Ok(())
            },
        )
        .expect_err("OS failure must surface");

        assert_eq!(error, "plugin refused change");
        assert!(!save_called.get());
    }

    #[test]
    fn failed_save_rolls_back_the_os_change() {
        let applied = RefCell::new(Vec::new());
        let error = commit_ui_settings(
            false,
            settings(true),
            |enabled| {
                applied.borrow_mut().push(enabled);
                Ok(())
            },
            |_| Err("disk full".into()),
        )
        .expect_err("save failure must surface");

        assert_eq!(&*applied.borrow(), &[true, false]);
        assert!(error.contains("disk full"), "{error}");
        assert!(
            error.contains("restored previous autostart state"),
            "{error}"
        );
    }

    #[test]
    fn state_is_returned_only_after_a_successful_save() {
        let os_calls = Cell::new(0);
        let save_called = Cell::new(false);
        let desired = settings(false);
        let confirmed = commit_ui_settings(
            false,
            desired,
            |_| {
                os_calls.set(os_calls.get() + 1);
                Ok(())
            },
            |_| {
                save_called.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(confirmed, desired);
        assert_eq!(os_calls.get(), 0, "unchanged autostart needs no OS call");
        assert!(save_called.get());
    }
}
