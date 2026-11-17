//! What the window shows: one panel, like the top of a sampler. Display, master knob and stop along the top;
//! the library on the left, the sixteen pads in the middle, the selected pad and the setup on the right.
use super::icons::{self, icon};
use super::style::{self, alpha, mix, PadLook};
use super::{App, Carried, DeviceChoice, Message, Modal, Target};
use crate::audio::Slot;
use crate::keys::Hotkey;
use crate::settings::PADS;
use crate::updater::Phase;
use iced::alignment::{Horizontal, Vertical};
use iced::font::{Family, Weight};
use iced::mouse::Interaction;
use iced::widget::text::{Shaping, Wrapping};
use iced::widget::{
    button, center, column, container, horizontal_space, lazy, mouse_area, opaque, pick_list, progress_bar, row,
    scrollable, slider, stack, text, text_input, toggler, Column, Row, Space, Text,
};
use iced::{Alignment, Color, Element, Fill, Font, Padding, Point, Size};

/// Names and sentences: IBM Plex Sans Thai. Labels, numbers and the display: Chakra Petch. Both ship inside the exe.
pub const FONT: Font = Font { family: Family::Name("IBM Plex Sans Thai"), ..Font::DEFAULT };
const MEDIUM: Font = Font { weight: Weight::Medium, ..FONT };
const SEMI: Font = Font { weight: Weight::Semibold, ..FONT };
const TECH: Font = Font { family: Family::Name("Chakra Petch"), weight: Weight::Semibold, ..Font::DEFAULT };
const TECH_BOLD: Font = Font { weight: Weight::Bold, ..TECH };

pub fn fonts() -> [&'static [u8]; 7] {
    [
        include_bytes!("../../assets/fonts/IBMPlexSansThai-Regular.ttf"),
        include_bytes!("../../assets/fonts/IBMPlexSansThai-Medium.ttf"),
        include_bytes!("../../assets/fonts/IBMPlexSansThai-SemiBold.ttf"),
        include_bytes!("../../assets/fonts/IBMPlexSansThai-Bold.ttf"),
        include_bytes!("../../assets/fonts/ChakraPetch-Medium.ttf"),
        include_bytes!("../../assets/fonts/ChakraPetch-SemiBold.ttf"),
        include_bytes!("../../assets/fonts/ChakraPetch-Bold.ttf"),
    ]
}

type El<'a> = Element<'a, Message>;

// The frame of the layout. `pad_at` works out from these where each pad is, so keep the view and it in step.
const GAP: f32 = 10.0;
const TOP: f32 = 80.0;
const LEFT: f32 = 268.0;
const RIGHT: f32 = 300.0;
const GRID_INSET: f32 = 0.0;
const PAD_GAP: f32 = 4.0;

/// The pad under a point of the window, if any. The gaps between pads count for the nearest pad, so a drop that
/// lands on a seam still goes somewhere.
pub fn pad_at(window: Size, p: Point) -> Option<usize> {
    let x0 = GAP + LEFT + GAP + GRID_INSET;
    let x1 = window.width - GAP - RIGHT - GAP - GRID_INSET;
    let y0 = GAP + TOP + GAP + GRID_INSET;
    let y1 = window.height - GAP - GRID_INSET;
    let half = PAD_GAP / 2.0;
    if x1 <= x0 || y1 <= y0 || p.x < x0 - half || p.x >= x1 + half || p.y < y0 - half || p.y >= y1 + half {
        return None;
    }
    let cell = |v: f32, from: f32, to: f32| (((v - from + half) / ((to - from + PAD_GAP) / 4.0)) as usize).min(3);
    Some(cell(p.y, y0, y1) * 4 + cell(p.x, x0, x1))
}

/// Text that shapes Thai properly.
fn t<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s).shaping(Shaping::Advanced).line_height(1.3)
}

/// A printed label: small capitals in the device font.
fn silk<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    t(s).font(TECH).size(11).color(style::LABEL)
}

fn hint<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    t(s).size(11.5).color(style::LABEL)
}

