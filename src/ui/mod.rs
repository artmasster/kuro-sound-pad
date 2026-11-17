//! The window: state, what each message does, and the subscriptions that feed it.
mod icons;
mod knob;
mod ops;
mod style;
mod view;

use crate::audio::{self, Cmd, Levels, Slot};
use crate::hotkey::{self, Action, Captured};
use crate::keys::Hotkey;
use crate::library::{self, LibSound};
use crate::settings::{self, Settings, Sound, Source, AUDIO_EXTENSIONS, PADS};
use crate::transfer::{self, Imported};
use crate::updater::{self, Outcome, Phase, Release, UpMsg, Updater};
use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use iced::{event, keyboard, mouse, window, Event, Point, Size, Subscription, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use view::{fonts, FONT};

pub const WINDOW: Size = Size::new(1280.0, 800.0);
/// How often the things that move (progress strips, the meter, the clock) are brought up to date. Thirty times a
/// second looks smooth for these and costs half of following the screen's own rate (or a quarter, at 120 Hz).
const TICK: Duration = Duration::from_millis(33);
pub const MIN_WINDOW: Size = Size::new(1120.0, 700.0);

/// What a captured key is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Pad(usize),
    Stop,
}

#[derive(Clone, Debug)]
pub enum Modal {
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

/// What the mouse picked up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carried {
    /// A sound of the library, by position in the list.
    Library(usize),
    Pad(usize),
}

/// A press that may become a drag: it does once the cursor has moved a little.
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    pub item: Carried,
    origin: Option<Point>,
    pub at: Point,
    pub moving: bool,
}

#[derive(Clone, Debug)]
pub enum Message {
    Audio(audio::Event),
    Captured(Captured),
    Frame(Instant),
    /// A pad's sound has been measured: pad, which load of the pad it was for, seconds.
    Measured(usize, u32, Option<f32>),
    Focus(bool),
    Resized(Size),
    Escape,
    Dropped(PathBuf),
    Cursor(Point),
    Released,
    /// Whether a text field of this window has the keyboard.
    Typing(bool),

    PadDown(usize),
    PadSelect(usize),
    PadHover(usize, bool),
    RowDown(usize),
    RowHover(usize, bool),
    /// Listen to a library sound, or stop listening to it.
    Audition(usize),
    /// Put a library sound on the next free pad.
    Add(usize),
    StopAll,

    KnobDown,
    KnobHover(bool),
    KnobWheel(mouse::ScrollDelta),
    PadVolume(usize, u8),
    Save,
    PadColor(usize, u8),
    PadName(usize, String),
    PickFile(usize),
    FilePicked(usize, Option<PathBuf>),
    ClearPad(usize),
    Query(String),
    Unfocus,

    Output(DeviceChoice),
    Monitor(DeviceChoice),
    RefreshDevices,
    GlobalHotkeys(bool),
    Capture(Target),
    ClearKey(Target),
    ResetKeys,

    Export,
    ExportTo(Option<PathBuf>),
    Import,
    ImportFrom(Option<PathBuf>),
    ImportConfirm,
    ClearAll,
    Open(Modal),
    CloseModal,
    /// Swallows a click (on a dialog, so it does not reach the backdrop).
    Nothing,
    Welcome(bool),

    /// News from the updater's worker threads.
    Update(UpMsg),
    /// Look for a newer version; `true` when the user asked (who then gets an answer either way).
    UpdateCheck(bool),
    UpdateNow,
    UpdateSkip,

    /// Development aid: a picture of the window, to check the layout without looking at the screen.
    Shot(window::Screenshot),
}

/// A line on the display that goes away by itself.
pub struct Notice {
    pub text: String,
    pub error: bool,
    until: Instant,
}

