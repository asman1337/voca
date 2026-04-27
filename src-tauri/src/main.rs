// VOCA — Tauri v2 entry point
// On macOS/Windows this is the binary entry. Mobile uses lib.rs directly.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    voca_lib::run();
}
