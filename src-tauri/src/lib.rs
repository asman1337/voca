//! VOCA — library root.
//!
//! Declares all subsystem modules, wires the [`Pipeline`] into Tauri's
//! managed state, registers IPC commands, and starts the hotkey daemon.

mod audio;
mod autolaunch;
mod config;
mod downloader;
mod hotkey;
mod inject;
mod orb;
mod pipeline;
mod stt;

use std::sync::{Arc, Mutex};
use tauri::Manager;

use config::AppConfig;
use pipeline::{Pipeline, SharedPipeline};

/// Application entry point (also used by mobile via `tauri::mobile_entry_point`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ── 1. Load config (writes defaults on first run) ─────────────
            let config = AppConfig::load();
            log::info!("Config loaded from {:?}", AppConfig::config_path());

            // ── 2. Create the central pipeline ────────────────────────────
            let pipeline: SharedPipeline = Arc::new(Pipeline::new(app.handle().clone(), config.clone()));
            app.manage(Arc::clone(&pipeline));

            // ── 3. Store config in Tauri state for IPC commands ───────────
            app.manage(Mutex::new(config.clone()));

            // ── 4. Start global hotkey daemon ──────────────────────────────
            hotkey::setup(Arc::clone(&pipeline));

            // ── 5. System tray with menu ───────────────────────────────────
            {
                use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

                let show_i = MenuItem::with_id(app, "show_hide", "Show / Hide Orb", true, None::<&str>)?;
                let sep1   = PredefinedMenuItem::separator(app)?;
                let mute_i = MenuItem::with_id(app, "mute", "Toggle Mute", true, None::<&str>)?;
                let sep2   = PredefinedMenuItem::separator(app)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit VOCA", true, None::<&str>)?;

                let menu = Menu::with_items(app, &[&show_i, &sep1, &mute_i, &sep2, &quit_i])?;

                let tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().cloned().expect("app icon missing"))
                    .tooltip("VOCA — Voice to Cursor, Always")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show_hide" => {
                            if let Some(win) = app.get_webview_window("orb") {
                                if win.is_visible().unwrap_or(false) {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                }
                            }
                        }
                        "mute" => {
                            if let Some(pl) = app.try_state::<SharedPipeline>() {
                                pl.handle_mute_toggle();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        // Left-click toggles orb visibility
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(win) = app.get_webview_window("orb") {
                                if win.is_visible().unwrap_or(false) {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                }
                            }
                        }
                    })
                    .build(app)?;

                // Keep the tray alive for the lifetime of the app.
                app.manage(tray);
            }

            // ── 6. Apply auto-launch preference from config ─────────────
            {
                let exe = std::env::current_exe()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let want = config.auto_launch;
                let have = autolaunch::is_enabled();
                if want != have {
                    if let Err(e) = autolaunch::set(want, &exe) {
                        log::warn!("Auto-launch sync failed: {e}");
                    }
                }
            }

            // ── 7. Linux session type detection ───────────────────────────
            #[cfg(target_os = "linux")]
            {
                let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
                let on_x11     = std::env::var("DISPLAY").is_ok();
                if on_wayland {
                    log::info!("Linux: Wayland session detected — injection via ydotool (requires ydotoold running)");
                } else if on_x11 {
                    log::info!("Linux: X11 session detected — injection via xdotool");
                } else {
                    log::warn!("Linux: no DISPLAY or WAYLAND_DISPLAY — injection will fall back to clipboard");
                }
            }

            // ── 8. Make the orb non-activating on Windows ─────────────────
            #[cfg(target_os = "windows")]
            if let Some(win) = app.get_webview_window("orb") {
                use windows::Win32::UI::WindowsAndMessaging::{
                    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
                };
                if let Ok(hwnd_tauri) = win.hwnd() {
                    let hwnd = windows::Win32::Foundation::HWND(hwnd_tauri.0);
                    unsafe {
                        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
                    }
                }
            }

            log::info!("VOCA initialised — pipeline ready, hotkeys active, tray visible");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_toggle_listening,
            cmd_start_listening,
            cmd_stop_listening,
            cmd_toggle_mute,
            cmd_get_state,
            cmd_dismiss,
            cmd_get_config,
            cmd_save_config,
            cmd_set_auto_launch,
            cmd_is_auto_launch_enabled,
            cmd_list_models,
            cmd_download_model,
            cmd_cancel_download,
            cmd_delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VOCA");
}