pub struct App {
    pub settings: Settings,
    pub library: Vec<LibSound>,
    audio: Sender<Cmd>,
    levels: Arc<Levels>,
    /// The level meter, left and right, 0 to 1: jumps up with the sound, falls back slowly.
    pub meter: [f32; 2],
    pub devices: Vec<String>,
    /// When each playing sound started and how long it runs, if the file says.
    playing: HashMap<Slot, (Instant, Option<f32>)>,
    /// The pad started last: the one the display follows.
    pub latest: Option<usize>,
    /// Pads whose file could not be read.
    pub broken: [bool; PADS],
    /// How long each pad's sound runs, once measured.
    pub seconds: [Option<f32>; PADS],
    /// Counts each pad's loads, so a measurement of a sound since replaced is dropped.
    loads: [u32; PADS],
    events: UnboundedSender<Message>,
    pub selected: Option<usize>,
    pub hover: Option<usize>,
    pub hover_row: Option<usize>,
    pub drag: Option<Drag>,
    /// The master knob while it is being turned: the cursor height and the volume when it was grabbed.
    pub knob: Option<(Option<f32>, u8)>,
    knob_hover: bool,
    /// The knob's picture for the current volume, drawn again only when it changes.
    pub knob_picture: iced::widget::svg::Handle,
    /// A file dropped from outside a moment ago and the pad it went to: more files of the same drop follow it.
    last_drop: Option<(Instant, usize)>,
    pub capture: Option<Target>,
    pub modal: Option<Modal>,
    pub query: String,
    /// The library entry being listened to.
    pub preview: Option<usize>,
    preview_pending: u32,
    focused: bool,
    typing: bool,
    pub window: Size,
    pub notice: Option<Notice>,
    pub updater: Updater,
    pub now: Instant,
    /// `KURO_LOG` is set: print what the audio thread reports and what the meter reads.
    log: bool,
    /// Frames left until the development screenshot.
    shot_in: Option<u32>,
    /// The screenshot is on its way: nothing may change until it is taken.
    shooting: bool,
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
        Event::Window(window::Event::Resized(size)) => Some(Message::Resized(size)),
        Event::Window(window::Event::FileDropped(path)) => Some(Message::Dropped(path)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(Message::Released),
        Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(named), .. }) => match named {
            keyboard::key::Named::Escape => Some(Message::Escape),
            // Tab moves the keyboard between fields: ask again who has it.
            keyboard::key::Named::Tab => Some(Message::Released),
            _ => None,
        },
        _ => None,
    }
}

/// The same, plus where the cursor is: only listened to while something is being dragged.
fn on_event_dragging(event: Event, status: event::Status, window: window::Id) -> Option<Message> {
    match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::Cursor(position)),
        other => on_event(other, status, window),
    }
}

/// Where the cursor is inside this app's window right now, as fractions of its width and height. Windows reports no
/// cursor movement while a file is dragged in from outside, so the drop asks the system directly.
#[cfg(windows)]
fn cursor_in_window() -> Option<(f32, f32)> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, GetCursorPos};
    unsafe {
        let title: Vec<u16> = crate::TITLE.encode_utf16().chain([0]).collect();
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())).ok()?;
        let mut p = POINT::default();
        GetCursorPos(&mut p).ok()?;
        ScreenToClient(hwnd, &mut p).ok().ok()?;
        let mut r = RECT::default();
        GetClientRect(hwnd, &mut r).ok()?;
        (r.right > 0 && r.bottom > 0).then(|| (p.x as f32 / r.right as f32, p.y as f32 / r.bottom as f32))
    }
}

#[cfg(not(windows))]
fn cursor_in_window() -> Option<(f32, f32)> {
    None
}

fn save_png(path: &std::path::Path, shot: &window::Screenshot) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(path)?), shot.size.width, shot.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&shot.bytes)?;
    Ok(())
}

