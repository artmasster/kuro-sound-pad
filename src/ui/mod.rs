//! The window: state, what each message does, and the subscriptions that feed it.
mod icons;
mod style;
mod view;

use crate::audio::{self, Cmd, Slot};
use crate::hotkey::{self, Action, Captured};
use crate::keys::Hotkey;
use crate::library::{self, LibSound};
use crate::settings::{self, Settings, Sound, Source, AUDIO_EXTENSIONS, PADS};
use crate::transfer::{self, Imported};
use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use iced::{event, keyboard, window, Event, Subscription, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub use style::Palette;
pub use view::FONT;

/// What a captured key is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Pad(usize),
    Stop,
}

#[derive(Clone, Debug)]
pub enum Modal {
    /// Picking from the bundled sounds, for a pad or (`None`) for the next free one.
    Library(Option<usize>),
    Shortcuts,
    Rename(usize, String),
    Import(Box<Imported>),
    ConfirmClear,
    /// First run: how it works, and the question about hotkeys outside the app.
    Welcome,
    Help,
}

/// One entry of a device list: a device, or the list's "none of them" choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceChoice {
    pub name: Option<String>,
    pub label: String,
}

impl std::fmt::Display for DeviceChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Audio(audio::Event),
    Captured(Captured),
    Frame(Instant),
    /// A pad's sound has been measured: pad, which load of the pad it was for, seconds.
    Measured(usize, u32, Option<f32>),
    Focus(bool),
    Escape,
    Dropped(PathBuf),

    PadPressed(usize),
    StopAll,
    EditMode(bool),
    Open(Modal),
    CloseModal,
    /// Swallows a click (on a dialog, so it does not reach the backdrop).
    Nothing,

    Master(u8),
    PadVolume(usize, u8),
    Save,
    Output(DeviceChoice),
    Monitor(DeviceChoice),
    RefreshDevices,
    GlobalHotkeys(bool),
    Theme(&'static str),

    Export,
    ExportTo(Option<PathBuf>),
    Import,
    ImportFrom(Option<PathBuf>),
    ImportConfirm,
    ClearAll,

    PickFile(usize),
    FilePicked(usize, Option<PathBuf>),
    ClearPad(usize),
    Test(usize),
    Query(String),
    Preview(usize),
    Choose(usize),

    Capture(Target),
    ClearKey(Target),
    ResetKeys,
    RenameInput(String),
    RenameDone,
    Welcome(bool),

    /// Development aid: a picture of the window, to check the layout without looking at the screen.
    Shot(window::Screenshot),
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    until: Instant,
}

pub struct App {
    pub settings: Settings,
    pub library: Vec<LibSound>,
    audio: Sender<Cmd>,
    pub devices: Vec<String>,
    /// When each playing sound started and how long it runs, if known.
    playing: HashMap<Slot, (Instant, Option<f32>)>,
    /// Pads whose file could not be read.
    pub broken: [bool; PADS],
    /// How long each pad's sound runs, once measured.
    pub seconds: [Option<f32>; PADS],
    /// Counts each pad's loads, so a measurement of a sound since replaced is dropped.
    loads: [u32; PADS],
    events: UnboundedSender<Message>,
    pub selected: Option<usize>,
    pub capture: Option<Target>,
    pub modal: Option<Modal>,
    pub query: String,
    /// The library entry being listened to.
    pub preview: Option<usize>,
    preview_pending: u32,
    focused: bool,
    pub toast: Option<Toast>,
    pub now: Instant,
    /// Frames left until the development screenshot.
    shot_in: Option<u32>,
}

/// Events from the audio thread and the keyboard hook, on their way to the window.
static EVENTS: Mutex<Option<UnboundedReceiver<Message>>> = Mutex::new(None);

fn events() -> UnboundedReceiver<Message> {
    // Only the first call gets the real receiver; a later one (there is none in practice) gets a closed stream.
    EVENTS.lock().unwrap().take().unwrap_or_else(|| unbounded().1)
}

