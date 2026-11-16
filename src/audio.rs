//! Playback. One thread owns the output devices and every playing sound; the window and the hotkey hook send it
//! commands, so a hotkey reaches the speakers without a trip through the window.
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{cpal, Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::settings::PADS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Pad(usize),
    /// Listening to a sound in the library before picking it.
    Preview,
}

pub enum Cmd {
    /// The sound a pad plays (`None` empties it).
    SetPad(usize, Option<Arc<[u8]>>),
    PadVolume(usize, f32),
    Master(f32),
    /// Play a pad from the start, cutting its own previous run.
    Trigger(usize),
    Preview(Arc<[u8]>),
    Stop(Slot),
    StopAll,
    /// Device names; `None` for the main one follows the system default.
    Outputs { main: Option<String>, monitor: Option<String> },
    ListDevices,
}

#[derive(Clone, Debug)]
pub enum Event {
    Started { slot: Slot, seconds: Option<f32> },
    Ended(Slot),
    Devices(Vec<String>),
    Error(String),
}

struct Output {
    /// The device this stream was opened on; for the default device, to notice when the default changes.
    device: Option<String>,
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

struct Engine {
    events: Box<dyn Fn(Event) + Send>,
    pads: Vec<(Option<Arc<[u8]>>, f32)>,
    master: f32,
    want_main: Option<String>,
    want_monitor: Option<String>,
    main: Option<Output>,
    monitor: Option<Output>,
    voices: HashMap<Slot, Vec<Sink>>,
}

/// Starts the audio thread. `events` is called on that thread.
pub fn spawn(events: impl Fn(Event) + Send + 'static) -> Sender<Cmd> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            Engine {
                events: Box::new(events),
                pads: vec![(None, 1.0); PADS],
                master: 1.0,
                want_main: None,
                want_monitor: None,
                main: None,
                monitor: None,
                voices: HashMap::new(),
            }
            .run(rx)
        })
        .expect("audio thread");
    tx
}

fn device_name(d: &cpal::Device) -> Option<String> {
    d.name().ok()
}

fn default_device_name() -> Option<String> {
    cpal::default_host().default_output_device().as_ref().and_then(device_name)
}

fn list_devices() -> Vec<String> {
    let mut names: Vec<String> = cpal::default_host()
        .output_devices()
        .map(|it| it.filter_map(|d| device_name(&d)).collect())
        .unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

fn open(name: Option<&str>) -> Result<Output, String> {
    let host = cpal::default_host();
    let device = match name {
        None => host.default_output_device().ok_or("ไม่พบอุปกรณ์เสียงออก")?,
        Some(n) => host
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| device_name(d).as_deref() == Some(n))
            .ok_or_else(|| format!("ไม่พบอุปกรณ์เสียง \"{n}\""))?,
    };
    let (stream, handle) =
        OutputStream::try_from_device(&device).map_err(|e| format!("เปิดอุปกรณ์เสียงไม่ได้: {e}"))?;
    Ok(Output { device: device_name(&device), _stream: stream, handle })
}

