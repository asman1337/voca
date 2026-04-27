//! Auto-launch on OS login.
//!
//! # Windows
//! Writes / removes the registry key
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` → `"VOCA"`.
//! Uses the `winreg` crate (already available via the `windows` dep tree;
//! we use the safe `winreg` wrapper crate added to Cargo.toml).
//!
//! # macOS
//! Writes / removes a LaunchAgent plist at
//! `~/Library/LaunchAgents/com.b3mlabs.voca.plist`.
//!
//! # Linux
//! Writes / removes a `.desktop` file under
//! `~/.config/autostart/voca.desktop` (XDG autostart spec).

/// Enable or disable auto-launch for the current user.
///
/// `exe_path` must be the absolute path to the VOCA executable.
/// Returns `Err` with a human-readable message on failure.
pub fn set(enable: bool, exe_path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return windows_set(enable, exe_path);

    #[cfg(target_os = "macos")]
    return macos_set(enable, exe_path);

    #[cfg(target_os = "linux")]
    return linux_set(enable, exe_path);

    #[allow(unreachable_code)]
    Err("auto-launch not supported on this platform".into())
}

/// Read whether auto-launch is currently active (from the OS, not just config).
pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    return windows_is_enabled();

    #[cfg(target_os = "macos")]
    return macos_is_enabled();

    #[cfg(target_os = "linux")]
    return linux_is_enabled();

    #[allow(unreachable_code)]
    false
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn windows_set(enable: bool, exe_path: &str) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_SET_VALUE,
        )
        .map_err(|e| format!("Registry open failed: {e}"))?;

    if enable {
        run_key
            .set_value("VOCA", &exe_path)
            .map_err(|e| format!("Registry write failed: {e}"))?;
        log::info!("Auto-launch enabled → {exe_path}");
    } else {
        // delete_value returns Err if the key doesn't exist — ignore that.
        let _ = run_key.delete_value("VOCA");
        log::info!("Auto-launch disabled (registry key removed)");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_is_enabled() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") else {
        return false;
    };
    run_key.get_value::<String, _>("VOCA").is_ok()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn plist_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents/com.b3mlabs.voca.plist")
}

#[cfg(target_os = "macos")]
fn macos_set(enable: bool, exe_path: &str) -> Result<(), String> {
    let path = plist_path();
    if enable {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>             <string>com.b3mlabs.voca</string>
    <key>ProgramArguments</key>  <array><string>{exe_path}</string></array>
    <key>RunAtLoad</key>         <true/>
    <key>KeepAlive</key>         <false/>
</dict>
</plist>
"#
        );
        std::fs::write(&path, plist).map_err(|e| e.to_string())?;
        log::info!("Auto-launch enabled → {}", path.display());
    } else {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        log::info!("Auto-launch disabled (plist removed)");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_is_enabled() -> bool {
    plist_path().exists()
}

// ── Linux (XDG autostart) ─────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn desktop_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join("autostart/voca.desktop")
}

#[cfg(target_os = "linux")]
fn linux_set(enable: bool, exe_path: &str) -> Result<(), String> {
    let path = desktop_path();
    if enable {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName=VOCA\nExec={exe_path}\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, desktop).map_err(|e| e.to_string())?;
        log::info!("Auto-launch enabled → {}", path.display());
    } else {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        log::info!("Auto-launch disabled (.desktop removed)");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_is_enabled() -> bool {
    desktop_path().exists()
}
