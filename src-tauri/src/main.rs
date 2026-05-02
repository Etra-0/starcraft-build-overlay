// src-tauri/src/main.rs
// Tauri main process binary entry point. Hides the console on Windows release
// builds, then hands off to the library crate's run() function.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    bw_build_overlay_lib::run();
}