impl App {
    /// What to do right after the window opens: nothing, unless a development aid was asked for.
    pub fn boot(&mut self) -> Task<Message> {
        // `KURO_AUTOPLAY=0,1,h5,h6,stop,2` runs those steps half a second apart: a number plays that pad as its
        // hotkey would, `h5` moves the mouse onto pad 5 as far as the window can tell, `check` looks for an update,
        // `update` accepts the one on offer, `m0+4` changes the lengths shown on those pads together, `-` waits,
        // anything else stops all.
        if let Ok(script) = std::env::var("KURO_AUTOPLAY") {
            let (audio, events) = (self.audio.clone(), self.events.clone());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                let mut over = None;
                for step in script.split(',') {
                    if step == "check" {
                        let _ = events.unbounded_send(Message::UpdateCheck(true));
                    } else if step == "update" {
                        let _ = events.unbounded_send(Message::UpdateNow);
                    } else if let Some(pads) = step.strip_prefix('m') {
                        // `m0+4`: those pads' lengths change at the same moment, as when their sounds finish
                        // being measured together after start-up (each pad has been loaded once by then).
                        for i in pads.split('+').filter_map(|n| n.parse::<usize>().ok()).filter(|i| *i < PADS) {
                            let _ = events.unbounded_send(Message::Measured(i, 1, Some(9.9)));
                        }
                    } else if step == "-" {
                        // Just wait.
                    } else if let Some(i) = step.strip_prefix('h').and_then(|n| n.parse::<usize>().ok()).filter(|i| *i < PADS) {
                        if let Some(old) = over.replace(i) {
                            let _ = events.unbounded_send(Message::PadHover(old, false));
                        }
                        let _ = events.unbounded_send(Message::PadHover(i, true));
                    } else if let Some(i) = step.parse().ok().filter(|i| *i < PADS) {
                        let _ = audio.send(Cmd::Trigger(i));
                    } else {
                        let _ = audio.send(Cmd::StopAll);
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            });
        }
        // `KURO_SHOT=<png>` saves a picture of the window and quits; `KURO_SHOT_STATE` names what to show.
        if std::env::var_os("KURO_SHOT").is_none() {
            self.updater.check(false, updater::FIRST_CHECK);
            return Task::none();
        }
        let offer = |updater: &mut Updater, phase| {
            updater.release = Some(Release { version: "2.2.0".into(), file: String::new(), sha512: String::new(), size: None });
            updater.phase = phase;
        };
        let now = Instant::now();
        self.now = now;
        self.modal = None;
        for part in std::env::var("KURO_SHOT_STATE").unwrap_or_default().split(',') {
            match part {
                "select" => self.selected = Some(1),
                "empty" => self.selected = Some(15),
                "playing" => {
                    self.playing.insert(Slot::Pad(0), (now - Duration::from_millis(600), Some(1.3)));
                    self.latest = Some(0);
                    self.meter = [0.55, 0.4];
                }
                "preview" => {
                    self.preview = Some(6);
                    self.playing.insert(Slot::Preview, (now - Duration::from_millis(900), Some(2.4)));
                    self.meter = [0.3, 0.3];
                }
                "search" => self.query = "ตบ".into(),
                "capture" => {
                    self.selected = Some(5);
                    self.capture = Some(Target::Pad(5));
                }
                "drag" => {
                    self.hover = Some(9);
                    self.drag = Some(Drag { item: Carried::Library(1), origin: Some(Point::ORIGIN), at: Point::new(640.0, 420.0), moving: true });
                }
                "swap" => {
                    self.hover = Some(14);
                    self.drag = Some(Drag { item: Carried::Pad(0), origin: Some(Point::ORIGIN), at: Point::new(820.0, 560.0), moving: true });
                }
                "error" => self.say("ไม่พบไฟล์เสียงของ \"Vine Boom\"", true),
                "welcome" => self.modal = Some(Modal::Welcome),
                "help" => self.modal = Some(Modal::Help),
                "clear" => self.modal = Some(Modal::ConfirmClear),
                "update" => offer(&mut self.updater, Phase::Available),
                "downloading" => {
                    offer(&mut self.updater, Phase::Downloading);
                    self.updater.percent = 43;
                }
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
        let (audio, levels) = audio::spawn(move |e| {
            if log {
                eprintln!("audio: {e:?}");
            }
            let _ = audio_tx.unbounded_send(Message::Audio(e));
        });
        let capture_tx = tx.clone();
        hotkey::init(audio.clone(), move |c| {
            let _ = capture_tx.unbounded_send(Message::Captured(c));
        });
        let settings_volume = settings.master_volume;
        let update_tx = tx.clone();
        let updater = Updater::new(move |m| {
            let _ = update_tx.unbounded_send(Message::Update(m));
        });
        let mut app = Self {
            modal: settings.global_hotkeys.is_none().then_some(Modal::Welcome),
            settings,
            library: library::load(),
            audio,
            levels,
            meter: [0.0; 2],
            devices: Vec::new(),
            playing: HashMap::new(),
            latest: None,
            broken: [false; PADS],
            seconds: [None; PADS],
            loads: [0; PADS],
            events: tx,
            selected: None,
            hover: None,
            hover_row: None,
            drag: None,
            knob: None,
            knob_hover: false,
            knob_picture: iced::widget::svg::Handle::from_memory(knob::svg(settings_volume, false)),
            last_drop: None,
            capture: None,
            query: String::new(),
            preview: None,
            preview_pending: 0,
            focused: true,
            typing: false,
            window: WINDOW,
            notice: None,
            updater,
            now: Instant::now(),
            log,
            shot_in: None,
            shooting: false,
        };
        app.send(Cmd::ListDevices);
        app.sync_everything();
        app
    }

    pub fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "kuro".to_string(),
            iced::theme::Palette {
                background: style::CHASSIS,
                text: style::PRINT,
                primary: style::AMBER,
                success: style::GREEN,
                danger: style::RED,
            },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let input = if self.drag.is_some() || self.knob.is_some() { event::listen_with(on_event_dragging) } else { event::listen_with(on_event) };
        // A screenshot reads what was drawn last. A layout after that draw would leave it with text that no longer
        // exists, so once the picture is ordered the window gets no more messages.
        if self.shooting {
            return Subscription::none();
        }
        let mut subs = vec![Subscription::run(events), input];
        // The tick drives the progress strips, the meter and the notice's timeout; nothing moves otherwise, and then
        // the window does no work at all.
        if !self.playing.is_empty() || self.notice.is_some() || self.shot_in.is_some() || self.meter.iter().any(|m| *m > 0.001) {
            subs.push(iced::time::every(TICK).map(Message::Frame));
        }
        // An installed copy looks for a newer version once an hour.
        if self.updater.supported() {
            subs.push(iced::time::every(updater::CHECK_EVERY).map(|_| Message::UpdateCheck(false)));
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
        self.notice = Some(Notice { text: text.into(), error, until: now + Duration::from_millis(if error { 5000 } else { 2800 }) });
    }

    pub fn is_playing(&self, slot: Slot) -> bool {
        self.playing.contains_key(&slot)
    }

    pub fn anything_playing(&self) -> bool {
        !self.playing.is_empty()
    }

    /// Seconds a playing sound has run and its length, if known; `None` when it is not playing.
    pub fn clock(&self, slot: Slot) -> Option<(f32, Option<f32>)> {
        let (start, declared) = self.playing.get(&slot)?;
        let t = self.now.saturating_duration_since(*start).as_secs_f32();
        let measured = match slot {
            Slot::Pad(i) => self.seconds[i],
            Slot::Preview => None,
        };
        let total = measured.or(*declared).filter(|s| *s > 0.0);
        Some((total.map_or(t, |s| t.min(s)), total))
    }

    /// How far a playing sound is, 0 to 1. A sound of unknown length sweeps.
    pub fn progress(&self, slot: Slot) -> Option<f32> {
        let (t, total) = self.clock(slot)?;
        Some(match total {
            Some(s) => (t / s).clamp(0.0, 1.0),
            None => (t * 0.8).fract(),
        })
    }

    fn set_master(&mut self, v: u8) {
        if v != self.settings.master_volume {
            self.settings.master_volume = v;
            self.send(Cmd::Master(v as f32 / 100.0));
            self.draw_knob();
        }
    }

    fn draw_knob(&mut self) {
        let hot = self.knob_hover || self.knob.is_some();
        self.knob_picture = iced::widget::svg::Handle::from_memory(knob::svg(self.settings.master_volume, hot));
    }

    /// The pad a drag would land on: the one the cursor is over, as the pads themselves report it.
    pub fn drop_target(&self) -> Option<usize> {
        let drag = self.drag.filter(|d| d.moving)?;
        self.hover.filter(|i| drag.item != Carried::Pad(*i))
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
        self.draw_knob();
        self.sync_outputs();
        for i in 0..PADS {
            self.sync_pad(i);
        }
        self.sync_keys();
        self.sync_hook();
    }

    /// The hook listens while it may: everywhere once allowed, else only while this window has the focus. It holds
    /// back while the user types into a field of this window or a dialog is open.
    fn sync_hook(&self) {
        hotkey::set_paused(self.modal.is_some() || (self.typing && self.focused));
        hotkey::set_active(self.focused || self.settings.global_hotkeys == Some(true));
    }

    fn stop_capture(&mut self) {
        if self.capture.take().is_some() {
            hotkey::set_capturing(false);
        }
    }

    fn close_modal(&mut self) {
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

    fn first_free(&self) -> Option<usize> {
        (0..PADS).find(|i| self.settings.pads[*i].sound.is_none())
    }

    /// Where a sound without a chosen pad goes: the selected pad if it is free, else the first free one.
    fn free_pad(&self) -> Option<usize> {
        self.selected.filter(|i| self.settings.pads[*i].sound.is_none()).or_else(|| self.first_free())
    }

    fn library_sound(&self, n: usize) -> Option<Sound> {
        let s = self.library.get(n)?;
        Some(Sound { name: s.name.clone(), source: Source::Library(s.file.clone()) })
    }

    fn pick(&self, title: &str, json: bool) -> rfd::AsyncFileDialog {
        let d = rfd::AsyncFileDialog::new().set_title(title);
        if json {
            d.add_filter("การตั้งค่า Kuro Sound Pad (JSON)", &["json"])
        } else {
            d.add_filter("ไฟล์เสียง", &AUDIO_EXTENSIONS)
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let passive = matches!(
            message,
            Message::Frame(_)
                | Message::Measured(..)
                | Message::Audio(_)
                | Message::Captured(_)
                | Message::Capture(_)
                | Message::Focus(true)
                | Message::Resized(_)
                | Message::Cursor(_)
                | Message::Released
                | Message::Typing(_)
                | Message::PadHover(..)
                | Message::RowHover(..)
                | Message::KnobHover(_)
                | Message::Update(_)
                | Message::UpdateCheck(false)
        );
        if !passive {
            self.stop_capture();
        }
        let task = self.handle(message);
        self.sync_hook();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(_) if self.shooting => {}
            Message::Frame(now) => {
                if let Some(n) = &mut self.shot_in {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        self.shot_in = None;
                        self.shooting = true;
                        // Long enough for the last frame to be drawn.
                        return Task::future(async { std::thread::sleep(Duration::from_millis(400)) })
                            .then(|()| window::get_latest())
                            .and_then(window::screenshot)
                            .map(Message::Shot);
                    }
                    // Time stands still for the picture, so it shows the same thing every time.
                    return Task::none();
                }
                let dt = now.saturating_duration_since(self.now).as_secs_f32().min(0.1);
                self.now = now;
                let peaks = self.levels.take();
                if self.log && peaks.iter().any(|p| *p > 0.0) {
                    eprintln!("meter: {:.2} {:.2}", peaks[0], peaks[1]);
                }
                for (m, p) in self.meter.iter_mut().zip(peaks) {
                    *m = p.max(*m - dt * 1.6).clamp(0.0, 1.0);
                }
                if self.notice.as_ref().is_some_and(|t| t.until <= now) {
                    self.notice = None;
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
                    match slot {
                        Slot::Pad(i) => self.latest = Some(i),
                        Slot::Preview => self.preview_pending = self.preview_pending.saturating_sub(1),
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
                    let lost = self.settings.assign_key(
                        match target {
                            Target::Pad(i) => Some(i),
                            Target::Stop => None,
                        },
                        key,
                    );
                    match lost {
                        Some(Some(i)) => self.say(format!("ปุ่ม {} ย้ายมาจากแผง {}", key.label(), i + 1), false),
                        Some(None) => self.say(format!("ปุ่ม {} ย้ายมาจากปุ่มหยุดเสียง", key.label()), false),
                        None => {}
                    }
                    self.sync_keys();
                    self.save();
                }
            }
            Message::Focus(f) => {
                self.focused = f;
                // (A window started for a screenshot may never get the focus: its staged drag stays.)
                if !f && self.shot_in.is_none() && !self.shooting {
                    self.drag = None;
                    self.knob = None;
                }
            }
            Message::Resized(size) => self.window = size,
            Message::Typing(t) => self.typing = t,
            Message::Unfocus => {
                self.typing = false;
                return ops::unfocus().discard();
            }
            Message::Escape => {
                self.drag = None;
                self.typing = false;
                if self.modal.is_some() {
                    self.close_modal();
                } else {
                    self.selected = None;
                }
                return ops::unfocus().discard();
            }
            Message::Dropped(path) => {
                // A settings file dropped on the window is an import.
                if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
                    return self.handle(Message::ImportFrom(Some(path)));
                }
                let now = Instant::now();
                // Several files dropped together arrive one by one: the first goes where the cursor is, the rest
                // onto the free pads after it.
                let follow = self.last_drop.filter(|(at, _)| now.duration_since(*at) < Duration::from_millis(400));
                let under = cursor_in_window()
                    .and_then(|(x, y)| view::pad_at(self.window, Point::new(x * self.window.width, y * self.window.height)));
                let target = match follow {
                    Some((_, last)) => (last + 1..PADS).chain(0..last).find(|i| self.settings.pads[*i].sound.is_none()),
                    None => under.or_else(|| self.free_pad()),
                };
                match (target, settings::import_file(&path)) {
                    (Some(i), Ok(sound)) => {
                        self.say(format!("\"{}\" → แผง {}", sound.name, i + 1), false);
                        self.set_sound(i, Some(sound));
                        self.selected = Some(i);
                        self.last_drop = Some((now, i));
                    }
                    (None, _) => self.say("ไม่มีแผงว่าง ลากไฟล์ไปวางบนแผงที่จะแทนที่ได้เลย", true),
                    (_, Err(e)) => self.say(e, true),
                }
            }
            Message::Cursor(at) => {
                if let Some((from, start)) = &mut self.knob {
                    let y = *from.get_or_insert(at.y);
                    let v = knob::turned(*start, y - at.y);
                    self.set_master(v);
                }
                if let Some(drag) = &mut self.drag {
                    drag.at = at;
                    let origin = *drag.origin.get_or_insert(at);
                    if !drag.moving && (at.x - origin.x).hypot(at.y - origin.y) > 6.0 {
                        drag.moving = true;
                        // A pad starts playing when pressed; one that is being carried away should not.
                        if let Carried::Pad(i) = drag.item {
                            self.send(Cmd::Stop(Slot::Pad(i)));
                        }
                    }
                }
            }
            Message::Released => {
                if self.knob.take().is_some() {
                    self.draw_knob();
                    self.save();
                }
                let target = self.drop_target();
                if let Some(drag) = self.drag.take() {
                    match (drag.moving, drag.item, target) {
                        (true, Carried::Library(n), Some(i)) => {
                            if let Some(sound) = self.library_sound(n) {
                                self.set_sound(i, Some(sound));
                                self.selected = Some(i);
                            }
                        }
                        (true, Carried::Pad(from), Some(to)) => {
                            self.send(Cmd::Stop(Slot::Pad(from)));
                            self.send(Cmd::Stop(Slot::Pad(to)));
                            self.settings.swap_pads(from, to);
                            self.sync_pad(from);
                            self.sync_pad(to);
                            self.selected = Some(to);
                            self.save();
                        }
                        // A press on a library row that never moved is a click: listen to it.
                        (false, Carried::Library(n), _) => return self.handle(Message::Audition(n)),
                        _ => {}
                    }
                }
                return ops::typing().map(Message::Typing);
            }

            Message::PadDown(i) => {
                self.selected = Some(i);
                if self.settings.pads[i].sound.is_some() {
                    self.send(Cmd::Trigger(i));
                    self.drag = Some(Drag { item: Carried::Pad(i), origin: None, at: Point::ORIGIN, moving: false });
                } else {
                    // An empty pad asks for a sound.
                    return self.handle(Message::PickFile(i));
                }
            }
            Message::PadSelect(i) => self.selected = Some(i),
            Message::PadHover(i, on) => {
                if on {
                    self.hover = Some(i);
                } else if self.hover == Some(i) {
                    self.hover = None;
                }
            }
            Message::RowDown(n) => {
                self.drag = Some(Drag { item: Carried::Library(n), origin: None, at: Point::ORIGIN, moving: false });
            }
            Message::RowHover(n, on) => {
                if on {
                    self.hover_row = Some(n);
                } else if self.hover_row == Some(n) {
                    self.hover_row = None;
                }
            }
            Message::Audition(n) => {
                if self.preview == Some(n) {
                    self.send(Cmd::Stop(Slot::Preview));
                } else if let Some(s) = self.library.get(n) {
                    match library::read(&s.file) {
                        Ok(bytes) => {
                            self.preview = Some(n);
                            self.preview_pending += 1;
                            self.send(Cmd::Preview(bytes.into()));
                        }
                        Err(e) => self.say(format!("เปิดไฟล์เสียงไม่ได้: {e}"), true),
                    }
                }
            }
            Message::Add(n) => match (self.free_pad(), self.library_sound(n)) {
                (Some(i), Some(sound)) => {
                    self.say(format!("\"{}\" → แผง {}", sound.name, i + 1), false);
                    self.set_sound(i, Some(sound));
                    self.selected = Some(i);
                }
                (None, _) => self.say("ไม่มีแผงว่าง ลากเสียงไปวางบนแผงที่จะแทนที่ได้เลย", true),
                _ => {}
            },
            Message::StopAll => self.send(Cmd::StopAll),

            Message::KnobDown => {
                self.knob = Some((None, self.settings.master_volume));
                self.draw_knob();
            }
            Message::KnobHover(on) => {
                self.knob_hover = on;
                self.draw_knob();
            }
            Message::KnobWheel(delta) => {
                let up = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y > 0.0,
                };
                let v = self.settings.master_volume;
                self.set_master(if up { (v + 5).min(100) } else { v.saturating_sub(5) });
                self.save();
            }
            Message::PadVolume(i, v) => {
                self.settings.pads[i].volume = v;
                self.send(Cmd::PadVolume(i, v as f32 / 100.0));
            }
            Message::Save => self.save(),
            Message::PadColor(i, c) => {
                self.settings.pads[i].color = Some(c);
                self.save();
            }
            Message::PadName(i, name) => {
                self.typing = true;
                if let Some(sound) = self.settings.pads[i].sound.as_mut() {
                    sound.name = name.chars().filter(|c| !c.is_control()).take(100).collect();
                    self.save();
                }
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
            Message::ClearPad(i) => {
                self.settings.pads[i].color = None;
                self.set_sound(i, None);
            }
            Message::Query(q) => {
                self.typing = true;
                self.query = q;
            }

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
                    p.color = None;
                }
                self.selected = None;
                self.sync_everything();
                self.save();
                self.settings.collect_garbage();
                self.say("ล้างแผงเสียงทั้งหมดแล้ว", false);
            }
            Message::Open(modal) => self.modal = Some(modal),
            Message::CloseModal => self.close_modal(),
            Message::Nothing => {}
            Message::Welcome(allow) => {
                self.settings.global_hotkeys = Some(allow);
                self.modal = None;
                self.save();
            }

            Message::Update(news) => match self.updater.on(news) {
                Outcome::Found(version) => {
                    if self.settings.skip_version.as_deref() != Some(version.as_str()) {
                        self.say(format!("มีรุ่นใหม่ {version} · กดอัปเดตได้ที่มุมขวาล่าง"), false);
                    }
                }
                Outcome::UpToDate => self.say(concat!("เป็นรุ่นล่าสุดแล้ว (", env!("CARGO_PKG_VERSION"), ")"), false),
                Outcome::CheckFailed(e) => self.say(format!("ตรวจหารุ่นใหม่ไม่ได้: {e}"), true),
                Outcome::Run(installer) => match updater::run(&installer) {
                    // The installer waits for this app to close, then starts the new version.
                    Ok(()) => return iced::exit(),
                    Err(e) => self.updater.fail(e),
                },
                Outcome::Nothing => {}
            },
            Message::UpdateCheck(asked) => {
                // Asking again brings back a version that was skipped.
                if asked && self.settings.skip_version.take().is_some() {
                    self.save();
                }
                self.updater.check(asked, Duration::ZERO);
            }
            Message::UpdateNow => self.updater.download(),
            Message::UpdateSkip => {
                if let Some(release) = &self.updater.release {
                    self.settings.skip_version = Some(release.version.clone());
                    self.save();
                }
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
