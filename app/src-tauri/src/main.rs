#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod display;
mod panel;

use client::Client;
use fang_protocol::api::{Boost, Command, FanMode, GpuMode, KbdEffect, LogoMode, PerfMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

#[tauri::command]
fn daemon_connected(client: State<'_, Client>) -> bool {
    client.connected()
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
fn get_display() -> display::DisplayInfo {
    display::get()
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
fn set_refresh_rate(hz: u32) -> Result<display::DisplayInfo, String> {
    display::set(hz)
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
fn set_ui_settings(app: AppHandle, ui: State<'_, Ui>, settings: UiSettings) -> Result<(), String> {
    *ui.0.lock().unwrap() = settings;
    if let Some(p) = settings_path(&app) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&p, serde_json::to_vec_pretty(&settings).unwrap())
            .map_err(|e| e.to_string())?;
    }
    let autolaunch = app.autolaunch();
    let result = if settings.autostart {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| format!("autostart: {e}"))
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Fang", true, None::<&str>)?;
    let silent = MenuItem::with_id(app, "mode:silent", "Silent", true, None::<&str>)?;
    let balanced = MenuItem::with_id(app, "mode:balanced", "Balanced", true, None::<&str>)?;
    let creator = MenuItem::with_id(app, "mode:creator", "Creator", true, None::<&str>)?;
    let gaming = MenuItem::with_id(app, "mode:gaming", "Gaming", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Fang", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &silent,
            &balanced,
            &creator,
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(Ui(Mutex::new(load_settings(&handle))));
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
            get_status,
            set_perf_mode,
            set_fan,
            set_gpu_mode,
            set_bho,
            set_lighting,
            set_color_preset,
            set_monitor_brightness,
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
