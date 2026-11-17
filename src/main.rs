//! Kuro Sound Pad: 16 pads of sounds, played by a click or a hotkey.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod hotkey;
mod http;
mod keys;
mod library;
mod pack;
mod settings;
mod transfer;
mod trust;
mod ui;
mod updater;

use iced::window;

pub const TITLE: &str = "Kuro Sound Pad";

/// One copy at a time: two would both answer every hotkey. The second start brings the first one's window forward.
#[cfg(windows)]
fn already_running() -> bool {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};
    unsafe {
        // Kept for the life of the process on purpose.
        let _ = CreateMutexW(None, false, w!("Local\\KuroSoundPad.single-instance"));
        if GetLastError() != ERROR_ALREADY_EXISTS {
            return false;
        }
        let title: Vec<u16> = TITLE.encode_utf16().chain([0]).collect();
        if let Ok(hwnd) = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        true
    }
}

#[cfg(not(windows))]
fn already_running() -> bool {
    false
}

fn icon() -> Option<window::Icon> {
    let mut reader = png::Decoder::new(&include_bytes!("../build/icon.png")[..]).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    window::icon::from_rgba(buf, info.width, info.height).ok()
}

fn main() -> iced::Result {
    if std::env::var_os("KURO_ALLOW_SECOND").is_none() && already_running() {
        return Ok(());
    }
    let mut app = iced::application(TITLE, ui::App::update, ui::App::view);
    for font in ui::fonts() {
        app = app.font(font);
    }
    app.subscription(ui::App::subscription)
        .theme(ui::App::theme)
        .default_font(ui::FONT)
        .antialiasing(true)
        .window(window::Settings {
            size: ui::WINDOW,
            min_size: Some(ui::MIN_WINDOW),
            icon: icon(),
            ..Default::default()
        })
        .run_with(|| {
            let mut app = ui::App::new();
            let task = app.boot();
            (app, task)
        })
}
