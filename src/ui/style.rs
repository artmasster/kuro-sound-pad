//! Colours and widget styles. Three looks share one layout: a palette decides everything.
use iced::border::Radius;
use iced::widget::{button, container, pick_list, progress_bar, scrollable, slider, text_input, toggler};
use iced::{overlay::menu, Background, Border, Color, Gradient, Shadow, Vector};

const fn rgb(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

pub fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub id: &'static str,
    pub label: &'static str,
    /// Window background.
    pub bg: Color,
    /// Top bar and sidebar.
    pub panel: Color,
    /// Pads, cards, buttons.
    pub card: Color,
    pub card_hover: Color,
    pub line: Color,
    pub text: Color,
    pub dim: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub on_danger: Color,
    /// One colour per pad, repeating.
    pub pad_colors: &'static [Color],
    /// How many pads in a row share a colour: 4 colours a whole row, 1 gives each pad the next one.
    pub pad_run: usize,
    /// Pads are filled with their colour (dark text on top) instead of carrying it as a light strip.
    pub solid_pads: bool,
    pub on_pad: Color,
    pub dark: bool,
}

pub const STUDIO: Palette = Palette {
    id: "studio",
    label: "สตูดิโอ",
    bg: rgb(0x111216),
    panel: rgb(0x16171C),
    card: rgb(0x1F2127),
    card_hover: rgb(0x272A31),
    line: rgb(0x30323B),
    text: rgb(0xF3F3F5),
    dim: rgb(0x9C9EA9),
    accent: rgb(0xFFB020),
    on_accent: rgb(0x1B1300),
    danger: rgb(0xE5383B),
    on_danger: rgb(0xFFFFFF),
    pad_colors: &[rgb(0xFFB020), rgb(0xFF6B5E), rgb(0x2FE6C8), rgb(0xB45CFF)],
    pad_run: 4,
    solid_pads: false,
    on_pad: rgb(0xF3F3F5),
    dark: true,
};

pub const NEON: Palette = Palette {
    id: "neon",
    label: "นีออน",
    bg: rgb(0x0A0C0F),
    panel: rgb(0x0E1014),
    card: rgb(0x15181D),
    card_hover: rgb(0x1D2127),
    line: rgb(0x2B3038),
    text: rgb(0xF4F6F8),
    dim: rgb(0x8C929B),
    accent: rgb(0xC6FF3D),
    on_accent: rgb(0x0B0D10),
    danger: rgb(0xF0312F),
    on_danger: rgb(0xFFFFFF),
    pad_colors: &[rgb(0xC6FF3D)],
    pad_run: 1,
    solid_pads: false,
    on_pad: rgb(0xF4F6F8),
    dark: true,
};

pub const CANDY: Palette = Palette {
    id: "candy",
    label: "ลูกกวาด",
    bg: rgb(0xF6F3EE),
    panel: rgb(0xFCFAF7),
    card: rgb(0xFFFFFF),
    card_hover: rgb(0xF1EDE6),
    line: rgb(0xD8D2C7),
    text: rgb(0x1C1C21),
    dim: rgb(0x6C6C75),
    accent: rgb(0x14B870),
    on_accent: rgb(0xFFFFFF),
    danger: rgb(0xEE3D45),
    on_danger: rgb(0xFFFFFF),
    pad_colors: &[rgb(0xFF5F63), rgb(0xFF9A3C), rgb(0xFFD93B), rgb(0x5BE6A0), rgb(0x56B9FF), rgb(0xB976FF)],
    pad_run: 1,
    solid_pads: true,
    on_pad: rgb(0x1C1C21),
    dark: false,
};

pub type ButtonStyle = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

pub const PALETTES: [Palette; 3] = [STUDIO, NEON, CANDY];

pub fn palette(id: &str) -> Palette {
    PALETTES.into_iter().find(|p| p.id == id).unwrap_or(STUDIO)
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border { color, width, radius: Radius::from(radius) }
}

fn shadow(p: Palette, y: f32, blur: f32) -> Shadow {
    Shadow { color: alpha(Color::BLACK, if p.dark { 0.45 } else { 0.14 }), offset: Vector::new(0.0, y), blur_radius: blur }
}

impl Palette {
    pub fn pad_color(&self, i: usize) -> Color {
        self.pad_colors[(i / self.pad_run) % self.pad_colors.len()]
    }

    pub fn fill(&self, c: Color) -> impl Fn(&iced::Theme) -> container::Style {
        move |_| container::Style { background: Some(c.into()), ..Default::default() }
    }

    pub fn bar(&self) -> impl Fn(&iced::Theme) -> container::Style {
        let p = *self;
        move |_| container::Style {
            background: Some(p.panel.into()),
            text_color: Some(p.text),
            ..Default::default()
        }
    }

    pub fn divider(&self) -> impl Fn(&iced::Theme) -> container::Style {
        self.fill(self.line)
    }

    pub fn rounded(&self, c: Color, radius: f32) -> impl Fn(&iced::Theme) -> container::Style {
        move |_| container::Style {
            background: Some(c.into()),
            border: border(Color::TRANSPARENT, 0.0, radius),
            ..Default::default()
        }
    }