// ── Tauri IPC commands ────────────────────────────────────────────────────────

/// Toggle: Idle → Listening (start capture) or Listening → Transcribing.
#[tauri::command]
fn cmd_toggle_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_toggle();
}

/// Explicitly start listening (PTT press or UI button).
#[tauri::command]
fn cmd_start_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_start();
}

/// Explicitly stop and begin transcription (PTT release).
#[tauri::command]
fn cmd_stop_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_stop();
}

/// Toggle mute on / off.
#[tauri::command]
fn cmd_toggle_mute(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_mute_toggle();
}

/// Return current orb state string for frontend initialisation sync.
#[tauri::command]
fn cmd_get_state(pipeline: tauri::State<SharedPipeline>) -> String {
    pipeline.get_state_str()
}

/// Dismiss the clipboard card (Injected → Idle).
#[tauri::command]
fn cmd_dismiss(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_dismiss();
}

/// Return the current config as JSON.
#[tauri::command]
fn cmd_get_config(cfg: tauri::State<Mutex<AppConfig>>) -> AppConfig {
    cfg.lock().unwrap().clone()
}

/// Persist a new config to disk and update in-memory state.
/// Returns an error string on failure (shown to frontend).
#[tauri::command]
fn cmd_save_config(
    new_cfg: AppConfig,
    cfg: tauri::State<Mutex<AppConfig>>,
) -> Result<(), String> {
    new_cfg.save()?;
    *cfg.lock().unwrap() = new_cfg;
    Ok(())
}

/// Enable or disable auto-launch for the current user.
/// Also persists the preference into config so it survives reinstalls.
#[tauri::command]
fn cmd_set_auto_launch(
    enable: bool,
    cfg: tauri::State<Mutex<AppConfig>>,
) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();

    autolaunch::set(enable, &exe)?;

    // Persist preference to config file.
    let mut guard = cfg.lock().unwrap();
    guard.auto_launch = enable;
    guard.save()
}

/// Query whether auto-launch is currently active (reads the OS, not just config).
#[tauri::command]
fn cmd_is_auto_launch_enabled() -> bool {
    autolaunch::is_enabled()
}

// ── Model downloader IPC ─────────────────────────────────────────────────────

/// Return the full model catalogue with per-model download status.
#[tauri::command]
fn cmd_list_models() -> Vec<downloader::ModelEntry> {
    downloader::list_models()
}

/// Start downloading a model in the background.
/// Returns immediately; progress arrives as `model_download_progress`,
/// `model_download_done`, or `model_download_error` events.
#[tauri::command]
fn cmd_download_model(app: tauri::AppHandle, model_id: String) {
    use tauri::Emitter;
    tauri::async_runtime::spawn(async move {
        if let Err(e) = downloader::download_model(app.clone(), model_id.clone()).await {
            log::error!("Model download failed for '{model_id}': {e}");
            let _ = app.emit(
                "model_download_error",
                downloader::ErrorPayload { model_id, error: e },
            );
        }
    });
}

/// Cancel the in-flight model download at the next chunk boundary.
#[tauri::command]
fn cmd_cancel_download() {
    downloader::cancel_download();
}

/// Delete a downloaded model file (and its SHA-256 sidecar) from disk.
#[tauri::command]
fn cmd_delete_model(model_id: String) -> Result<(), String> {
    downloader::delete_model(&model_id)
}

