// src-tauri/src/window.rs
// Owns the main BrowserWindow lifecycle: applies the always-on-top topmost
// flag, exposes per-platform opacity setting (Win32 layered window / NSWindow
// alphaValue / GtkWidget opacity), wires the F12 / Ctrl+Shift+I devtools
// toggle, and registers all Ctrl+Alt-* global shortcuts that the overlay
// relies on. Mirrors src/main/window.ts.

use crate::types::HotkeyAction;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

// objc 0.2's msg_send! / sel! macros aren't fully path-qualified internally;
// they look up sel! and sel_impl! in the call site's scope. Importing them
// here means the #[cfg(target_os = "macos")] block below can call msg_send!
// without "cannot find macro `sel` in this scope" on the macOS runner.
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

pub const MAIN_WINDOW_LABEL: &str = "main";

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
}

/// Apply per-platform window opacity. Tauri 2 doesn't ship a cross-platform
/// `set_opacity`, so we go to the OS handle.
#[allow(unused_variables)]
pub fn set_opacity(window: &WebviewWindow, value: f64) {
    let clamped = value.clamp(0.4, 1.0);

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
            LWA_ALPHA, WS_EX_LAYERED,
        };
        if let Ok(hwnd) = window.hwnd() {
            let alpha_byte = (clamped * 255.0) as u8;
            unsafe {
                let raw = hwnd.0 as isize;
                let hwnd_native: windows_sys::Win32::Foundation::HWND = raw as _;
                let ex = GetWindowLongPtrW(hwnd_native, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd_native, GWL_EXSTYLE, ex | WS_EX_LAYERED as isize);
                let _ = SetLayeredWindowAttributes(hwnd_native, 0, alpha_byte, LWA_ALPHA);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(ns_window_ptr) = window.ns_window() {
            unsafe {
                let ns_window = ns_window_ptr as *mut objc::runtime::Object;
                let _: () = msg_send![ns_window, setAlphaValue: clamped];
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(gtk_window) = window.gtk_window() {
            use gtk::prelude::WidgetExt;
            gtk_window.set_opacity(clamped);
        }
    }
}

pub fn toggle_visible(window: &WebviewWindow) {
    let is_visible = window.is_visible().unwrap_or(true);
    if is_visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn toggle_devtools(window: &WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}

/// Re-apply always-on-top after the window loses focus. Works around a
/// Tauri 2 / wry quirk on Windows where clicking the taskbar can drop the
/// overlay below it even when alwaysOnTop is set in tauri.conf.json.
pub fn install_always_on_top_keeper(window: &WebviewWindow) {
    let cloned = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(focused) = event {
            if !focused {
                let _ = cloned.set_always_on_top(true);
            }
        }
    });
}

fn shortcut_to_action(shortcut: &Shortcut) -> Option<HotkeyAction> {
    let ctrl_alt = Modifiers::CONTROL | Modifiers::ALT;
    let ctrl_alt_shift = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT;
    if shortcut.matches(ctrl_alt, Code::Digit1) {
        return Some(HotkeyAction::RaceTerran);
    }
    if shortcut.matches(ctrl_alt, Code::Digit2) {
        return Some(HotkeyAction::RaceProtoss);
    }
    if shortcut.matches(ctrl_alt, Code::Digit3) {
        return Some(HotkeyAction::RaceZerg);
    }
    if shortcut.matches(ctrl_alt, Code::KeyQ) {
        return Some(HotkeyAction::OppTerran);
    }
    if shortcut.matches(ctrl_alt, Code::KeyW) {
        return Some(HotkeyAction::OppZerg);
    }
    if shortcut.matches(ctrl_alt, Code::KeyE) {
        return Some(HotkeyAction::OppProtoss);
    }
    if shortcut.matches(ctrl_alt, Code::KeyR) {
        return Some(HotkeyAction::OppRandom);
    }
    if shortcut.matches(ctrl_alt_shift, Code::KeyB) {
        return Some(HotkeyAction::PrevBuild);
    }
    if shortcut.matches(ctrl_alt, Code::KeyB) {
        return Some(HotkeyAction::NextBuild);
    }
    if shortcut.matches(ctrl_alt, Code::PageDown) {
        return Some(HotkeyAction::NextPage);
    }
    if shortcut.matches(ctrl_alt, Code::PageUp) {
        return Some(HotkeyAction::PrevPage);
    }
    if shortcut.matches(ctrl_alt, Code::Digit0) {
        return Some(HotkeyAction::FirstPage);
    }
    if shortcut.matches(ctrl_alt, Code::KeyF) {
        return Some(HotkeyAction::ToggleFavorite);
    }
    if shortcut.matches(ctrl_alt, Code::KeyC) {
        return Some(HotkeyAction::ToggleCompact);
    }
    if shortcut.matches(ctrl_alt, Code::KeyH) {
        return Some(HotkeyAction::ToggleWindow);
    }
    None
}

fn all_shortcuts() -> Vec<Shortcut> {
    let ctrl_alt = Modifiers::CONTROL | Modifiers::ALT;
    let ctrl_alt_shift = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT;
    vec![
        Shortcut::new(Some(ctrl_alt), Code::Digit1),
        Shortcut::new(Some(ctrl_alt), Code::Digit2),
        Shortcut::new(Some(ctrl_alt), Code::Digit3),
        Shortcut::new(Some(ctrl_alt), Code::KeyQ),
        Shortcut::new(Some(ctrl_alt), Code::KeyW),
        Shortcut::new(Some(ctrl_alt), Code::KeyE),
        Shortcut::new(Some(ctrl_alt), Code::KeyR),
        Shortcut::new(Some(ctrl_alt), Code::KeyB),
        Shortcut::new(Some(ctrl_alt_shift), Code::KeyB),
        Shortcut::new(Some(ctrl_alt), Code::PageDown),
        Shortcut::new(Some(ctrl_alt), Code::PageUp),
        Shortcut::new(Some(ctrl_alt), Code::Digit0),
        Shortcut::new(Some(ctrl_alt), Code::KeyF),
        Shortcut::new(Some(ctrl_alt), Code::KeyC),
        Shortcut::new(Some(ctrl_alt), Code::KeyH),
    ]
}

pub fn register_shortcuts(app: &AppHandle) -> Result<(), tauri_plugin_global_shortcut::Error> {
    let app_handle = app.clone();
    let shortcuts = all_shortcuts();
    let manager = app.global_shortcut();
    manager.on_shortcuts(shortcuts, move |_app, shortcut, event: ShortcutEvent| {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        if let Some(action) = shortcut_to_action(shortcut) {
            let _ = app_handle.emit("hotkey", action);
        }
    })?;
    Ok(())
}

pub fn unregister_shortcuts(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}