    /// A panel inside the sidebar or a dialog.
    pub fn card(&self) -> impl Fn(&iced::Theme) -> container::Style {
        let p = *self;
        move |_| container::Style {
            background: Some(p.card.into()),
            border: border(p.line, 1.0, 12.0),
            text_color: Some(p.text),
            ..Default::default()
        }
    }

    pub fn dialog(&self) -> impl Fn(&iced::Theme) -> container::Style {
        let p = *self;
        move |_| container::Style {
            background: Some(p.panel.into()),
            border: border(p.line, 1.0, 18.0),
            text_color: Some(p.text),
            shadow: shadow(p, 16.0, 48.0),
        }
    }

    pub fn scrim(&self) -> impl Fn(&iced::Theme) -> container::Style {
        let c = if self.dark { alpha(Color::BLACK, 0.62) } else { alpha(rgb(0x2A2620), 0.45) };
        self.fill(c)
    }

    pub fn toast(&self, error: bool) -> impl Fn(&iced::Theme) -> container::Style {
        let p = *self;
        move |_| container::Style {
            background: Some(if error { p.danger } else { p.text }.into()),
            text_color: Some(if error { p.on_danger } else { p.bg }),
            border: border(Color::TRANSPARENT, 0.0, 999.0),
            shadow: shadow(p, 6.0, 20.0),
        }
    }

    /// The small box naming a pad's key.
    pub fn keycap(&self, on_pad: bool, active: bool) -> impl Fn(&iced::Theme) -> container::Style {
        let p = *self;
        move |_| {
            let (bg, fg, line) = if active {
                (p.accent, p.on_accent, p.accent)
            } else if on_pad && p.solid_pads {
                (alpha(Color::WHITE, 0.92), p.on_pad, alpha(p.on_pad, 0.85))
            } else {
                (alpha(p.bg, if p.dark { 0.55 } else { 1.0 }), p.text, if p.dark { mix(p.line, p.text, 0.25) } else { p.text })
            };
            container::Style {
                background: Some(bg.into()),
                text_color: Some(fg),
                border: border(line, 1.5, 7.0),
                ..Default::default()
            }
        }
    }

    pub fn badge(&self, c: Color, fg: Color) -> impl Fn(&iced::Theme) -> container::Style {
        move |_| container::Style {
            background: Some(c.into()),
            text_color: Some(fg),
            border: border(Color::TRANSPARENT, 0.0, 5.0),
            ..Default::default()
        }
    }