fn on_event(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Window(window::Event::Focused) => Some(Message::Focus(true)),
        Event::Window(window::Event::Unfocused) => Some(Message::Focus(false)),
        Event::Window(window::Event::FileDropped(path)) => Some(Message::Dropped(path)),
        Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. }) => {
            Some(Message::Escape)
        }
        _ => None,
    }
}

fn save_png(path: &std::path::Path, shot: &window::Screenshot) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(path)?), shot.size.width, shot.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&shot.bytes)?;
    Ok(())
}

impl App {
    /// What to do right after the window opens: nothing, unless a screenshot was asked for (`KURO_SHOT=<png>`,
    /// with `KURO_SHOT_STATE` naming the state to show).
    pub fn boot(&mut self) -> Task<Message> {
        // Development aid: `KURO_AUTOPLAY=0,1,0,stop,2` plays those pads half a second apart, as hotkeys would.
        if let Ok(script) = std::env::var("KURO_AUTOPLAY") {
            let audio = self.audio.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                for step in script.split(',') {
                    let _ = match step.parse() {
                        Ok(i) => audio.send(Cmd::Trigger(i)),
                        Err(_) => audio.send(Cmd::StopAll),
                    };
                    std::thread::sleep(Duration::from_millis(500));
                }
            });
        }
        if std::env::var_os("KURO_SHOT").is_none() {
            return Task::none();
        }
        let state = std::env::var("KURO_SHOT_STATE").unwrap_or_default();
        if let Some(theme) = std::env::var("KURO_SHOT_THEME").ok().filter(|t| !t.is_empty()) {
            self.settings.theme = theme;
        }
        let now = Instant::now();
        for part in state.split(',') {
            match part {
                "edit" => {
                    self.settings.edit_mode = true;
                    self.selected = Some(1);
                }
                "empty" => {
                    self.settings.edit_mode = true;
                    self.selected = Some(15);
                }
                "playing" => {
                    self.playing.insert(Slot::Pad(0), (now - Duration::from_millis(900), Some(2.0)));
                    self.now = now;
                }
                "library" => self.modal = Some(Modal::Library(None)),
                "shortcuts" => {
                    self.modal = Some(Modal::Shortcuts);
                    self.capture = Some(Target::Pad(5));
                }
                "welcome" => self.modal = Some(Modal::Welcome),
                "help" => self.modal = Some(Modal::Help),
                "clear" => self.modal = Some(Modal::ConfirmClear),
                "rename" => self.modal = Some(Modal::Rename(0, "Vine Boom".into())),
                "toast" => self.say("เพิ่ม \"Vine Boom\" ลงแผง 3", false),
                "none" => self.modal = None,
                _ => {}
            }
        }
        // The picture is taken a few frames in, once the window has drawn itself.
        self.shot_in = Some(4);
        Task::none()
    }

    pub fn new() -> Self {
        let settings = Settings::load();
        settings.collect_garbage();
        let (tx, rx) = unbounded();
        *EVENTS.lock().unwrap() = Some(rx);
        let audio_tx = tx.clone();
        // Development aid: `KURO_LOG=1` prints what the audio thread reports.
        let log = std::env::var_os("KURO_LOG").is_some();
        let audio = audio::spawn(move |e| {
            if log {
                eprintln!("audio: {e:?}");
            }
            let _ = audio_tx.unbounded_send(Message::Audio(e));
        });
        let capture_tx = tx.clone();
        hotkey::init(audio.clone(), move |c| {
            let _ = capture_tx.unbounded_send(Message::Captured(c));
        });
        let mut app = Self {
            modal: settings.global_hotkeys.is_none().then_some(Modal::Welcome),
            settings,
            library: library::load(),
            audio,
            devices: Vec::new(),
            playing: HashMap::new(),
            broken: [false; PADS],
            seconds: [None; PADS],
            loads: [0; PADS],
            events: tx,
            selected: None,
            capture: None,
            query: String::new(),
            preview: None,
            preview_pending: 0,
            focused: true,
            toast: None,
            now: Instant::now(),
            shot_in: None,
        };
        app.send(Cmd::ListDevices);
        app.sync_everything();
        app
    }

    pub fn palette(&self) -> Palette {
        style::palette(&self.settings.theme)
    }

    pub fn theme(&self) -> iced::Theme {
        let p = self.palette();
        iced::Theme::custom(
            p.id.to_string(),
            iced::theme::Palette { background: p.bg, text: p.text, primary: p.accent, success: p.accent, danger: p.danger },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![Subscription::run(events), event::listen_with(on_event)];
        // Frames drive the progress bars and the toast's timeout; nothing moves otherwise.
        if !self.playing.is_empty() || self.toast.is_some() || self.shot_in.is_some() {
            subs.push(window::frames().map(Message::Frame));
        }
        Subscription::batch(subs)
    }

    fn send(&self, cmd: Cmd) {
        let _ = self.audio.send(cmd);
    }

    fn save(&mut self) {
        if let Err(e) = self.settings.save() {
            self.say(e, true);
        }
    }

    fn say(&mut self, text: impl Into<String>, error: bool) {
        let now = Instant::now();
        self.now = now;
        self.toast = Some(Toast { text: text.into(), error, until: now + Duration::from_millis(if error { 4500 } else { 2600 }) });
    }

    /// How far a playing sound is, 0 to 1; `None` when it is not playing. A sound of unknown length sweeps.
    pub fn progress(&self, slot: Slot) -> Option<f32> {
        let (start, declared) = self.playing.get(&slot)?;
        let t = self.now.saturating_duration_since(*start).as_secs_f32();
        let measured = match slot {
            Slot::Pad(i) => self.seconds[i],
            Slot::Preview => None,
        };
        Some(match measured.or(*declared) {
            Some(s) if s > 0.0 => (t / s).clamp(0.0, 1.0),
            _ => (t * 0.8).fract(),
        })
    }

    fn sync_pad(&mut self, i: usize) {
        self.loads[i] += 1;
        self.seconds[i] = None;
        let pad = &self.settings.pads[i];
        let data = match pad.sound.as_ref().map(Sound::load) {
            None => None,
            Some(Ok(data)) => Some(data),
            Some(Err(e)) => {
                self.broken[i] = true;
                self.send(Cmd::SetPad(i, None));
                self.say(e, true);
                return;
            }
        };
        self.broken[i] = false;
        self.send(Cmd::PadVolume(i, pad.volume as f32 / 100.0));
        if let Some(data) = data.clone() {
            let (events, load) = (self.events.clone(), self.loads[i]);
            std::thread::spawn(move || {
                let _ = events.unbounded_send(Message::Measured(i, load, audio::measure(data)));
            });
        }
        self.send(Cmd::SetPad(i, data));
    }

    fn sync_keys(&self) {
        let mut b: Vec<(Hotkey, Action)> =
            self.settings.pads.iter().enumerate().filter_map(|(i, p)| Some((p.key?, Action::Pad(i)))).collect();
        if let Some(k) = self.settings.stop_key {
            b.push((k, Action::Stop));
        }
        hotkey::set_bindings(b);
    }

    fn sync_outputs(&self) {
        self.send(Cmd::Outputs { main: self.settings.output_device.clone(), monitor: self.settings.monitor_device.clone() });
    }

    fn sync_everything(&mut self) {
        self.send(Cmd::Master(self.settings.master_volume as f32 / 100.0));
        self.sync_outputs();
        for i in 0..PADS {
            self.sync_pad(i);
        }
        self.sync_keys();
        self.sync_hook();
    }

    /// The hook listens while it may: everywhere once allowed, else only while this window has the focus. Open
    /// dialogs hold the hotkeys back (there may be typing in them).
    fn sync_hook(&self) {
        hotkey::set_paused(self.modal.is_some());
        hotkey::set_active(self.focused || self.settings.global_hotkeys == Some(true));
    }

    fn stop_capture(&mut self) {
        if self.capture.take().is_some() {
            hotkey::set_capturing(false);
        }
    }

    fn close_modal(&mut self) {
        if matches!(self.modal, Some(Modal::Library(_))) {
            self.send(Cmd::Stop(Slot::Preview));
            self.preview = None;
        }
        // The first-run question needs an answer, not a click outside.
        if !matches!(self.modal, Some(Modal::Welcome)) {
            self.modal = None;
        }
    }

    fn set_sound(&mut self, i: usize, sound: Option<Sound>) {
        let pad = &mut self.settings.pads[i];
        if sound.is_none() {
            pad.volume = 100;
        }
        pad.sound = sound;
        self.sync_pad(i);
        self.save();
    }

    /// Where a sound without a chosen pad goes: the selected pad while editing, if it is free; else the first free
    /// one; else the selected one, replacing its sound.
    fn free_pad(&self) -> Option<usize> {
        let free = |i: &usize| self.settings.pads[*i].sound.is_none();
        let selected = self.selected.filter(|_| self.settings.edit_mode);
        selected.filter(free).or_else(|| (0..PADS).find(free)).or(selected)
    }

    fn pick(&self, title: &str, json: bool) -> rfd::AsyncFileDialog {
        let d = rfd::AsyncFileDialog::new().set_title(title);
        if json {
            d.add_filter("การตั้งค่า Sound Pad (JSON)", &["json"])
        } else {
            d.add_filter("ไฟล์เสียง", &AUDIO_EXTENSIONS)
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(message, Message::Frame(_) | Message::Measured(..) | Message::Audio(_) | Message::Captured(_) | Message::Capture(_) | Message::Focus(true)) {
            self.stop_capture();
        }
        let task = self.handle(message);
        self.sync_hook();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(now) => {
                if let Some(n) = &mut self.shot_in {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        self.shot_in = None;
                        return window::get_latest().and_then(window::screenshot).map(Message::Shot);
                    }
                    // Time stands still for the picture, so a "playing" pad shows the same progress every time.
                    return Task::none();
                }
                self.now = now;
                if self.toast.as_ref().is_some_and(|t| t.until <= now) {
                    self.toast = None;
                }
            }
            Message::Measured(i, load, seconds) => {
                if self.loads[i] == load {
                    self.seconds[i] = seconds;
                }
            }
            Message::Audio(e) => match e {
                audio::Event::Started { slot, seconds } => {
                    let now = Instant::now();
                    self.now = now;
                    self.playing.insert(slot, (now, seconds));
                    if slot == Slot::Preview {
                        self.preview_pending = self.preview_pending.saturating_sub(1);
                    }
                }
                audio::Event::Ended(slot) => {
                    self.playing.remove(&slot);
                    // Starting another preview ends the running one first: that end is not the new one's.
                    if slot == Slot::Preview && self.preview_pending == 0 {
                        self.preview = None;
                    }
                }
                audio::Event::Devices(d) => self.devices = d,
                audio::Event::Error(e) => {
                    self.preview_pending = 0;
                    self.preview = None;
                    self.say(e, true);
                }
            },
            Message::Captured(c) => {
                if let (Some(target), Captured::Key(key)) = (self.capture.take(), c) {
                    let lost = self.settings.assign_key(match target {
                        Target::Pad(i) => Some(i),
                        Target::Stop => None,
                    }, key);
                    match lost {
                        Some(Some(i)) => self.say(format!("ปุ่ม {} เคยเป็นของแผง {} ย้ายมาให้แล้ว", key.label(), i + 1), false),
                        Some(None) => self.say(format!("ปุ่ม {} เคยเป็นปุ่มหยุดเสียง ย้ายมาให้แล้ว", key.label()), false),
                        None => {}
                    }
                    self.sync_keys();
                    self.save();
                }
            }
            Message::Focus(f) => self.focused = f,
            Message::Escape => {
                if self.modal.is_some() {
                    self.close_modal();
                } else {
                    self.selected = None;
                }
            }
            Message::Dropped(path) => {
                // A settings file dropped on the window is an import.
                if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
                    return self.handle(Message::ImportFrom(Some(path)));
                }
                match (self.free_pad(), settings::import_file(&path)) {
                    (Some(i), Ok(sound)) => {
                        self.say(format!("เพิ่ม \"{}\" ลงแผง {}", sound.name, i + 1), false);
                        self.set_sound(i, Some(sound));
                        if self.settings.edit_mode {
                            self.selected = Some(i);
                        }
                    }
                    (None, _) => self.say("ไม่มีแผงว่าง เปิดโหมดแก้ไขแล้วเลือกแผงที่จะแทนที่ก่อน", true),
                    (_, Err(e)) => self.say(e, true),
                }
            }

            Message::PadPressed(i) => {
                let filled = self.settings.pads[i].sound.is_some();
                if self.settings.edit_mode {
                    self.selected = Some(i);
                } else if !filled {
                    // An empty pad is an invitation to fill it.
                    self.settings.edit_mode = true;
                    self.selected = Some(i);
                    self.save();
                }
                if filled {
                    self.send(Cmd::Trigger(i));
                }
            }
            Message::Test(i) => self.send(Cmd::Trigger(i)),
            Message::StopAll => self.send(Cmd::StopAll),
            Message::EditMode(on) => {
                self.settings.edit_mode = on;
                if !on {
                    self.selected = None;
                }
                self.save();
            }
            Message::Open(modal) => {
                if matches!(modal, Modal::Library(_)) {
                    self.query.clear();
                }
                self.modal = Some(modal);
                if matches!(self.modal, Some(Modal::Library(_))) {
                    return iced::widget::text_input::focus(view::search_id());
                }
                if matches!(self.modal, Some(Modal::Rename(..))) {
                    return Task::batch([
                        iced::widget::text_input::focus(view::rename_id()),
                        iced::widget::text_input::select_all(view::rename_id()),
                    ]);
                }
            }
            Message::CloseModal => self.close_modal(),
            Message::Nothing => {}

            Message::Master(v) => {
                self.settings.master_volume = v;
                self.send(Cmd::Master(v as f32 / 100.0));
            }
            Message::PadVolume(i, v) => {
                self.settings.pads[i].volume = v;
                self.send(Cmd::PadVolume(i, v as f32 / 100.0));
            }
            Message::Save => self.save(),
            Message::Output(c) => {
                self.settings.output_device = c.name;
                self.sync_outputs();
                self.save();
            }
            Message::Monitor(c) => {
                self.settings.monitor_device = c.name;
                self.sync_outputs();
                self.save();
            }
            Message::RefreshDevices => {
                self.send(Cmd::ListDevices);
                self.sync_outputs();
            }
            Message::GlobalHotkeys(on) => {
                self.settings.global_hotkeys = Some(on);
                self.save();
            }
            Message::Theme(id) => {
                self.settings.theme = id.to_string();
                self.save();
            }

            Message::Export => {
                let dialog = self.pick("ส่งออกการตั้งค่า", true).set_file_name(transfer::export_file_name());
                return Task::perform(async move { dialog.save_file().await.map(|f| f.path().to_path_buf()) }, Message::ExportTo);
            }
            Message::ExportTo(Some(path)) => match transfer::export(&self.settings, &path) {
                Ok(()) => self.say("ส่งออกการตั้งค่าแล้ว", false),
                Err(e) => self.say(e, true),
            },
            Message::Import => {
                let dialog = self.pick("นำเข้าการตั้งค่า", true);
                return Task::perform(async move { dialog.pick_file().await.map(|f| f.path().to_path_buf()) }, Message::ImportFrom);
            }
            Message::ImportFrom(Some(path)) => {
                let parsed = std::fs::read_to_string(&path)
                    .map_err(|e| format!("อ่านไฟล์ไม่ได้: {e}"))
                    .and_then(|text| transfer::parse(&text, &self.library));
                match parsed {
                    Ok(imported) => self.modal = Some(Modal::Import(Box::new(imported))),
                    Err(e) => self.say(e, true),
                }
            }
            Message::ImportConfirm => {
                if let Some(Modal::Import(imported)) = self.modal.take() {
                    let skipped = imported.skipped;
                    self.send(Cmd::StopAll);
                    match imported.apply(&mut self.settings) {
                        Ok(()) if skipped > 0 => self.say(format!("นำเข้าแล้ว (ข้าม {skipped} เสียงที่อ่านไม่ได้)"), false),
                        Ok(()) => self.say("นำเข้าการตั้งค่าแล้ว", false),
                        Err(e) => self.say(e, true),
                    }
                    self.settings.normalize();
                    self.selected = None;
                    self.sync_everything();
                    self.save();
                }
            }
            Message::ClearAll => {
                // As before: the sounds and their volumes go, the hotkeys and the master volume stay.
                self.modal = None;
                self.send(Cmd::StopAll);
                for p in &mut self.settings.pads {
                    p.sound = None;
                    p.volume = 100;
                }
                self.selected = None;
                self.sync_everything();
                self.save();
                self.settings.collect_garbage();
                self.say("ล้างแผงเสียงทั้งหมดแล้ว", false);
            }

            Message::PickFile(i) => {
                let dialog = self.pick("เลือกไฟล์เสียง", false);
                return Task::perform(async move { dialog.pick_file().await.map(|f| f.path().to_path_buf()) }, move |p| {
                    Message::FilePicked(i, p)
                });
            }
            Message::FilePicked(i, Some(path)) => match settings::import_file(&path) {
                Ok(sound) => self.set_sound(i, Some(sound)),
                Err(e) => self.say(e, true),
            },
            Message::ClearPad(i) => self.set_sound(i, None),
            Message::Query(q) => self.query = q,
            Message::Preview(n) => {
                if self.preview == Some(n) {
                    self.send(Cmd::Stop(Slot::Preview));
                } else if let Some(s) = self.library.get(n) {
                    match std::fs::read(settings::library_dir().join(&s.file)) {
                        Ok(bytes) => {
                            self.preview = Some(n);
                            self.preview_pending += 1;
                            self.send(Cmd::Preview(bytes.into()));
                        }
                        Err(e) => self.say(format!("เปิดไฟล์เสียงไม่ได้: {e}"), true),
                    }
                }
            }
            Message::Choose(n) => {
                let target = match self.modal {
                    Some(Modal::Library(Some(i))) => Some(i),
                    _ => self.free_pad(),
                };
                let (Some(i), Some(s)) = (target, self.library.get(n)) else {
                    self.say("ไม่มีแผงว่าง เปิดโหมดแก้ไขแล้วเลือกแผงที่จะแทนที่ก่อน", true);
                    return Task::none();
                };
                let sound = Sound { name: s.name.clone(), source: Source::Library(s.file.clone()) };
                if matches!(self.modal, Some(Modal::Library(Some(_)))) {
                    self.close_modal();
                } else {
                    self.say(format!("เพิ่ม \"{}\" ลงแผง {}", sound.name, i + 1), false);
                }
                self.set_sound(i, Some(sound));
            }

            Message::Capture(target) => {
                if self.capture == Some(target) {
                    self.stop_capture();
                } else {
                    self.capture = Some(target);
                    hotkey::set_capturing(true);
                }
            }
            Message::ClearKey(target) => {
                match target {
                    Target::Pad(i) => self.settings.pads[i].key = None,
                    Target::Stop => self.settings.stop_key = None,
                }
                self.sync_keys();
                self.save();
            }
            Message::ResetKeys => {
                self.settings.reset_keys();
                self.sync_keys();
                self.save();
                self.say("คืนปุ่มลัดเป็นค่าเริ่มต้นแล้ว", false);
            }
            Message::RenameInput(text) => {
                if let Some(Modal::Rename(_, name)) = &mut self.modal {
                    *name = text;
                }
            }
            Message::RenameDone => {
                if let Some(Modal::Rename(i, name)) = self.modal.take() {
                    let name = settings::clean_name(&name);
                    if let (false, Some(sound)) = (name.is_empty(), self.settings.pads[i].sound.as_mut()) {
                        sound.name = name;
                        self.save();
                    }
                }
            }
            Message::Welcome(allow) => {
                self.settings.global_hotkeys = Some(allow);
                self.settings.help_seen = true;
                self.modal = None;
                self.save();
            }
            Message::Shot(shot) => {
                if let Some(path) = std::env::var_os("KURO_SHOT") {
                    let _ = save_png(std::path::Path::new(&path), &shot);
                }
                return iced::exit();
            }
            Message::ExportTo(None) | Message::ImportFrom(None) | Message::FilePicked(_, None) => {}
        }
        Task::none()
    }
}
