//! What the window shows.
use super::icons::{self, icon};
use super::style::{alpha, mix, Palette, PALETTES};
use super::{App, DeviceChoice, Message, Modal, Target};
use crate::audio::Slot;
use crate::keys::Hotkey;
use crate::settings::PADS;
use iced::alignment::{Horizontal, Vertical};
use iced::font::{Family, Weight};
use iced::widget::text::{Shaping, Wrapping};
use iced::widget::{
    button, center, column, container, horizontal_space, mouse_area, opaque, pick_list, progress_bar, row, scrollable,
    slider, stack, text, text_input, toggler, vertical_space, Column, Row, Space, Text,
};
use iced::{Alignment, Color, Element, Fill, Font, Length, Padding};

pub const FONT: Font = Font { family: Family::Name("Leelawadee UI"), ..Font::DEFAULT };
const BOLD: Font = Font { weight: Weight::Bold, ..FONT };

type El<'a> = Element<'a, Message>;

pub fn search_id() -> text_input::Id {
    text_input::Id::new("library-search")
}

pub fn rename_id() -> text_input::Id {
    text_input::Id::new("rename")
}

/// Text that shapes Thai properly.
fn t<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s).shaping(Shaping::Advanced).line_height(1.35)
}

fn bold<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    t(s).font(BOLD)
}

fn label<'a>(s: impl text::IntoFragment<'a>, p: Palette) -> Text<'a> {
    bold(s).size(14).color(p.text)
}

fn hint<'a>(s: impl text::IntoFragment<'a>, p: Palette) -> Text<'a> {
    t(s).size(12.5).color(p.dim)
}

fn keycap<'a>(key: Option<Hotkey>, p: Palette, on_pad: bool, active: bool) -> El<'a> {
    let s = match (active, key) {
        (true, _) => "กดปุ่ม…".to_string(),
        (false, Some(k)) => k.label(),
        (false, None) => "–".to_string(),
    };
    container(bold(s).size(13))
        .padding(Padding::from([2, 8]))
        .style(p.keycap(on_pad, active))
        .into()
}

/// A button with an icon and a word.
fn action<'a>(icon_data: &'static [u8], word: &'a str, color: Color) -> Row<'a, Message> {
    row![icon(icon_data, 17.0, color), bold(word).size(14)].spacing(8).align_y(Alignment::Center)
}

impl App {
    pub fn view(&self) -> El<'_> {
        let p = self.palette();
        let body = row![
            container(self.grid()).padding(18).width(Fill).height(Fill),
            container(Space::new(1, Fill)).style(p.divider()),
            container(self.sidebar()).width(312).height(Fill).style(p.bar()),
        ];
        let base: El = container(column![self.top_bar(), container(Space::new(Fill, 1)).style(p.divider()), body])
            .width(Fill)
            .height(Fill)
            .style(p.fill(p.bg))
            .into();