fn clock(seconds: f32) -> String {
    let tenths = (seconds * 10.0) as u32;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

fn line<'a>() -> El<'a> {
    container(Space::new(Fill, 1)).style(style::fill(alpha(Color::WHITE, 0.07))).into()
}

/// A key with a word (and maybe an icon) on it.
fn key<'a>(icon_data: Option<&'static [u8]>, word: &'a str, color: Color) -> El<'a> {
    let mut r = Row::new().spacing(6).align_y(Alignment::Center);
    if let Some(data) = icon_data {
        r = r.push(icon(data, 13.0, color));
    }
    container(r.push(t(word).font(TECH).size(12))).center_x(Fill).into()
}

impl App {
    pub fn view(&self) -> El<'_> {
        let body = row![self.library_panel(), self.grid(), self.side()].spacing(GAP).height(Fill);
        // No background here: the window's own colour is the chassis.
        let base: El = container(column![self.top(), body].spacing(GAP)).padding(GAP).width(Fill).height(Fill).into();

        let mut layers = stack![base];
        // What is being dragged follows the cursor.
        if let Some(drag) = self.drag.filter(|d| d.moving) {
            let (name, color) = match drag.item {
                Carried::Library(n) => (self.library.get(n).map_or("", |s| s.name.as_str()), style::AMBER),
                Carried::Pad(i) => {
                    let pad = &self.settings.pads[i];
                    (pad.sound.as_ref().map_or("", |s| s.name.as_str()), style::led(pad.color, i))
                }
            };
            let chip = container(
                row![container(Space::new(8, 8)).style(style::lamp(color, true)), t(name).font(SEMI).size(13).wrapping(Wrapping::None)]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::from([6, 12]))
            .style(style::ghost(color));
            layers = layers.push(
                container(chip)
                    .padding(Padding { left: (drag.at.x + 14.0).max(0.0), top: (drag.at.y + 12.0).max(0.0), ..Padding::ZERO })
                    .width(Fill)
                    .height(Fill)
                    .clip(true),
            );
        }
        if let Some(modal) = &self.modal {
            let dialog = mouse_area(container(self.dialog(modal)).style(style::dialog)).on_press(Message::Nothing);
            layers = layers
                .push(opaque(mouse_area(center(opaque(dialog)).padding(24).style(style::scrim)).on_press(Message::CloseModal)));
        }
        layers.into()
    }

    // ---- top: brand, display, master, stop --------------------------------------------------------------------

    fn top(&self) -> El<'_> {
        let brand = column![
            row![icon(icons::LOGO, 22.0, style::AMBER), t("KURO").font(TECH_BOLD).size(28).color(style::PRINT)]
                .spacing(8)
                .align_y(Alignment::Center),
            t("SOUND PAD").font(TECH).size(12).color(style::LABEL),
        ]
        .spacing(0)
        .width(LEFT - 32.0);

        let v = self.settings.master_volume;
        let master = row![
            mouse_area(iced::widget::svg(self.knob_picture.clone()).width(72).height(72))
            .on_press(Message::KnobDown)
            .on_scroll(Message::KnobWheel)
            .on_enter(Message::KnobHover(true))
            .on_exit(Message::KnobHover(false))
            .interaction(if self.knob.is_some() { Interaction::Grabbing } else { Interaction::Grab }),
            column![silk("MASTER"), t(format!("{v}")).font(TECH_BOLD).size(22).color(style::PRINT)].spacing(0).width(52),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let live = self.anything_playing();
        let stop_key = self.settings.stop_key.map_or(String::new(), |k| k.label());
        let stop = button(
            container(
                column![
                    row![icon(icons::STOP, 15.0, Color::WHITE), t("STOP").font(TECH_BOLD).size(20)].spacing(8).align_y(Alignment::Center),
                    t(stop_key).font(TECH).size(11).color(alpha(Color::WHITE, 0.75)),
                ]
                .align_x(Alignment::Center),
            )
            .center(Fill),
        )
        .width(132)
        .height(66)
        .padding(0)
        .on_press(Message::StopAll)
        .style(style::stop(live));

        // Straight on the chassis (see `style::CHASSIS`): the display changes many times a second.
        container(row![brand, self.display(), master, stop].spacing(18).align_y(Alignment::Center))
            .padding(Padding::from([0, 12]))
            .height(TOP)
            .width(Fill)
            .align_y(Vertical::Center)
            .into()
    }

    fn display(&self) -> El<'_> {
        // The display follows the pad started last while it plays, else any other playing pad, else a preview.
        let pads: Vec<usize> = (0..PADS).filter(|i| self.is_playing(Slot::Pad(*i))).collect();
        let shown = self.latest.filter(|i| pads.contains(i)).or(pads.first().copied());
        let (title, slot): (String, Option<Slot>) = match (shown, self.preview) {
            (Some(i), _) => {
                let name = self.settings.pads[i].sound.as_ref().map_or("", |s| s.name.as_str());
                let more = if pads.len() > 1 { format!("  +{}", pads.len() - 1) } else { String::new() };
                (format!("{name}{more}"), Some(Slot::Pad(i)))
            }
            (None, Some(n)) if self.is_playing(Slot::Preview) => {
                (self.library.get(n).map_or(String::new(), |s| s.name.clone()), Some(Slot::Preview))
            }
            _ => ("READY".to_string(), None),
        };
        let time = match slot.and_then(|s| self.clock(s)) {
            Some((at, Some(total))) => format!("{} / {}", clock(at), clock(total)),
            Some((at, None)) => clock(at),
            None => "-:--.-".to_string(),
        };

        let filled = self.settings.pads.iter().filter(|p| p.sound.is_some()).count();
        let (status, status_color): (String, Color) = if let Some(n) = &self.notice {
            (n.text.clone(), if n.error { style::RED } else { style::AMBER })
        } else if self.capture.is_some() {
            ("กดปุ่มที่ต้องการบนคีย์บอร์ด · Esc ยกเลิก".into(), style::AMBER)
        } else if self.drag.is_some_and(|d| d.moving) {
            ("ปล่อยบนแผงที่ต้องการ".into(), style::AMBER)
        } else if matches!(slot, Some(Slot::Preview)) {
            ("ฟังตัวอย่างจากคลังเสียง".into(), style::AMBER_DIM)
        } else {
            let hotkeys = if self.settings.global_hotkeys == Some(true) { "ปุ่มลัดทั่วระบบ" } else { "ปุ่มลัดเฉพาะในแอป" };
            (format!("{filled}/{PADS} แผง · {hotkeys}"), style::AMBER_DIM)
        };

        let meter = |label: &'static str, level: f32| {
            // 42 dB of range across the bar.
            let lit = if level <= 0.0 { 0.0 } else { ((20.0 * level.log10() + 42.0) / 42.0).clamp(0.0, 1.0) * 24.0 };
            let mut bar = Row::new().spacing(2).align_y(Alignment::Center);
            bar = bar.push(t(label).font(TECH).size(10).color(style::AMBER_DIM).width(11));
            for i in 0..24 {
                let c = if i >= 21 { style::RED } else if i >= 16 { style::AMBER } else { style::GREEN };
                let c = if (i as f32) < lit { c } else { mix(c, Color::BLACK, 0.84) };
                bar = bar.push(container(Space::new(5, 8)).style(style::dot(c, 1.0)));
            }
            bar
        };

        container(
            row![
                column![
                    t(title).font(TECH_BOLD).size(24).color(if slot.is_some() { style::AMBER } else { style::AMBER_DIM }).wrapping(Wrapping::None),
                    t(status).font(MEDIUM).size(12.5).color(status_color).wrapping(Wrapping::None),
                ]
                .spacing(1)
                .width(Fill),
                column![
                    t(time).font(TECH).size(17).color(if slot.is_some() { style::AMBER } else { style::AMBER_DIM }),
                    meter("L", self.meter[0]),
                    meter("R", self.meter[1]),
                ]
                .spacing(4)
                .align_x(Alignment::End),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([6, 16]))
        .height(72)
        .width(Fill)
        .align_y(Vertical::Center)
        .style(style::lcd)
        .into()
    }

    // ---- left: the library ------------------------------------------------------------------------------------

    fn library_panel(&self) -> El<'_> {
        // The list is 288 rows. It is built again only when something it shows changes, not on every tick of a
        // playing sound.
        let carried = match self.drag {
            Some(d) if d.moving => match d.item {
                Carried::Library(n) => Some(n),
                Carried::Pad(_) => None,
            },
            _ => None,
        };
        let shown = (self.query.trim().to_lowercase(), self.preview, self.hover_row, carried, self.library.len());
        let library = &self.library;
        let list = lazy(shown, move |(query, preview, hover, carried, _)| -> Element<'static, Message> {
            let mut list = Column::new().spacing(1);
            let mut found = 0;
            for (n, s) in library.iter().enumerate() {
                if !(query.is_empty() || s.key.contains(query.as_str()) || s.id.to_string() == *query) {
                    continue;
                }
                found += 1;
                let playing = *preview == Some(n);
                let lead: Element<'static, Message> = if playing {
                    container(icon(icons::STOP, 10.0, style::AMBER)).width(30).into()
                } else {
                    t(format!("{:03}", s.id)).font(TECH).size(11).color(style::FAINT).width(30).into()
                };
                let name_color = if *carried == Some(n) { style::FAINT } else if playing { style::AMBER } else { style::PRINT };
                let mut item = row![lead, t(s.name.clone()).size(13).color(name_color).width(Fill).wrapping(Wrapping::None)]
                    .spacing(4)
                    .height(24)
                    .align_y(Alignment::Center);
                // Only the row under the cursor carries the + button: 288 of them would be 288 more things to lay
                // out and look at on every mouse move.
                if *hover == Some(n) {
                    item = item.push(button(icon(icons::PLUS, 11.0, style::PRINT)).padding(5).on_press(Message::Add(n)).style(style::flat()));
                }
                list = list.push(
                    mouse_area(
                        container(item)
                            .padding(Padding { top: 0.0, right: 2.0, bottom: 0.0, left: 8.0 })
                            .width(Fill)
                            .style(style::row(playing, *hover == Some(n))),
                    )
                    .on_press(Message::RowDown(n))
                    .on_enter(Message::RowHover(n, true))
                    .on_exit(Message::RowHover(n, false))
                    .interaction(Interaction::Grab),
                );
            }
            if found == 0 {
                let why = if library.is_empty() { "เปิดคลังเสียงไม่ได้ (library.kpk ข้างไฟล์โปรแกรม)" } else { "ไม่พบเสียงที่ค้นหา" };
                container(hint(why)).padding(12).center_x(Fill).into()
            } else {
                container(list).padding(Padding { right: 10.0, ..Padding::ZERO }).into()
            }
        });

        container(
            column![
                row![silk("LIBRARY"), horizontal_space(), t(format!("{}", self.library.len())).font(TECH).size(11).color(style::FAINT)]
                    .align_y(Alignment::Center),
                text_input("ค้นหาเสียง…", &self.query)
                    .on_input(Message::Query)
                    .on_submit(Message::Unfocus)
                    .padding(Padding::from([6, 10]))
                    .size(13)
                    .font(FONT)
                    .style(style::input),
                container(scrollable(list).height(Fill).style(style::scroll)).padding(4).height(Fill).width(Fill).style(style::well),
                hint("กดเพื่อฟัง · ลากไปวางบนแผง · + ลงแผงว่าง"),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(LEFT)
        .height(Fill)
        .style(style::panel)
        .into()
    }

    // ---- middle: the pads -------------------------------------------------------------------------------------

    fn grid(&self) -> El<'_> {
        let mut rows = Column::new().spacing(PAD_GAP);
        for r in 0..4 {
            let mut cells = Row::new().spacing(PAD_GAP);
            for c in 0..4 {
                cells = cells.push(self.pad(r * 4 + c));
            }
            rows = rows.push(cells.height(Fill));
        }
        // Straight on the chassis too: a playing pad's strip moves.
        container(rows).padding(GRID_INSET).width(Fill).height(Fill).into()
    }

    fn pad(&self, i: usize) -> El<'_> {
        let pad = &self.settings.pads[i];
        let color = style::led(pad.color, i);
        let progress = self.progress(Slot::Pad(i));
        let moving = self.drag.filter(|d| d.moving);
        let look = PadLook {
            filled: pad.sound.is_some(),
            playing: progress.is_some(),
            selected: self.selected == Some(i),
            hover: self.hover == Some(i) && moving.is_none(),
            pressed: self.drag.is_some_and(|d| !d.moving && d.item == Carried::Pad(i)),
            target: self.drop_target() == Some(i),
            lifted: moving.is_some_and(|d| d.item == Carried::Pad(i)),
            broken: self.broken[i],
        };
        let capturing = self.capture == Some(Target::Pad(i));
        let cap: El = match (capturing, pad.key) {
            (true, _) => container(t("…").font(TECH_BOLD).size(12)).padding(Padding::from([1, 7])).style(style::keycap(true)).into(),
            (false, Some(k)) => container(t(k.label()).font(TECH_BOLD).size(12)).padding(Padding::from([1, 7])).style(style::keycap(false)).into(),
            (false, None) => Space::new(0, 0).into(),
        };
        let number = t(format!("{:02}", i + 1)).font(TECH_BOLD).size(14).color(if look.filled { alpha(color, 0.95) } else { style::FAINT });
        let ink = if look.lifted { alpha(style::PRINT, 0.3) } else { style::PRINT };

        let content: El = match &pad.sound {
            None => column![
                row![number, horizontal_space(), cap].align_y(Alignment::Center),
                container(
                    column![icon(icons::PLUS, 22.0, if look.target || look.hover { style::LABEL } else { style::FAINT }), hint("วางเสียง").color(style::FAINT)]
                        .spacing(4)
                        .align_x(Alignment::Center)
                )
                .center(Fill),
                Space::new(Fill, 5),
            ]
            .spacing(4)
            .into(),
            Some(sound) => {
                let foot: El = if look.broken {
                    row![icon(icons::WARNING, 12.0, style::RED), t("ไฟล์หาย").size(11).color(style::RED)].spacing(4).into()
                } else {
                    let length = self.seconds[i].map_or(String::new(), clock);
                    let volume = if pad.volume == 100 { String::new() } else { format!("{}%", pad.volume) };
                    row![
                        t(length).font(TECH).size(11).color(style::LABEL),
                        horizontal_space(),
                        t(volume).font(TECH).size(11).color(style::LABEL)
                    ]
                    .into()
                };
                let strip: El = match progress {
                    Some(f) => progress_bar(0.0..=1.0, f).height(5).style(style::strip(Color::WHITE, alpha(color, 0.45))).into(),
                    None => container(Space::new(Fill, 5))
                        .style(style::dot(if look.broken || look.lifted { alpha(color, 0.25) } else { alpha(color, 0.9) }, 3.0))
                        .into(),
                };
                column![
                    row![number, horizontal_space(), cap].align_y(Alignment::Center),
                    container(t(sound.name.as_str()).font(SEMI).size(15).color(ink).align_x(Horizontal::Center).wrapping(Wrapping::WordOrGlyph))
                        .center(Fill)
                        .clip(true),
                    foot,
                    strip,
                ]
                .spacing(4)
                .into()
            }
        };
        // The halo stands in for a glow: lit in the pad's colour while it plays, white where a drop would land.
        let (ring, strength) = if look.target { (style::PRINT, 0.3) } else if look.playing { (color, 0.38) } else { (color, 0.0) };
        let body = container(content).padding(10).width(Fill).height(Fill).style(style::pad(color, look));
        mouse_area(container(body).padding(3).width(Fill).height(Fill).style(style::halo(ring, strength)))
            .on_press(Message::PadDown(i))
            .on_right_press(Message::PadSelect(i))
            .on_enter(Message::PadHover(i, true))
            .on_exit(Message::PadHover(i, false))
            .interaction(Interaction::Pointer)
            .into()
    }

    // ---- right: the selected pad, and the setup ---------------------------------------------------------------

    fn side(&self) -> El<'_> {
        column![self.pad_panel(), self.setup_panel()].spacing(GAP).width(RIGHT).height(Fill).into()
    }

    /// A key that takes a hotkey: click it, then press the key.
    fn key_field(&self, target: Target, key: Option<Hotkey>) -> El<'_> {
        let capturing = self.capture == Some(target);
        let word = match (capturing, key) {
            (true, _) => "กดปุ่ม…".to_string(),
            (false, Some(k)) => k.label(),
            (false, None) => "ไม่ได้ตั้ง".to_string(),
        };
        let mut r = row![button(container(t(word).font(TECH_BOLD).size(13).color(if key.is_some() || capturing { style::PRINT } else { style::FAINT })).center_x(Fill))
            .width(Fill)
            .padding(Padding::from([6, 8]))
            .on_press(Message::Capture(target))
            .style(if capturing { style::cap_lit() } else { style::cap() })]
        .spacing(4)
        .align_y(Alignment::Center);
        if key.is_some() && !capturing {
            r = r.push(button(icon(icons::CLOSE, 12.0, style::LABEL)).padding(7).on_press(Message::ClearKey(target)).style(style::flat()));
        }
        r.into()
    }

    fn pad_panel(&self) -> El<'_> {
        let content: El = match self.selected {
            None => column![
                silk("PAD"),
                container(
                    column![
                        t("ยังไม่ได้เลือกแผง").font(SEMI).size(14).color(style::PRINT),
                        hint("กดที่แผงเพื่อเล่นและเลือก"),
                        hint("คลิกขวาเพื่อเลือกโดยไม่เล่น"),
                        hint("ลากแผงไปวางบนอีกแผงเพื่อสลับที่"),
                    ]
                    .spacing(4)
                )
                .padding(Padding::from([8, 0])),
            ]
            .spacing(6)
            .into(),
            Some(i) => {
                let pad = &self.settings.pads[i];
                let color = style::led(pad.color, i);
                let head = row![
                    container(Space::new(10, 10)).style(style::lamp(color, self.is_playing(Slot::Pad(i)))),
                    t(format!("PAD {:02}", i + 1)).font(TECH_BOLD).size(16).color(style::PRINT),
                    horizontal_space(),
                    t(self.seconds[i].map_or(String::new(), clock)).font(TECH).size(12).color(style::LABEL),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                let hotkey = row![silk("KEY").width(40), self.key_field(Target::Pad(i), pad.key)].spacing(8).align_y(Alignment::Center);

                match &pad.sound {
                    None => column![
                        head,
                        hint("แผงนี้ว่าง: ลากเสียงจากคลังหรือไฟล์จากเครื่องมาวางบนแผงได้เลย"),
                        button(key(Some(icons::FILE), "เลือกไฟล์เสียง", style::PRINT)).width(Fill).padding(8).on_press(Message::PickFile(i)).style(style::cap()),
                        hotkey,
                    ]
                    .spacing(10)
                    .into(),
                    Some(sound) => {
                        let chosen = pad.color.map_or([0, 1, 5, 3][(i / 4) % 4], |c| c as usize);
                        let mut colors = Row::new().align_y(Alignment::Center);
                        for (n, c) in style::LEDS.into_iter().enumerate() {
                            if n > 0 {
                                colors = colors.push(horizontal_space());
                            }
                            colors = colors.push(
                                button(Space::new(16, 16)).padding(0).on_press(Message::PadColor(i, n as u8)).style(style::swatch(c, n == chosen)),
                            );
                        }
                        column![
                            head,
                            container(colors).padding(Padding::from([4, 4])),
                            text_input("ชื่อเสียง", &sound.name)
                                .on_input(move |s| Message::PadName(i, s))
                                .on_submit(Message::Unfocus)
                                .padding(Padding::from([7, 10]))
                                .size(14)
                                .font(MEDIUM)
                                .style(style::input),
                            row![
                                silk("VOL").width(40),
                                slider(0..=100u8, pad.volume, move |v| Message::PadVolume(i, v)).step(1u8).on_release(Message::Save).height(22).style(style::fader(color)),
                                t(format!("{}", pad.volume)).font(TECH_BOLD).size(13).color(style::PRINT).width(28).align_x(Horizontal::Right),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            hotkey,
                            row![
                                button(key(Some(icons::FILE), "เปลี่ยนไฟล์", style::PRINT)).width(Fill).padding(7).on_press(Message::PickFile(i)).style(style::cap()),
                                button(key(Some(icons::TRASH), "เอาออก", style::rgb(0xFF8A80))).width(Fill).padding(7).on_press(Message::ClearPad(i)).style(style::cap_danger()),
                            ]
                            .spacing(8),
                        ]
                        .spacing(10)
                        .into()
                    }
                }
            }
        };
        container(content).padding(12).width(Fill).style(style::panel).into()
    }

    fn setup_panel(&self) -> El<'_> {
        let s = &self.settings;
        let mut outputs = vec![DeviceChoice { name: None, label: "ค่าเริ่มต้นของระบบ".into() }];
        let mut monitors = vec![DeviceChoice { name: None, label: "ปิด".into() }];
        for d in &self.devices {
            let c = DeviceChoice { name: Some(d.clone()), label: d.clone() };
            outputs.push(c.clone());
            monitors.push(c);
        }
        // A saved device that is unplugged right now still shows as the choice.
        let current = |name: &Option<String>, list: &mut Vec<DeviceChoice>| {
            let found = list.iter().find(|c| &c.name == name).cloned();
            found.unwrap_or_else(|| {
                let c = DeviceChoice { name: name.clone(), label: format!("{} (ไม่ได้เชื่อมต่อ)", name.clone().unwrap_or_default()) };
                list.push(c.clone());
                c
            })
        };
        let out_now = current(&s.output_device, &mut outputs);
        let mon_now = current(&s.monitor_device, &mut monitors);
        let picker = |options: Vec<DeviceChoice>, now: DeviceChoice, on: fn(DeviceChoice) -> Message| {
            pick_list(options, Some(now), on)
                .width(Fill)
                .padding(Padding::from([6, 10]))
                .text_size(12.5)
                .text_shaping(Shaping::Advanced)
                .font(FONT)
                .style(style::pick)
                .menu_style(style::menu)
        };
        let global = s.global_hotkeys == Some(true);

        let content = column![
            row![
                silk("OUTPUT"),
                horizontal_space(),
                button(icon(icons::REFRESH, 12.0, style::LABEL)).padding(4).on_press(Message::RefreshDevices).style(style::flat()),
            ]
            .align_y(Alignment::Center),
            picker(outputs, out_now, Message::Output),
            silk("MONITOR · ส่งออกอีกเครื่อง (เช่น VB-Cable เข้าไมค์)"),
            picker(monitors, mon_now, Message::Monitor),
            line(),
            row![
                container(Space::new(9, 9)).style(style::lamp(style::GREEN, global)),
                silk("GLOBAL HOTKEY"),
                horizontal_space(),
                toggler(global).on_toggle(Message::GlobalHotkeys).size(20).style(style::switch),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            hint(if global { "ปุ่มลัดทำงานแม้อยู่ในเกมหรือแอปอื่น ไม่บันทึกการพิมพ์" } else { "ปุ่มลัดทำงานเฉพาะตอนหน้าต่างนี้อยู่ด้านหน้า" }),
            row![silk("STOP KEY").width(64), self.key_field(Target::Stop, s.stop_key)].spacing(8).align_y(Alignment::Center),
            line(),
            row![
                button(key(Some(icons::IMPORT), "นำเข้า", style::PRINT)).width(Fill).padding(7).on_press(Message::Import).style(style::cap()),
                button(key(Some(icons::FILE), "ส่งออก", style::PRINT)).width(Fill).padding(7).on_press(Message::Export).style(style::cap()),
            ]
            .spacing(8),
            row![
                button(key(Some(icons::RESET), "คืนปุ่มลัด", style::PRINT)).width(Fill).padding(7).on_press(Message::ResetKeys).style(style::cap()),
                button(key(Some(icons::TRASH), "ล้างทั้งหมด", style::rgb(0xFF8A80))).width(Fill).padding(7).on_press(Message::Open(Modal::ConfirmClear)).style(style::cap_danger()),
            ]
            .spacing(8),
        ]
        .spacing(8);

        // The version: on an installed copy it is also the button that looks for a newer one.
        let version = concat!("v", env!("CARGO_PKG_VERSION"));
        let version: El = if !self.updater.supported() {
            hint(version).color(style::FAINT).into()
        } else if self.updater.phase == Phase::Checking {
            hint("กำลังตรวจหารุ่นใหม่…").color(style::FAINT).into()
        } else {
            button(row![hint(version), icon(icons::REFRESH, 11.0, style::LABEL)].spacing(5).align_y(Alignment::Center))
                .padding(Padding::from([3, 6]))
                .on_press(Message::UpdateCheck(true))
                .style(style::flat())
                .into()
        };
        let mut parts = column![scrollable(container(content).padding(Padding { right: 10.0, ..Padding::ZERO })).height(Fill).style(style::scroll)]
            .spacing(6);
        if let Some(update) = self.update_block() {
            parts = parts.push(container(update).padding(Padding { right: 6.0, ..Padding::ZERO }));
        }
        container(
            parts.push(
                row![
                    version,
                    horizontal_space(),
                    button(row![icon(icons::HELP, 12.0, style::LABEL), hint("วิธีใช้")].spacing(5).align_y(Alignment::Center))
                        .padding(Padding::from([3, 6]))
                        .on_press(Message::Open(Modal::Help))
                        .style(style::flat()),
                ]
                .align_y(Alignment::Center),
            ),
        )
        .padding(Padding { top: 12.0, right: 6.0, bottom: 8.0, left: 12.0 })
        .width(Fill)
        .height(Fill)
        .style(style::panel)
        .into()
    }

    /// A newer version on offer, or how its download is going. Nothing while there is nothing to say.
    fn update_block(&self) -> Option<El<'_>> {
        let up = &self.updater;
        let version = up.release.as_ref().map_or("", |r| r.version.as_str());
        let body: El = match up.phase {
            Phase::Available if self.settings.skip_version.as_deref() != Some(version) => column![
                row![
                    container(Space::new(9, 9)).style(style::lamp(style::AMBER, true)),
                    t(format!("มีรุ่นใหม่ {version}")).font(SEMI).size(13.5).color(style::PRINT),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                row![
                    button(key(None, "ข้ามรุ่นนี้", style::PRINT)).width(Fill).padding(6).on_press(Message::UpdateSkip).style(style::cap()),
                    button(key(Some(icons::IMPORT), "อัปเดต", style::rgb(0x1A1000))).width(Fill).padding(6).on_press(Message::UpdateNow).style(style::cap_go()),
                ]
                .spacing(8),
            ]
            .spacing(8)
            .into(),
            Phase::Downloading => column![
                row![
                    t(format!("กำลังดาวน์โหลด {version}")).size(12.5).color(style::PRINT),
                    horizontal_space(),
                    t(format!("{}%", up.percent)).font(TECH).size(12).color(style::AMBER),
                ]
                .align_y(Alignment::Center),
                progress_bar(0.0..=100.0, up.percent as f32).height(5).style(style::strip(style::AMBER, style::rgb(0x2A2418))),
            ]
            .spacing(7)
            .into(),
            Phase::Installing => t("กำลังติดตั้ง แอปจะเปิดขึ้นมาใหม่เอง").size(12.5).color(style::AMBER).into(),
            Phase::Failed => column![
                t(format!("อัปเดตไม่สำเร็จ: {}", up.error.as_deref().unwrap_or_default())).size(12).color(style::RED),
                button(key(Some(icons::REFRESH), "ลองอีกครั้ง", style::PRINT)).width(Fill).padding(6).on_press(Message::UpdateNow).style(style::cap()),
            ]
            .spacing(8)
            .into(),
            _ => return None,
        };
        Some(container(body).padding(10).width(Fill).style(style::well).into())
    }

    // ---- the few dialogs left ---------------------------------------------------------------------------------

    fn dialog<'a>(&'a self, modal: &'a Modal) -> El<'a> {
        let title = |s: &'a str| t(s).font(SEMI).size(19).color(style::PRINT);
        let body = |s: &'a str| t(s).size(13.5).color(style::LABEL);
        let cancel = |word: &'a str| button(key(None, word, style::PRINT)).width(110).padding(8).on_press(Message::CloseModal).style(style::cap());

        match modal {
            Modal::Import(imported) => {
                let mut list = Column::new().spacing(2);
                for (i, pad) in imported.pads.iter().enumerate() {
                    let name = pad.sound.as_ref().map_or("ว่าง", |(n, _)| n.as_str());
                    let mut r = row![
                        t(format!("{:02}", i + 1)).font(TECH).size(12).color(style::FAINT).width(26),
                        t(name).size(13).color(if pad.sound.is_some() { style::PRINT } else { style::FAINT }).width(Fill).wrapping(Wrapping::None),
                    ]
                    .spacing(6);
                    if pad.volume != 100 {
                        r = r.push(t(format!("{}%", pad.volume)).font(TECH).size(12).color(style::AMBER));
                    }
                    if let Some(k) = pad.key {
                        r = r.push(t(k.label()).font(TECH).size(12).color(style::LABEL).width(86).align_x(Horizontal::Right));
                    }
                    list = list.push(r);
                }
                let mut summary = format!(
                    "เสียง {}/{PADS} แผง · ปุ่มลัด {}/{PADS} · ระดับเสียงหลัก {}% · ปรับเสียงรายแผง {} แผง",
                    imported.sound_count(),
                    imported.key_count(),
                    imported.master_volume,
                    imported.custom_volume_count()
                );
                if imported.skipped > 0 {
                    summary.push_str(&format!(" · อ่านไม่ได้ {} เสียง (จะถูกข้าม)", imported.skipped));
                }
                container(
                    column![
                        title("นำเข้าการตั้งค่า"),
                        body("เสียง ระดับเสียง และปุ่มลัดในไฟล์นี้จะแทนที่ของปัจจุบันทั้งหมด"),
                        t(summary).size(13).color(style::AMBER),
                        container(scrollable(container(list).padding(Padding { right: 12.0, ..Padding::ZERO })).height(230).style(style::scroll))
                            .padding(10)
                            .width(Fill)
                            .style(style::well),
                        row![
                            horizontal_space(),
                            cancel("ยกเลิก"),
                            button(key(None, "นำเข้า", style::PRINT)).width(110).padding(8).on_press(Message::ImportConfirm).style(style::cap_go()),
                        ]
                        .spacing(8),
                    ]
                    .spacing(12),
                )
                .padding(20)
                .width(520)
                .into()
            }

            Modal::ConfirmClear => container(
                column![
                    title("ล้างแผงเสียงทั้งหมด?"),
                    body("เสียง สี และระดับเสียงของทุกแผงจะถูกลบ ย้อนกลับไม่ได้ ปุ่มลัดและระดับเสียงหลักยังอยู่เหมือนเดิม"),
                    row![
                        horizontal_space(),
                        cancel("ยกเลิก"),
                        button(key(None, "ล้างทั้งหมด", style::rgb(0xFF8A80))).width(120).padding(8).on_press(Message::ClearAll).style(style::cap_danger()),
                    ]
                    .spacing(8),
                ]
                .spacing(14),
            )
            .padding(20)
            .width(430)
            .into(),

            Modal::Welcome | Modal::Help => {
                let welcome = matches!(modal, Modal::Welcome);
                let step = |n: u32, head: &'a str, what: &'a str| {
                    row![
                        container(t(n.to_string()).font(TECH_BOLD).size(14).color(style::rgb(0x1A1000)))
                            .width(26)
                            .height(26)
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .style(style::dot(style::AMBER, 13.0)),
                        column![t(head).font(SEMI).size(14.5).color(style::PRINT), body(what).size(12.5)].spacing(1).width(Fill),
                    ]
                    .spacing(12)
                };
                let steps = column![
                    step(1, "ใส่เสียงลงแผง", "ลากเสียงจากคลังด้านซ้าย หรือลากไฟล์จากเครื่อง มาวางบนแผงที่ต้องการ กดที่แผงว่างเพื่อเลือกไฟล์ก็ได้"),
                    step(2, "กดเล่น", "กดที่แผงหรือกดปุ่มลัดบนคีย์บอร์ด กดซ้ำเพื่อเริ่มใหม่ เล่นหลายแผงซ้อนกันได้ ปุ่ม STOP หยุดทุกเสียง"),
                    step(3, "ปรับแต่งแผง", "แผงที่กดล่าสุดจะขึ้นที่มุมขวาบน เปลี่ยนชื่อ สีไฟ ระดับเสียง และปุ่มลัดได้ตรงนั้น คลิกขวาเพื่อเลือกโดยไม่เล่น"),
                    step(4, "จัดเรียง", "ลากแผงไปวางบนอีกแผงเพื่อสลับที่ ปุ่มลัดอยู่กับตำแหน่งเดิม"),
                ]
                .spacing(12);
                let foot: El = if welcome {
                    column![
                        container(
                            column![
                                t("ให้ปุ่มลัดทำงานนอกแอปด้วยไหม?").font(SEMI).size(14.5).color(style::PRINT),
                                body("ถ้าอนุญาต แอปจะคอยดูการกดแป้นพิมพ์ทั่วระบบ เพื่อให้กดปุ่มลัดเล่นเสียงได้ตอนอยู่ในเกมหรือแอปอื่น").size(12.5),
                                body("แอปเทียบเฉพาะปุ่มลัดที่ตั้งไว้ ไม่บันทึก ไม่เก็บ และไม่ส่งการกดปุ่มไปที่ไหน เปลี่ยนได้ทุกเมื่อที่สวิตช์ GLOBAL HOTKEY").size(12.5),
                            ]
                            .spacing(5)
                        )
                        .padding(12)
                        .width(Fill)
                        .style(style::well),
                        row![
                            horizontal_space(),
                            button(key(None, "ใช้เฉพาะในแอป", style::PRINT)).width(150).padding(9).on_press(Message::Welcome(false)).style(style::cap()),
                            button(key(None, "อนุญาต", style::PRINT)).width(130).padding(9).on_press(Message::Welcome(true)).style(style::cap_go()),
                        ]
                        .spacing(8),
                    ]
                    .spacing(12)
                    .into()
                } else {
                    row![horizontal_space(), button(key(None, "เข้าใจแล้ว", style::PRINT)).width(120).padding(8).on_press(Message::CloseModal).style(style::cap_go())].into()
                };
                container(
                    column![
                        row![icon(icons::LOGO, 22.0, style::AMBER), title(if welcome { "ยินดีต้อนรับสู่ Kuro Sound Pad" } else { "วิธีใช้" })]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        steps,
                        foot,
                    ]
                    .spacing(16),
                )
                .padding(22)
                .width(560)
                .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_are_found_where_they_are_drawn() {
        let w = Size::new(1280.0, 800.0);
        let (x0, x1) = (GAP + LEFT + GAP + GRID_INSET, w.width - GAP - RIGHT - GAP - GRID_INSET);
        let (y0, y1) = (GAP + TOP + GAP + GRID_INSET, w.height - GAP - GRID_INSET);
        let (pw, ph) = ((x1 - x0 - 3.0 * PAD_GAP) / 4.0, (y1 - y0 - 3.0 * PAD_GAP) / 4.0);
        for r in 0..4 {
            for c in 0..4 {
                let (px, py) = (x0 + c as f32 * (pw + PAD_GAP), y0 + r as f32 * (ph + PAD_GAP));
                for (dx, dy) in [(1.0, 1.0), (pw / 2.0, ph / 2.0), (pw - 1.0, ph - 1.0)] {
                    assert_eq!(pad_at(w, Point::new(px + dx, py + dy)), Some(r * 4 + c));
                }
            }
        }
        // The library, the side panels and the top are not pads.
        assert_eq!(pad_at(w, Point::new(100.0, 400.0)), None);
        assert_eq!(pad_at(w, Point::new(1200.0, 400.0)), None);
        assert_eq!(pad_at(w, Point::new(640.0, 50.0)), None);
        assert_eq!(pad_at(Size::new(10.0, 10.0), Point::new(5.0, 5.0)), None);
    }

    #[test]
    fn clock_format() {
        assert_eq!(clock(0.0), "0:00.0");
        assert_eq!(clock(1.34), "0:01.3");
        assert_eq!(clock(75.96), "1:15.9");
    }
}
