//! Hotkeys that work while another app has the focus: a low-level keyboard hook on its own thread.
//!
//! The hook only compares each key press with the registered hotkeys: nothing is recorded, and keys are passed on
//! untouched (the pad keys are ordinary letters, typing must keep working). It is installed only while it is wanted:
//! always when the user allowed global hotkeys, otherwise only while this app's window has the focus.
use crate::audio::Cmd;
use crate::keys::Hotkey;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pad(usize),
    Stop,
}

#[derive(Clone, Copy, Debug)]
pub enum Captured {
    Key(Hotkey),
    Cancelled,
}

struct State {
    bindings: Vec<(Hotkey, Action)>,
    audio: Option<Sender<Cmd>>,
    on_capture: Option<Box<dyn Fn(Captured) + Send>>,
    /// The next key press goes to `on_capture` instead of playing anything.
    capturing: bool,
    /// The user is typing into a text field of this app.
    paused: bool,
    held: [bool; 256],
    wanted: bool,
    thread: Option<u32>,
}

static STATE: Mutex<State> = Mutex::new(State {
    bindings: Vec::new(),
    audio: None,
    on_capture: None,
    capturing: false,
    paused: false,
    held: [false; 256],
    wanted: false,
    thread: None,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn init(audio: Sender<Cmd>, on_capture: impl Fn(Captured) + Send + 'static) {
    let mut s = state();
    s.audio = Some(audio);
    s.on_capture = Some(Box::new(on_capture));
}

pub fn set_bindings(bindings: Vec<(Hotkey, Action)>) {
    state().bindings = bindings;
}

pub fn set_paused(paused: bool) {
    state().paused = paused;
}

pub fn set_capturing(capturing: bool) {
    state().capturing = capturing;
}

/// What a key event means. `true` swallows the key (only a key captured for a new hotkey is swallowed).
fn on_key(vk: u32, down: bool, mods: impl FnOnce() -> (bool, bool, bool)) -> bool {
    let mut s = state();
    let Some(held) = s.held.get_mut(vk as usize) else { return false };
    let repeat = *held;
    *held = down;
    let vk = vk as u16;
    if !down || repeat || crate::keys::is_modifier(vk) {
        return false;
    }
    if s.capturing {
        s.capturing = false;
        let (ctrl, shift, alt) = mods();
        let captured =
            if vk == crate::keys::VK_ESCAPE { Captured::Cancelled } else { Captured::Key(Hotkey { vk, ctrl, shift, alt }) };
        if let Some(f) = &s.on_capture {
            f(captured);
        }
        return true;
    }
    if s.paused || s.bindings.iter().all(|(k, _)| k.vk != vk) {
        return false;
    }
    let (ctrl, shift, alt) = mods();
    let key = Hotkey { vk, ctrl, shift, alt };
    if let (Some(&(_, action)), Some(audio)) = (s.bindings.iter().find(|(k, _)| *k == key), &s.audio) {
        let _ = audio.send(match action {
            Action::Pad(i) => Cmd::Trigger(i),
            Action::Stop => Cmd::StopAll,
        });
    }
    false
}

#[cfg(windows)]
mod imp {
    use super::{on_key, state};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    fn pressed(vk: i32) -> bool {
        unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
    }

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if on_key(info.vkCode, down, || (pressed(0x11), pressed(0x10), pressed(0x12))) {
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    fn run() {
        unsafe {
            let module = GetModuleHandleW(None).ok().map(|m| m.into());
            let Ok(hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), module, 0) else {
                state().thread = None;
                return;
            };
            let mut msg = MSG::default();
            // Gives this thread its message queue, so `set_active(false)` has somewhere to post WM_QUIT.
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            let keep = {
                let mut s = state();
                s.held = [false; 256];
                s.thread = Some(GetCurrentThreadId());
                s.wanted
            };
            if keep {
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            }
            let _ = UnhookWindowsHookEx(hook);
            let again = {
                let mut s = state();
                s.thread = None;
                s.wanted
            };
            // Wanted again while this one was closing.
            if again {
                super::set_active(true);
            }
        }
    }

    pub fn start() {
        let _ = std::thread::Builder::new().name("hotkeys".into()).spawn(run);
    }

    pub fn stop(thread: u32) {
        unsafe {
            let _ = PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn start() {}
    pub fn stop(_thread: u32) {}
}

/// Installs or removes the hook.
pub fn set_active(active: bool) {
    let mut s = state();
    let was = s.wanted;
    s.wanted = active;
    match (active, s.thread) {
        // 0 marks a thread that is starting and has not published its id yet.
        (true, None) => {
            s.thread = Some(0);
            drop(s);
            imp::start();
        }
        (false, Some(id)) if id != 0 && was => {
            drop(s);
            imp::stop(id);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn matching_capturing_and_repeats() {
        let (tx, rx) = mpsc::channel();
        let (ctx, crx) = mpsc::channel();
        init(tx, move |c| {
            let _ = ctx.send(c);
        });
        set_bindings(vec![
            (Hotkey::plain(0x37), Action::Pad(0)),
            (Hotkey { vk: 0x41, ctrl: true, shift: false, alt: false }, Action::Pad(5)),
            (Hotkey::plain(0x59), Action::Stop),
        ]);
        let none = || (false, false, false);
        let ctrl = || (true, false, false);

        assert!(!on_key(0x37, true, none));
        assert!(matches!(rx.try_recv(), Ok(Cmd::Trigger(0))));
        // Held down: Windows repeats the key, the pad must not restart.
        assert!(!on_key(0x37, true, none));
        assert!(rx.try_recv().is_err());
        on_key(0x37, false, none);
        on_key(0x37, true, none);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Trigger(0))));
        on_key(0x37, false, none);

        // The modifiers must match exactly.
        on_key(0x41, true, none);
        assert!(rx.try_recv().is_err());
        on_key(0x41, false, none);
        on_key(0x41, true, ctrl);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Trigger(5))));
        on_key(0x41, false, none);
        on_key(0x37, true, ctrl);
        assert!(rx.try_recv().is_err());
        on_key(0x37, false, none);

        on_key(0x59, true, none);
        assert!(matches!(rx.try_recv(), Ok(Cmd::StopAll)));
        on_key(0x59, false, none);

        set_paused(true);
        on_key(0x37, true, none);
        assert!(rx.try_recv().is_err());
        on_key(0x37, false, none);
        set_paused(false);

        // Capturing: a modifier alone is not a hotkey, the next real key is taken and swallowed.
        set_capturing(true);
        assert!(!on_key(0x11, true, none));
        assert!(on_key(0x37, true, ctrl));
        assert!(rx.try_recv().is_err());
        match crx.try_recv() {
            Ok(Captured::Key(k)) => assert_eq!(k, Hotkey { vk: 0x37, ctrl: true, shift: false, alt: false }),
            other => panic!("{other:?}"),
        }
        on_key(0x37, false, none);
        set_capturing(true);
        assert!(on_key(0x1B, true, none));
        assert!(matches!(crx.try_recv(), Ok(Captured::Cancelled)));
    }
}