    /// A sound pad. `color` is the pad's own colour.
    pub fn pad(
        &self,
        color: Color,
        filled: bool,
        playing: bool,
        selected: bool,
        broken: bool,
    ) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
        let p = *self;
        move |_, status| {
            let hover = matches!(status, button::Status::Hovered);
            let down = matches!(status, button::Status::Pressed);
            let (bg, text): (Background, Color) = if !filled {
                (if hover || down { alpha(p.card_hover, 0.9) } else { alpha(p.card, if p.dark { 0.35 } else { 0.5 }) }.into(), p.dim)
            } else if p.solid_pads {
                let base = if broken { mix(color, p.bg, 0.6) } else { color };
                let top = mix(base, Color::WHITE, if down { 0.0 } else if hover { 0.22 } else { 0.1 });
                let bottom = mix(base, Color::BLACK, if down { 0.1 } else { 0.03 });
                (Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, top).add_stop(1.0, bottom)).into(), p.on_pad)
            } else {
                let lift = if playing { 0.16 } else if down { 0.1 } else if hover { 0.05 } else { 0.0 };
                let base = mix(p.card, color, lift);
                let top = mix(base, Color::WHITE, 0.035);
                let bottom = mix(base, Color::BLACK, 0.12);
                (Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, top).add_stop(1.0, bottom)).into(), p.on_pad)
            };
            let (line, width) = if selected {
                (p.text, 2.5)
            } else if playing {
                (if p.solid_pads { Color::WHITE } else { color }, 2.5)
            } else if !filled {
                (if hover { p.dim } else { p.line }, 1.5)
            } else if p.solid_pads {
                (alpha(Color::BLACK, 0.08), 1.0)
            } else {
                (if hover { mix(p.line, color, 0.45) } else { p.line }, 1.0)
            };
            let glow = if playing {
                Shadow { color: alpha(color, if p.dark { 0.45 } else { 0.55 }), offset: Vector::new(0.0, 0.0), blur_radius: 22.0 }
            } else if filled {
                shadow(p, if down { 1.0 } else { 3.0 }, if down { 3.0 } else { 9.0 })
            } else {
                Shadow::default()
            };
            button::Style { background: Some(bg), text_color: text, border: border(line, width, 14.0), shadow: glow }
        }
    }

    fn button(&self, bg: Color, fg: Color, line: Color, hover: Color) -> ButtonStyle {
        let p = *self;
        Box::new(move |_, status| {
            let (bg, fg, line) = match status {
                button::Status::Hovered => (hover, fg, line),
                button::Status::Pressed => (mix(hover, if p.dark { Color::BLACK } else { p.text }, 0.12), fg, line),
                button::Status::Disabled => (alpha(bg, 0.4), alpha(fg, 0.4), alpha(line, 0.4)),
                button::Status::Active => (bg, fg, line),
            };
            button::Style { background: Some(bg.into()), text_color: fg, border: border(line, 1.0, 10.0), shadow: Shadow::default() }
        })
    }

    /// The default button: quiet, outlined.
    pub fn secondary(&self) -> ButtonStyle {
        self.button(self.card, self.text, self.line, self.card_hover)
    }

    pub fn primary(&self) -> ButtonStyle {
        self.button(self.accent, self.on_accent, self.accent, mix(self.accent, Color::WHITE, 0.18))
    }

    pub fn danger_button(&self) -> ButtonStyle {
        self.button(self.danger, self.on_danger, self.danger, mix(self.danger, Color::WHITE, 0.14))
    }

    /// Red text on a quiet button: destructive, but not shouting.
    pub fn danger_quiet(&self) -> ButtonStyle {
        let red = if self.dark { mix(self.danger, Color::WHITE, 0.25) } else { self.danger };
        self.button(alpha(self.danger, 0.1), red, alpha(self.danger, 0.4), alpha(self.danger, 0.2))
    }

    /// No box until hovered: icon buttons, list rows.
    pub fn ghost(&self) -> ButtonStyle {
        self.button(Color::TRANSPARENT, self.text, Color::TRANSPARENT, self.card_hover)
    }

    /// A button that is "on" (the key being captured, the chosen theme).
    pub fn selected(&self) -> ButtonStyle {
        self.button(alpha(self.accent, 0.16), self.text, self.accent, alpha(self.accent, 0.24))
    }

    pub fn slider(&self, color: Color) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
        let p = *self;
        move |_, status| slider::Style {
            rail: slider::Rail {
                backgrounds: (color.into(), mix(p.line, p.dim, 0.15).into()),
                width: 6.0,
                border: border(Color::TRANSPARENT, 0.0, 3.0),
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: if matches!(status, slider::Status::Active) { 8.0 } else { 9.5 } },
                background: Color::WHITE.into(),
                border_width: 2.0,
                border_color: color,
            },
        }
    }

    pub fn meter(&self, bar: Color, track: Color) -> impl Fn(&iced::Theme) -> progress_bar::Style {
        move |_| progress_bar::Style { background: track.into(), bar: bar.into(), border: border(Color::TRANSPARENT, 0.0, 99.0) }
    }

    pub fn input(&self) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
        let p = *self;
        move |_, status| text_input::Style {
            background: p.card.into(),
            border: border(if matches!(status, text_input::Status::Focused) { p.accent } else { p.line }, 1.5, 10.0),
            icon: p.dim,
            placeholder: alpha(p.dim, 0.8),
            value: p.text,
            selection: alpha(p.accent, 0.35),
        }
    }

    pub fn pick(&self) -> impl Fn(&iced::Theme, pick_list::Status) -> pick_list::Style {
        let p = *self;
        move |_, status| pick_list::Style {
            text_color: p.text,
            placeholder_color: p.dim,
            handle_color: p.dim,
            background: if matches!(status, pick_list::Status::Active) { p.card } else { p.card_hover }.into(),
            border: border(if matches!(status, pick_list::Status::Opened) { p.accent } else { p.line }, 1.0, 10.0),
        }
    }

    pub fn menu(&self) -> impl Fn(&iced::Theme) -> menu::Style {
        let p = *self;
        move |_| menu::Style {
            background: p.card.into(),
            border: border(p.line, 1.0, 10.0),
            text_color: p.text,
            selected_text_color: p.on_accent,
            selected_background: p.accent.into(),
        }
    }

    pub fn toggle(&self) -> impl Fn(&iced::Theme, toggler::Status) -> toggler::Style {
        let p = *self;
        move |_, status| {
            let on = match status {
                toggler::Status::Active { is_toggled } | toggler::Status::Hovered { is_toggled } => is_toggled,
                toggler::Status::Disabled => false,
            };
            toggler::Style {
                background: if on { p.accent } else { mix(p.line, p.dim, 0.3) },
                background_border_width: 0.0,
                background_border_color: Color::TRANSPARENT,
                foreground: Color::WHITE,
                foreground_border_width: 0.0,
                foreground_border_color: Color::TRANSPARENT,
            }
        }
    }

    pub fn scroll(&self) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style {
        let p = *self;
        move |_, status| {
            let active = !matches!(status, scrollable::Status::Active);
            let rail = scrollable::Rail {
                background: None,
                border: border(Color::TRANSPARENT, 0.0, 4.0),
                scroller: scrollable::Scroller {
                    color: alpha(p.dim, if active { 0.7 } else { 0.35 }),
                    border: border(Color::TRANSPARENT, 0.0, 4.0),
                },
            };
            scrollable::Style { container: container::Style::default(), vertical_rail: rail, horizontal_rail: rail, gap: None }
        }
    }
}