impl Engine {
    fn run(mut self, rx: Receiver<Cmd>) {
        self.reopen();
        loop {
            // While something plays, wake up often enough to notice it ending.
            let cmd = if self.voices.is_empty() {
                rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
            } else {
                rx.recv_timeout(Duration::from_millis(25))
            };
            match cmd {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.reap();
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::SetPad(i, data) => {
                if let Some(p) = self.pads.get_mut(i) {
                    p.0 = data;
                    self.stop(Slot::Pad(i));
                }
            }
            Cmd::PadVolume(i, v) => {
                if let Some(p) = self.pads.get_mut(i) {
                    p.1 = v;
                    self.apply_volume(Slot::Pad(i));
                }
            }
            Cmd::Master(v) => {
                self.master = v;
                let slots: Vec<Slot> = self.voices.keys().copied().collect();
                for s in slots {
                    self.apply_volume(s);
                }
            }
            Cmd::Trigger(i) => {
                if let Some((Some(data), _)) = self.pads.get(i) {
                    let data = data.clone();
                    self.play(Slot::Pad(i), data);
                }
            }
            Cmd::Preview(data) => self.play(Slot::Preview, data),
            Cmd::Stop(slot) => self.stop(slot),
            Cmd::StopAll => {
                let slots: Vec<Slot> = self.voices.keys().copied().collect();
                for s in slots {
                    self.stop(s);
                }
            }
            Cmd::Outputs { main, monitor } => {
                // The same device twice would play everything doubled.
                let monitor = monitor.filter(|m| Some(m) != main.as_ref());
                if main != self.want_main || monitor != self.want_monitor || self.main.is_none() {
                    self.want_main = main;
                    self.want_monitor = monitor;
                    self.reopen();
                }
            }
            Cmd::ListDevices => (self.events)(Event::Devices(list_devices())),
        }
    }

    /// Opens the wanted devices again. Whatever is playing stops: its sinks belong to the old streams.
    fn reopen(&mut self) {
        let slots: Vec<Slot> = self.voices.keys().copied().collect();
        for s in slots {
            self.stop(s);
        }
        self.main = None;
        self.monitor = None;
        match open(self.want_main.as_deref()) {
            Ok(o) => self.main = Some(o),
            Err(e) => (self.events)(Event::Error(e)),
        }
        if let Some(name) = self.want_monitor.clone() {
            // A monitor that turns out to be the default device already plays as the main one.
            if self.main.as_ref().and_then(|m| m.device.as_ref()) != Some(&name) {
                match open(Some(&name)) {
                    Ok(o) => self.monitor = Some(o),
                    Err(e) => (self.events)(Event::Error(e)),
                }
            }
        }
    }

    fn volume(&self, slot: Slot) -> f32 {
        match slot {
            Slot::Pad(i) => self.master * self.pads.get(i).map_or(1.0, |p| p.1),
            Slot::Preview => self.master * 0.5,
        }
    }

    fn apply_volume(&mut self, slot: Slot) {
        let v = self.volume(slot);
        if let Some(sinks) = self.voices.get(&slot) {
            for s in sinks {
                s.set_volume(v);
            }
        }
    }

    fn stop(&mut self, slot: Slot) {
        if let Some(sinks) = self.voices.remove(&slot) {
            for s in sinks {
                s.stop();
            }
            (self.events)(Event::Ended(slot));
        }
    }

    fn play(&mut self, slot: Slot, data: Arc<[u8]>) {
        self.stop(slot);
        // Nothing playing: a good moment to follow a changed default device, or to retry one that failed to open.
        if self.voices.is_empty() {
            let stale = match &self.main {
                None => true,
                Some(m) => self.want_main.is_none() && m.device != default_device_name(),
            };
            if stale || (self.want_monitor.is_some() && self.monitor.is_none()) {
                self.reopen();
            }
        }
        let volume = self.volume(slot);
        let mut sinks = Vec::new();
        let mut seconds = None;
        let mut error = None;
        for out in [&self.main, &self.monitor].into_iter().flatten() {
            let source = match Decoder::new(Cursor::new(data.clone())) {
                Ok(s) => s,
                Err(e) => {
                    error = Some(format!("เล่นไฟล์เสียงนี้ไม่ได้: {e}"));
                    break;
                }
            };
            seconds = seconds.or(source.total_duration().map(|d| d.as_secs_f32()));
            match Sink::try_new(&out.handle) {
                Ok(sink) => {
                    sink.set_volume(volume);
                    sink.append(source);
                    sinks.push(sink);
                }
                Err(e) => error = Some(format!("เล่นเสียงไม่ได้: {e}")),
            }
        }
        if sinks.is_empty() {
            (self.events)(Event::Error(error.unwrap_or_else(|| "ไม่มีอุปกรณ์เสียงออก".into())));
            return;
        }
        self.voices.insert(slot, sinks);
        (self.events)(Event::Started { slot, seconds });
    }

    fn reap(&mut self) {
        let done: Vec<Slot> =
            self.voices.iter().filter(|(_, sinks)| sinks.iter().all(Sink::empty)).map(|(s, _)| *s).collect();
        for slot in done {
            self.voices.remove(&slot);
            (self.events)(Event::Ended(slot));
        }
    }
}

/// How long a sound runs, in seconds, by decoding it to the end. The length a file declares is rounded to whole
/// seconds (or missing), too coarse for a progress bar on a one-second clip. Slow for long files: call it off the
/// window's thread.
pub fn measure(data: Arc<[u8]>) -> Option<f32> {
    let source = Decoder::new(Cursor::new(data)).ok()?;
    let per_second = source.channels() as f32 * source.sample_rate() as f32;
    let samples = source.count();
    (samples > 0 && per_second > 0.0).then(|| samples as f32 / per_second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_sound_decodes() {
        let dir = crate::settings::library_dir();
        let mut bad = Vec::new();
        for s in crate::library::load() {
            let data: Arc<[u8]> = std::fs::read(dir.join(&s.file)).unwrap().into();
            match Decoder::new(Cursor::new(data)) {
                Ok(d) => {
                    if d.take(1000).count() == 0 {
                        bad.push(format!("{}: silent", s.file));
                    }
                }
                Err(e) => bad.push(format!("{}: {e}", s.file)),
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn measures_a_short_clip() {
        let dir = crate::settings::library_dir();
        let data: Arc<[u8]> = std::fs::read(dir.join("2_vine-boom.mp3")).unwrap().into();
        let seconds = measure(data).unwrap();
        assert!((0.5..5.0).contains(&seconds), "{seconds}");
        assert!(measure(Arc::from(&b"not audio at all"[..])).is_none());
    }
}