        let mut layers = stack![base];
        if let Some(modal) = &self.modal {
            let dialog = self.dialog(modal);
            // A click on the dialog stops there; one on the dimmed backdrop closes it.
            let dialog = mouse_area(container(dialog).style(p.dialog())).on_press(Message::Nothing);
            layers = layers.push(opaque(
                mouse_area(center(opaque(dialog)).padding(24).style(p.scrim())).on_press(Message::CloseModal),
            ));
        }
        if let Some(toast) = &self.toast {
            layers = layers.push(
                container(container(bold(toast.text.as_str()).size(14)).padding(Padding::from([10, 20])).style(p.toast(toast.error)))
                    .width(Fill)
                    .height(Fill)
                    .padding(26)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Bottom),
            );
        }
        layers.into()
    }

    fn top_bar(&self) -> El<'_> {
        let p = self.palette();
        let s = &self.settings;
        let stop = button(
            row![
                icon(icons::STOP, 16.0, p.on_danger),
                bold("หยุดทั้งหมด").size(15),
                keycap(s.stop_key, p, false, false),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([8, 14]))
        .on_press(Message::StopAll)
        .style(p.danger_button());

        let edit = row![
            toggler(s.edit_mode).on_toggle(Message::EditMode).size(24).style(p.toggle()),
            bold("แก้ไข").size(15).color(if s.edit_mode { p.text } else { p.dim }),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let bar = row![
            icon(icons::LOGO, 30.0, p.accent),
            column![
                bold("Kuro Sound Pad").size(20).color(p.text),
            ],
            hint(concat!("v", env!("CARGO_PKG_VERSION")), p),
            horizontal_space(),
            stop,
            container(Space::new(1, 26)).style(p.divider()),
            edit,
            container(Space::new(1, 26)).style(p.divider()),
            button(action(icons::KEYBOARD, "ปุ่มลัด", p.text))
                .padding(Padding::from([9, 14]))
                .on_press(Message::Open(Modal::Shortcuts))
                .style(p.secondary()),
            button(action(icons::LIBRARY, "คลังเสียง", p.text))
                .padding(Padding::from([9, 14]))
                .on_press(Message::Open(Modal::Library(None)))
                .style(p.secondary()),
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        container(bar).padding(Padding::from([12, 18])).width(Fill).style(p.bar()).into()
    }

    fn grid(&self) -> El<'_> {
        let mut rows = Column::new().spacing(14);
        for r in 0..4 {
            let mut line = Row::new().spacing(14);
            for c in 0..4 {
                line = line.push(self.pad(r * 4 + c));
            }
            rows = rows.push(line.height(Fill));
        }
        rows.into()
    }

    fn pad(&self, i: usize) -> El<'_> {
        let p = self.palette();
        let pad = &self.settings.pads[i];
        let color = p.pad_color(i);
        let progress = self.progress(Slot::Pad(i));
        let playing = progress.is_some();
        let selected = self.settings.edit_mode && self.selected == Some(i);
        let capturing = self.capture == Some(Target::Pad(i)) && self.modal.is_none();
        let cap = keycap(pad.key, p, true, capturing);
        let broken = self.broken[i];
        let ink = if p.solid_pads { p.on_pad } else { p.text };

        let content: El = match &pad.sound {
            None => stack![
                center(icon(icons::PLUS, 30.0, alpha(p.dim, 0.75))),
                container(cap).width(Fill).align_x(Horizontal::Right),
            ]
            .into(),
            Some(sound) => {
                let tag: El = if playing {
                    let (bg, fg) = if p.solid_pads { (p.on_pad, Color::WHITE) } else { (color, p.on_accent) };
                    container(bold("กำลังเล่น").size(11)).padding(Padding::from([1, 7])).style(p.badge(bg, fg)).into()
                } else if broken {
                    row![icon(icons::WARNING, 14.0, ink), bold("ไฟล์หาย").size(11).color(ink)].spacing(4).into()
                } else {
                    let length = match self.seconds[i] {
                        Some(s) if s < 60.0 => format!("{:02}  ·  {s:.1} วิ", i + 1),
                        Some(s) => format!("{:02}  ·  {}:{:02}", i + 1, s as u32 / 60, s as u32 % 60),
                        None => format!("{:02}", i + 1),
                    };
                    bold(length).size(12).color(alpha(ink, 0.5)).into()
                };
                let track = if p.solid_pads { alpha(Color::BLACK, 0.16) } else { alpha(p.text, 0.12) };
                let meter: El = match progress {
                    Some(f) => progress_bar(0.0..=1.0, f)
                        .height(7)
                        .style(p.meter(if p.solid_pads { Color::WHITE } else { color }, track))
                        .into(),
                    None => row![
                        icon(icons::VOLUME, 15.0, alpha(ink, 0.7)),
                        progress_bar(0.0..=100.0, pad.volume as f32)
                            .height(4)
                            .style(p.meter(alpha(ink, if p.solid_pads { 0.55 } else { 0.6 }), track)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                };
                let mut col = column![
                    row![tag, horizontal_space(), cap].align_y(Alignment::Center),
                    container(bold(sound.name.as_str()).size(18).color(ink).wrapping(Wrapping::WordOrGlyph))
                        .height(Fill)
                        .width(Fill)
                        .align_y(Vertical::Center)
                        .clip(true),
                    container(meter).height(12).align_y(Vertical::Center),
                ]
                .spacing(6);
                if !p.solid_pads {
                    // The pad's colour as a light strip along the bottom.
                    col = col.push(container(Space::new(Fill, 5)).style(p.rounded(if broken { alpha(color, 0.3) } else { color }, 3.0)));
                }
                col.into()
            }
        };
        button(container(content).padding(12).width(Fill).height(Fill))
            .padding(0)
            .width(Fill)
            .height(Fill)
            .on_press(Message::PadPressed(i))
            .style(p.pad(color, pad.sound.is_some(), playing, selected, broken))
            .into()
    }

    fn sidebar(&self) -> El<'_> {
        let p = self.palette();
        let content = match self.selected.filter(|_| self.settings.edit_mode) {
            Some(i) => self.inspector(i),
            None if self.settings.edit_mode => column![
                self.edit_intro(),
                self.stop_key_card(),
            ]
            .spacing(14)
            .into(),
            None => self.controls(),
        };
        column![
            scrollable(container(content).padding(Padding { top: 18.0, right: 20.0, bottom: 12.0, left: 18.0 }))
                .height(Fill)
                .style(p.scroll()),
            container(
                row![
                    hint("โดย Art_Masster", p),
                    horizontal_space(),
                    button(row![icon(icons::HELP, 15.0, p.dim), t("วิธีใช้").size(13).color(p.dim)].spacing(6).align_y(Alignment::Center))
                        .padding(Padding::from([4, 8]))
                        .on_press(Message::Open(Modal::Help))
                        .style(p.ghost()),
                ]
                .align_y(Alignment::Center)
            )
            .padding(Padding { top: 6.0, right: 12.0, bottom: 10.0, left: 18.0 }),
        ]
        .into()
    }

    fn controls(&self) -> El<'_> {
        let p = self.palette();
        let s = &self.settings;

        let volume = column![
            row![label("ระดับเสียงหลัก", p), horizontal_space(), bold(format!("{}%", s.master_volume)).size(14).color(p.text)],
            slider(0..=100u8, s.master_volume, Message::Master).step(5u8).on_release(Message::Save).style(p.slider(p.accent)),
        ]
        .spacing(8);

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
                .padding(Padding::from([8, 12]))
                .text_size(14)
                .text_shaping(Shaping::Advanced)
                .font(FONT)
                .style(p.pick())
                .menu_style(p.menu())
        };
        let devices = column![
            row![
                label("อุปกรณ์เสียงออก", p),
                horizontal_space(),
                button(icon(icons::REFRESH, 15.0, p.dim)).padding(5).on_press(Message::RefreshDevices).style(p.ghost()),
            ]
            .align_y(Alignment::Center),
            picker(outputs, out_now, Message::Output),
            Space::new(0, 4),
            label("ส่งเสียงออกอีกเครื่อง", p),
            picker(monitors, mon_now, Message::Monitor),
            hint("เช่น VB-Cable เพื่อให้เสียงเข้าไมค์ใน Discord หรือเกม", p),
        ]
        .spacing(8);

        let global = s.global_hotkeys == Some(true);
        let hotkeys = column![
            row![
                label("ปุ่มลัดทั่วระบบ", p),
                horizontal_space(),
                toggler(global).on_toggle(Message::GlobalHotkeys).size(24).style(p.toggle()),
            ]
            .align_y(Alignment::Center),
            hint(
                if global {
                    "เปิดอยู่: กดปุ่มลัดเล่นเสียงได้แม้อยู่ในเกมหรือแอปอื่น แอปดูเฉพาะปุ่มที่ตั้งไว้ ไม่บันทึกการพิมพ์"
                } else {
                    "ปิดอยู่: ปุ่มลัดทำงานเฉพาะตอนหน้าต่างนี้อยู่ด้านหน้า"
                },
                p
            ),
        ]
        .spacing(8);

        let mut themes = Row::new().spacing(8);
        for theme in PALETTES {
            let on = theme.id == p.id;
            let swatch = row![
                container(Space::new(12, 12)).style(theme.rounded(theme.accent, 6.0)),
                t(theme.label).size(13),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            themes = themes.push(
                button(container(swatch).center_x(Fill))
                    .width(Fill)
                    .padding(Padding::from([7, 4]))
                    .on_press(Message::Theme(theme.id))
                    .style(if on { p.selected() } else { p.secondary() }),
            );
        }

        let files = column![
            label("การตั้งค่า", p),
            row![
                button(container(action(icons::IMPORT, "นำเข้า", p.text)).center_x(Fill))
                    .width(Fill)
                    .padding(10)
                    .on_press(Message::Import)
                    .style(p.secondary()),
                button(container(action(icons::FILE, "ส่งออก", p.text)).center_x(Fill))
                    .width(Fill)
                    .padding(10)
                    .on_press(Message::Export)
                    .style(p.secondary()),
            ]
            .spacing(8),
            button(container(action(icons::TRASH, "ล้างแผงเสียงทั้งหมด", if p.dark { mix(p.danger, Color::WHITE, 0.25) } else { p.danger })).center_x(Fill))
                .width(Fill)
                .padding(10)
                .on_press(Message::Open(Modal::ConfirmClear))
                .style(p.danger_quiet()),
        ]
        .spacing(8);

        let line = || container(Space::new(Fill, 1)).style(p.divider());
        column![volume, line(), devices, line(), hotkeys, line(), column![label("ธีม", p), themes].spacing(8), line(), files]
            .spacing(16)
            .into()
    }

    fn edit_intro(&self) -> El<'_> {
        let p = self.palette();
        container(
            column![
                label("โหมดแก้ไข", p),
                hint("กดที่แผงเพื่อเลือก แล้วตั้งเสียง ชื่อ ระดับเสียง และปุ่มลัดได้ที่นี่", p),
                hint("ลากไฟล์เสียงมาวางบนหน้าต่างเพื่อเพิ่มลงแผงว่างได้เลย", p),
            ]
            .spacing(8),
        )
        .padding(14)
        .width(Fill)
        .style(p.card())
        .into()
    }

    /// The key button of the inspector and of the shortcut dialog: click, then press the key.
    fn key_button(&self, target: Target, key: Option<Hotkey>) -> El<'_> {
        let p = self.palette();
        let capturing = self.capture == Some(target);
        let word = match (capturing, key) {
            (true, _) => "กดปุ่มที่ต้องการ…".to_string(),
            (false, Some(k)) => k.label(),
            (false, None) => "ไม่ได้ตั้ง".to_string(),
        };
        let mut line = row![button(container(bold(word).size(14).color(if key.is_some() || capturing { p.text } else { p.dim })).center_x(Fill))
            .width(Fill)
            .padding(Padding::from([9, 8]))
            .on_press(Message::Capture(target))
            .style(if capturing { p.selected() } else { p.secondary() })]
        .spacing(6)
        .align_y(Alignment::Center);
        if key.is_some() && !capturing {
            line = line.push(button(icon(icons::CLOSE, 14.0, p.dim)).padding(9).on_press(Message::ClearKey(target)).style(p.ghost()));
        }
        line.into()
    }

    fn stop_key_card(&self) -> El<'_> {
        let p = self.palette();
        container(
            column![
                label("ปุ่มหยุดเสียงทั้งหมด", p),
                self.key_button(Target::Stop, self.settings.stop_key),
                button(container(action(icons::RESET, "คืนปุ่มลัดเริ่มต้น", p.text)).center_x(Fill))
                    .width(Fill)
                    .padding(9)
                    .on_press(Message::ResetKeys)
                    .style(p.secondary()),
            ]
            .spacing(10),
        )
        .padding(14)
        .width(Fill)
        .style(p.card())
        .into()
    }

    fn inspector(&self, i: usize) -> El<'_> {
        let p = self.palette();
        let pad = &self.settings.pads[i];
        let color = p.pad_color(i);
        let head = row![
            container(Space::new(10, 22)).style(p.rounded(color, 4.0)),
            bold(format!("แผง {}", i + 1)).size(19).color(p.text),
            horizontal_space(),
            button(icon(icons::CLOSE, 15.0, p.dim)).padding(6).on_press(Message::Escape).style(p.ghost()),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let source = |primary: bool| {
            let style = |on: bool| if on { p.primary() } else { p.secondary() };
            let ink = if primary { p.on_accent } else { p.text };
            row![
                button(container(action(icons::FILE, "ไฟล์เสียง", ink)).center_x(Fill))
                    .width(Fill)
                    .padding(10)
                    .on_press(Message::PickFile(i))
                    .style(style(primary)),
                button(container(action(icons::LIBRARY, "คลังเสียง", p.text)).center_x(Fill))
                    .width(Fill)
                    .padding(10)
                    .on_press(Message::Open(Modal::Library(Some(i))))
                    .style(p.secondary()),
            ]
            .spacing(8)
        };

        let key = column![label("ปุ่มลัด", p), self.key_button(Target::Pad(i), pad.key)].spacing(8);

        let Some(sound) = &pad.sound else {
            return column![
                head,
                container(
                    column![
                        label("แผงนี้ยังว่าง", p),
                        hint("เลือกไฟล์จากเครื่อง หรือหยิบจากคลังเสียงที่มีมาให้ หรือลากไฟล์มาวางบนหน้าต่าง", p),
                        source(true),
                    ]
                    .spacing(10)
                )
                .padding(14)
                .width(Fill)
                .style(p.card()),
                key,
            ]
            .spacing(16)
            .into();
        };

        let name = column![
            label("ชื่อ", p),
            button(
                row![
                    t(sound.name.as_str()).size(15).color(p.text).width(Fill).wrapping(Wrapping::WordOrGlyph),
                    icon(icons::EDIT, 15.0, p.dim),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
            )
            .width(Fill)
            .padding(Padding::from([9, 12]))
            .on_press(Message::Open(Modal::Rename(i, sound.name.clone())))
            .style(p.secondary()),
        ]
        .spacing(8);

        let volume = column![
            row![label("ระดับเสียงของแผงนี้", p), horizontal_space(), bold(format!("{}%", pad.volume)).size(14).color(p.text)],
            slider(0..=100u8, pad.volume, move |v| Message::PadVolume(i, v))
                .step(5u8)
                .on_release(Message::Save)
                .style(p.slider(if p.solid_pads { p.accent } else { color })),
        ]
        .spacing(8);

        let line = || container(Space::new(Fill, 1)).style(p.divider());
        column![
            head,
            button(container(action(icons::PLAY, "ทดลองเล่น", p.on_accent)).center_x(Fill))
                .width(Fill)
                .padding(10)
                .on_press(Message::Test(i))
                .style(p.primary()),
            name,
            volume,
            key,
            line(),
            column![label("เปลี่ยนเสียง", p), source(false)].spacing(8),
            button(container(action(icons::TRASH, "เอาเสียงออกจากแผง", if p.dark { mix(p.danger, Color::WHITE, 0.25) } else { p.danger })).center_x(Fill))
                .width(Fill)
                .padding(10)
                .on_press(Message::ClearPad(i))
                .style(p.danger_quiet()),
        ]
        .spacing(16)
        .into()
    }

    fn dialog<'a>(&'a self, modal: &'a Modal) -> El<'a> {
        let p = self.palette();
        let title = |s: &'a str| bold(s).size(20).color(p.text);
        let close = || button(icon(icons::CLOSE, 16.0, p.dim)).padding(7).on_press(Message::CloseModal).style(p.ghost());
        let cancel = |word: &'a str| button(bold(word).size(14)).padding(Padding::from([9, 18])).on_press(Message::CloseModal).style(p.secondary());

        match modal {
            Modal::Library(target) => {
                let query = self.query.trim().to_lowercase();
                let matches: Vec<(usize, &crate::library::LibSound)> = self
                    .library
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| query.is_empty() || s.key.contains(&query) || s.id.to_string() == query)
                    .collect();
                let mut list = Column::new().spacing(6);
                for pair in matches.chunks(2) {
                    let mut line = Row::new().spacing(6);
                    for &(n, s) in pair {
                        let previewing = self.preview == Some(n);
                        line = line.push(
                            container(
                                row![
                                    button(icon(if previewing { icons::STOP } else { icons::PLAY }, 14.0, if previewing { p.on_accent } else { p.text }))
                                        .padding(9)
                                        .on_press(Message::Preview(n))
                                        .style(if previewing { p.primary() } else { p.ghost() }),
                                    button(
                                        row![
                                            t(format!("#{}", s.id)).size(12).color(p.dim).width(34),
                                            t(s.name.as_str()).size(14).color(p.text).wrapping(Wrapping::None),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center)
                                    )
                                    .width(Fill)
                                    .padding(Padding::from([8, 6]))
                                    .clip(true)
                                    .on_press(Message::Choose(n))
                                    .style(p.ghost()),
                                ]
                                .align_y(Alignment::Center),
                            )
                            .width(Fill)
                            .padding(2)
                            .style(p.card()),
                        );
                    }
                    if pair.len() == 1 {
                        line = line.push(Space::new(Fill, 0));
                    }
                    list = list.push(line);
                }
                let found = matches.len();
                let body: El = if found == 0 {
                    center(hint("ไม่พบเสียงที่ค้นหา", p)).height(160).into()
                } else {
                    scrollable(container(list).padding(Padding { right: 14.0, ..Padding::ZERO })).height(Fill).style(p.scroll()).into()
                };
                let sub = match target {
                    Some(i) => format!("เลือกเสียงให้แผง {} · กดปุ่มเล่นด้านหน้าเพื่อฟังก่อน", i + 1),
                    None => "กดชื่อเสียงเพื่อเพิ่มลงแผงว่างถัดไป · กดปุ่มเล่นด้านหน้าเพื่อฟังก่อน".to_string(),
                };
                container(
                    column![
                        row![
                            icon(icons::LIBRARY, 22.0, p.accent),
                            title("คลังเสียง"),
                            hint(format!("{found} จาก {} เสียง", self.library.len()), p),
                            horizontal_space(),
                            close(),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                        hint(sub, p),
                        text_input("ค้นหาเสียง…", &self.query)
                            .id(search_id())
                            .on_input(Message::Query)
                            .padding(Padding::from([10, 14]))
                            .size(15)
                            .font(FONT)
                            .style(p.input()),
                        body,
                    ]
                    .spacing(12),
                )
                .padding(22)
                .width(760)
                .height(600)
                .into()
            }

            Modal::Shortcuts => {
                let mut grid = Column::new().spacing(10);
                for r in 0..4 {
                    let mut line = Row::new().spacing(10);
                    for c in 0..4 {
                        let i = r * 4 + c;
                        let pad = &self.settings.pads[i];
                        let name = pad.sound.as_ref().map_or("ว่าง", |s| s.name.as_str());
                        line = line.push(
                            column![
                                row![
                                    container(Space::new(6, 14)).style(p.rounded(p.pad_color(i), 3.0)),
                                    t(format!("{} · {name}", i + 1)).size(12.5).color(p.dim).wrapping(Wrapping::None).width(Fill).height(18),
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center)
                                .clip(true),
                                self.key_button(Target::Pad(i), pad.key),
                            ]
                            .spacing(5)
                            .width(Fill),
                        );
                    }
                    grid = grid.push(line);
                }
                container(
                    column![
                        row![icon(icons::KEYBOARD, 22.0, p.accent), title("ปุ่มลัด"), horizontal_space(), close()]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        hint("กดที่ช่อง แล้วกดปุ่มที่ต้องการ ใช้ร่วมกับ Ctrl, Shift, Alt ได้ · Esc ยกเลิก · × ลบปุ่มลัด", p),
                        grid,
                        container(Space::new(Fill, 1)).style(p.divider()),
                        row![
                            column![label("หยุดเสียงทั้งหมด", p), container(self.key_button(Target::Stop, self.settings.stop_key)).width(220)].spacing(6),
                            horizontal_space(),
                            button(action(icons::RESET, "คืนค่าเริ่มต้น", p.text))
                                .padding(Padding::from([9, 14]))
                                .on_press(Message::ResetKeys)
                                .style(p.secondary()),
                            button(bold("เสร็จ").size(14)).padding(Padding::from([9, 22])).on_press(Message::CloseModal).style(p.primary()),
                        ]
                        .spacing(10)
                        .align_y(Alignment::End),
                    ]
                    .spacing(14),
                )
                .padding(22)
                .width(820)
                .into()
            }

            Modal::Rename(i, name) => container(
                column![
                    title("เปลี่ยนชื่อ"),
                    hint(format!("ชื่อที่แสดงบนแผง {}", i + 1), p),
                    text_input("ชื่อเสียง", name)
                        .id(rename_id())
                        .on_input(Message::RenameInput)
                        .on_submit(Message::RenameDone)
                        .padding(Padding::from([10, 14]))
                        .size(15)
                        .font(FONT)
                        .style(p.input()),
                    row![
                        horizontal_space(),
                        cancel("ยกเลิก"),
                        button(bold("บันทึก").size(14)).padding(Padding::from([9, 22])).on_press(Message::RenameDone).style(p.primary()),
                    ]
                    .spacing(10),
                ]
                .spacing(12),
            )
            .padding(22)
            .width(420)
            .into(),

            Modal::Import(imported) => {
                let mut list = Column::new().spacing(3);
                for (i, pad) in imported.pads.iter().enumerate() {
                    let name = pad.sound.as_ref().map_or("ว่าง", |(n, _)| n.as_str());
                    let mut line = row![
                        t(format!("แผง {}", i + 1)).size(13).color(p.dim).width(54),
                        t(name).size(13).color(if pad.sound.is_some() { p.text } else { p.dim }).width(Fill).wrapping(Wrapping::None),
                    ]
                    .spacing(6);
                    if pad.volume != 100 {
                        line = line.push(t(format!("{}%", pad.volume)).size(13).color(p.accent));
                    }
                    if let Some(k) = pad.key {
                        line = line.push(t(k.label()).size(13).color(p.dim).width(90).align_x(Horizontal::Right));
                    }
                    list = list.push(line);
                }
                let mut summary = column![
                    t(format!("ระดับเสียงหลัก {}%", imported.master_volume)).size(14),
                    t(format!("เสียง {}/{PADS} แผง", imported.sound_count())).size(14),
                    t(format!("ปุ่มลัด {}/{PADS} แผง", imported.key_count())).size(14),
                    t(format!("ปรับระดับเสียงรายแผง {} แผง", imported.custom_volume_count())).size(14),
                ]
                .spacing(3);
                if imported.skipped > 0 {
                    summary = summary.push(t(format!("อ่านไม่ได้ {} เสียง (จะถูกข้าม)", imported.skipped)).size(14).color(p.danger));
                }
                container(
                    column![
                        title("นำเข้าการตั้งค่า"),
                        hint("เสียง ระดับเสียง และปุ่มลัดในไฟล์นี้จะแทนที่ของปัจจุบันทั้งหมด", p),
                        container(summary).padding(12).width(Fill).style(p.card()),
                        container(scrollable(container(list).padding(Padding { right: 14.0, ..Padding::ZERO })).height(210).style(p.scroll()))
                            .padding(12)
                            .width(Fill)
                            .style(p.card()),
                        row![
                            horizontal_space(),
                            cancel("ยกเลิก"),
                            button(bold("นำเข้า").size(14)).padding(Padding::from([9, 22])).on_press(Message::ImportConfirm).style(p.primary()),
                        ]
                        .spacing(10),
                    ]
                    .spacing(12),
                )
                .padding(22)
                .width(500)
                .into()
            }

            Modal::ConfirmClear => container(
                column![
                    title("ล้างแผงเสียงทั้งหมด?"),
                    t("เสียงและระดับเสียงของทุกแผงจะถูกลบ ย้อนกลับไม่ได้ ปุ่มลัดและระดับเสียงหลักยังอยู่เหมือนเดิม")
                        .size(14)
                        .color(p.dim),
                    row![
                        horizontal_space(),
                        cancel("ยกเลิก"),
                        button(bold("ล้างทั้งหมด").size(14)).padding(Padding::from([9, 22])).on_press(Message::ClearAll).style(p.danger_button()),
                    ]
                    .spacing(10),
                ]
                .spacing(14),
            )
            .padding(22)
            .width(440)
            .into(),

            Modal::Welcome | Modal::Help => {
                let welcome = matches!(modal, Modal::Welcome);
                let step = |n: u32, head: &'a str, body: &'a str| {
                    row![
                        container(bold(n.to_string()).size(15).color(p.on_accent))
                            .width(30)
                            .height(30)
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .style(p.rounded(p.accent, 15.0)),
                        column![bold(head).size(15).color(p.text), hint(body, p)].spacing(2).width(Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Start)
                };
                let steps = column![
                    step(1, "เปิด \"แก้ไข\" แล้วกดเลือกแผง", "หรือกดที่แผงว่างได้เลย แถบด้านขวาจะเปลี่ยนเป็นตัวตั้งค่าของแผงนั้น"),
                    step(2, "ใส่เสียง", "เลือกไฟล์จากเครื่อง หยิบจากคลังเสียงกว่า 280 เสียง หรือลากไฟล์มาวางบนหน้าต่าง"),
                    step(3, "ตั้งชื่อ ระดับเสียง และปุ่มลัด", "ปุ่มลัดใช้ร่วมกับ Ctrl, Shift, Alt ได้ และมีปุ่มหยุดเสียงทั้งหมด (เริ่มต้นคือ Y)"),
                    step(4, "กดเล่น", "กดที่แผงหรือกดปุ่มลัด กดซ้ำเพื่อเริ่มใหม่ จะส่งเสียงออกอีกเครื่อง (เช่น VB-Cable) ให้เข้าไมค์ด้วยก็ได้"),
                ]
                .spacing(14);
                let foot: El = if welcome {
                    column![
                        container(
                            column![
                                row![icon(icons::KEYBOARD, 18.0, p.accent), label("ให้ปุ่มลัดทำงานนอกแอปด้วยไหม?", p)].spacing(8).align_y(Alignment::Center),
                                hint("ถ้าอนุญาต แอปจะคอยดูการกดแป้นพิมพ์ทั่วระบบ เพื่อให้กดปุ่มลัดเล่นเสียงได้ตอนอยู่ในเกมหรือแอปอื่น", p),
                                hint("แอปเทียบเฉพาะปุ่มลัดที่ตั้งไว้ ไม่บันทึก ไม่เก็บ และไม่ส่งการกดปุ่มไปที่ไหน เปลี่ยนใจได้ทุกเมื่อที่แถบด้านขวา", p),
                            ]
                            .spacing(6)
                        )
                        .padding(14)
                        .width(Fill)
                        .style(p.card()),
                        row![
                            horizontal_space(),
                            button(bold("ใช้เฉพาะในแอป").size(14)).padding(Padding::from([10, 18])).on_press(Message::Welcome(false)).style(p.secondary()),
                            button(bold("อนุญาต").size(14)).padding(Padding::from([10, 26])).on_press(Message::Welcome(true)).style(p.primary()),
                        ]
                        .spacing(10),
                    ]
                    .spacing(14)
                    .into()
                } else {
                    row![horizontal_space(), button(bold("เข้าใจแล้ว").size(14)).padding(Padding::from([9, 22])).on_press(Message::CloseModal).style(p.primary())].into()
                };
                container(
                    column![
                        row![icon(icons::LOGO, 26.0, p.accent), title(if welcome { "ยินดีต้อนรับสู่ Kuro Sound Pad" } else { "วิธีใช้" })]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        steps,
                        foot,
                    ]
                    .spacing(18),
                )
                .padding(24)
                .width(560)
                .into()
            }
        }
    }
}

#[allow(dead_code)]
fn _unused() -> (Length, fn() -> iced::widget::Space) {
    (Length::Shrink, vertical_space)
}
