//! The look: one black piece of hardware. Matte chassis, recessed panels, rubber pads with a light strip, an amber
//! display. Everything here is colours and widget styles; the layout is in `view.rs`.
use iced::border::Radius;
use iced::widget::{button, container, pick_list, progress_bar, scrollable, slider, text_input, toggler};
use iced::{overlay::menu, Background, Border, Color, Gradient, Shadow, Vector};
use std::f32::consts::PI;

pub const fn rgb(hex: u32) -> Color {
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

/// The chassis between the panels.
pub const CHASSIS: Color = rgb(0x08080A);
pub const PANEL_TOP: Color = rgb(0x19191C);
pub const PANEL_BOTTOM: Color = rgb(0x111113);
/// A hole in a panel: fields, list rows, the grooves of faders.
pub const WELL: Color = rgb(0x0A0A0C);
pub const EDGE: Color = rgb(0x000000);
/// The light catching the top of a raised part.
pub const RIM: Color = rgb(0x2E2E33);
pub const KEY_TOP: Color = rgb(0x2B2C31);
pub const KEY_BOTTOM: Color = rgb(0x1C1D21);
/// Silk-screen print.
pub const PRINT: Color = rgb(0xECECEF);
pub const LABEL: Color = rgb(0x85858E);
pub const FAINT: Color = rgb(0x55555D);
pub const AMBER: Color = rgb(0xFFB627);
pub const AMBER_DIM: Color = rgb(0x8F6A22);
pub const LCD_TOP: Color = rgb(0x0B0904);
pub const LCD_BOTTOM: Color = rgb(0x17120A);
pub const RED: Color = rgb(0xFF4438);
pub const GREEN: Color = rgb(0x3DDC84);

/// The pad lights a user can pick.
pub const LEDS: [Color; 8] = [
    rgb(0xFFB020), // amber
    rgb(0xFF5E4F), // coral
    rgb(0xFF4FA3), // pink
    rgb(0xA868FF), // violet
    rgb(0x4C8DFF), // blue
    rgb(0x22D3EE), // cyan
    rgb(0x3DDC84), // green
    rgb(0xE8E8EE), // white
];

/// A pad's light: its own choice, else its row's (amber, coral, cyan, violet).
pub fn led(choice: Option<u8>, pad: usize) -> Color {
    let i = choice.map_or([0, 1, 5, 3][(pad / 4) % 4], |c| c as usize);
    LEDS[i % LEDS.len()]
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border { color, width, radius: Radius::from(radius) }
}

/// Top-to-bottom gradient.
fn down(top: Color, bottom: Color) -> Background {
    Gradient::Linear(iced::gradient::Linear::new(PI).add_stop(0.0, top).add_stop(1.0, bottom)).into()
}

fn drop(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow { color: alpha(Color::BLACK, a), offset: Vector::new(0.0, y), blur_radius: blur }
}

fn glow(c: Color, blur: f32, a: f32) -> Shadow {
    Shadow { color: alpha(c, a), offset: Vector::ZERO, blur_radius: blur }
}

pub fn fill(c: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style { background: Some(c.into()), ..Default::default() }
}

pub fn dot(c: Color, radius: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style { background: Some(c.into()), border: border(Color::TRANSPARENT, 0.0, radius), ..Default::default() }
}

/// A lit indicator: the colour and its halo.
pub fn lamp(c: Color, on: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(if on { c } else { mix(c, Color::BLACK, 0.78) }.into()),
        border: border(alpha(Color::BLACK, 0.8), 1.0, 99.0),
        shadow: if on { glow(c, 8.0, 0.75) } else { Shadow::default() },
        ..Default::default()
    }
}

pub fn chassis(_: &iced::Theme) -> container::Style {
    container::Style { background: Some(down(rgb(0x0C0C0E), CHASSIS)), text_color: Some(PRINT), ..Default::default() }
}

/// A plate screwed onto the chassis.
pub fn panel(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(down(PANEL_TOP, PANEL_BOTTOM)),
        border: border(RIM, 1.0, 10.0),
        shadow: drop(2.0, 6.0, 0.7),
        text_color: Some(PRINT),
    }
}

/// A recess in a plate.
pub fn well(_: &iced::Theme) -> container::Style {
    container::Style { background: Some(WELL.into()), border: border(EDGE, 1.0, 7.0), ..Default::default() }
}

pub fn lcd(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(down(LCD_TOP, LCD_BOTTOM)),
        border: border(EDGE, 2.0, 8.0),
        text_color: Some(AMBER),
        shadow: glow(AMBER, 14.0, 0.08),
    }
}

pub fn dialog(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(down(rgb(0x1D1D21), rgb(0x131315))),
        border: border(rgb(0x3A3A40), 1.0, 12.0),
        text_color: Some(PRINT),
        shadow: drop(18.0, 50.0, 0.8),
    }
}

pub fn scrim(_: &iced::Theme) -> container::Style {
    container::Style { background: Some(alpha(Color::BLACK, 0.7).into()), ..Default::default() }
}

/// The chip that follows the cursor while something is dragged.
pub fn ghost(c: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(down(KEY_TOP, KEY_BOTTOM)),
        border: border(c, 1.5, 8.0),
        text_color: Some(PRINT),
        shadow: drop(6.0, 16.0, 0.8),
    }
}

/// The printed key on a pad.
pub fn keycap(active: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(if active { AMBER } else { rgb(0x0D0D0F) }.into()),
        text_color: Some(if active { rgb(0x1A1000) } else { PRINT }),
        border: border(if active { AMBER } else { rgb(0x3C3C43) }, 1.0, 5.0),
        ..Default::default()
    }
}

#[derive(Clone, Copy, Default)]
pub struct PadLook {
    pub filled: bool,
    pub playing: bool,
    pub selected: bool,
    pub hover: bool,
    pub pressed: bool,
    /// Something dragged is over this pad.
    pub target: bool,
    /// This pad is the thing being dragged.
    pub lifted: bool,
    pub broken: bool,
}

/// A rubber pad.
pub fn pad(color: Color, look: PadLook) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| {
        let (mut top, mut bottom) = if look.filled { (rgb(0x2E2F35), rgb(0x1D1E22)) } else { (rgb(0x151517), rgb(0x101012)) };
        if look.playing {
            top = mix(top, color, 0.34);
            bottom = mix(bottom, color, 0.14);
        } else if look.pressed {
            // Pushed in: the light comes from below.
            (top, bottom) = (mix(bottom, Color::BLACK, 0.25), mix(top, Color::BLACK, 0.25));
        } else if look.hover || look.target {
            top = mix(top, Color::WHITE, 0.06);
            bottom = mix(bottom, Color::WHITE, 0.04);
        }
        let (line, width) = if look.target {
            (PRINT, 2.0)
        } else if look.playing {
            (color, 2.0)
        } else if look.selected {
            (alpha(color, 0.95), 2.0)
        } else if look.filled {
            (rgb(0x3B3C42), 1.0)
        } else {
            (rgb(0x26262A), 1.0)
        };
        let shadow = if look.playing {
            glow(color, 26.0, 0.55)
        } else if look.target {
            glow(PRINT, 16.0, 0.3)
        } else if look.pressed || !look.filled {
            Shadow::default()
        } else {
            drop(3.0, 6.0, 0.8)
        };
        container::Style {
            background: Some(down(top, bottom)),
            border: border(line, width, 12.0),
            shadow,
            text_color: Some(if look.lifted { alpha(PRINT, 0.35) } else { PRINT }),
        }
    }
}

/// A library row.
pub fn row(playing: bool, hover: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(if playing { alpha(AMBER, 0.16) } else if hover { rgb(0x1E1E22) } else { Color::TRANSPARENT }.into()),
        border: border(if playing { alpha(AMBER, 0.55) } else { Color::TRANSPARENT }, 1.0, 5.0),
        ..Default::default()
    }
}

pub type ButtonStyle = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

fn key(top: Color, bottom: Color, text: Color, line: Color, radius: f32) -> ButtonStyle {
    Box::new(move |_, status| {
        let (t, b, shadow) = match status {
            button::Status::Hovered => (mix(top, Color::WHITE, 0.08), mix(bottom, Color::WHITE, 0.06), drop(2.0, 4.0, 0.7)),
            button::Status::Pressed => (mix(bottom, Color::BLACK, 0.2), mix(top, Color::BLACK, 0.2), Shadow::default()),
            _ => (top, bottom, drop(2.0, 4.0, 0.7)),
        };
        let fade = if matches!(status, button::Status::Disabled) { 0.4 } else { 1.0 };
        button::Style {
            background: Some(down(alpha(t, fade), alpha(b, fade))),
            text_color: alpha(text, fade),
            border: border(line, 1.0, radius),
            shadow,
        }
    })
}

/// A small hardware key.
pub fn cap() -> ButtonStyle {
    key(KEY_TOP, KEY_BOTTOM, PRINT, rgb(0x3D3E44), 6.0)
}

/// A key that is lit: armed, chosen.
pub fn cap_lit() -> ButtonStyle {
    key(mix(KEY_TOP, AMBER, 0.45), mix(KEY_BOTTOM, AMBER, 0.3), PRINT, AMBER, 6.0)
}

pub fn cap_danger() -> ButtonStyle {
    key(rgb(0x3A1D1D), rgb(0x271314), rgb(0xFF8A80), rgb(0x6A2A2A), 6.0)
}

pub fn cap_go() -> ButtonStyle {
    key(mix(AMBER, Color::WHITE, 0.1), mix(AMBER, Color::BLACK, 0.18), rgb(0x1A1000), rgb(0x5A3C00), 6.0)
}

/// Flat, no body: the little buttons inside list rows and fields.
pub fn flat() -> ButtonStyle {
    Box::new(move |_, status| button::Style {
        background: Some(match status {
            button::Status::Hovered => rgb(0x2A2A2F),
            button::Status::Pressed => rgb(0x101012),
            _ => Color::TRANSPARENT,
        }.into()),
        text_color: PRINT,
        border: border(Color::TRANSPARENT, 0.0, 5.0),
        shadow: Shadow::default(),
    })
}

/// The big stop button; it glows while something plays.
pub fn stop(live: bool) -> ButtonStyle {
    Box::new(move |_, status| {
        let (top, bottom) = match (status, live) {
            (button::Status::Pressed, _) => (rgb(0x9C1A16), rgb(0xD2362C)),
            (button::Status::Hovered, _) => (rgb(0xFF6A5C), rgb(0xD92A22)),
            (_, true) => (rgb(0xFF5548), rgb(0xD0241D)),
            (_, false) => (rgb(0xB9342C), rgb(0x851915)),
        };
        button::Style {
            background: Some(down(top, bottom)),
            text_color: Color::WHITE,
            border: border(rgb(0x2A0605), 2.0, 12.0),
            shadow: if live { glow(RED, 22.0, 0.6) } else { drop(3.0, 6.0, 0.8) },
        }
    })
}

/// A round colour choice; the chosen one gets a ring and a halo.
pub fn swatch(c: Color, chosen: bool) -> ButtonStyle {
    Box::new(move |_, status| button::Style {
        background: Some(if chosen || matches!(status, button::Status::Hovered) { c } else { mix(c, Color::BLACK, 0.45) }.into()),
        text_color: PRINT,
        border: border(if chosen { PRINT } else { EDGE }, if chosen { 2.0 } else { 1.0 }, 99.0),
        shadow: if chosen { glow(c, 10.0, 0.8) } else { Shadow::default() },
    })
}

/// A fader: a groove, lit up to the cap, and a metal cap.
pub fn fader(color: Color) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
    move |_, status| slider::Style {
        rail: slider::Rail { backgrounds: (color.into(), WELL.into()), width: 6.0, border: border(EDGE, 1.0, 3.0) },
        handle: slider::Handle {
            shape: slider::HandleShape::Rectangle { width: 14, border_radius: Radius::from(3.0) },
            background: if matches!(status, slider::Status::Active) { rgb(0xC9C9CF) } else { rgb(0xF2F2F5) }.into(),
            border_width: 1.0,
            border_color: EDGE,
        },
    }
}

pub fn strip(bar: Color, track: Color) -> impl Fn(&iced::Theme) -> progress_bar::Style {
    move |_| progress_bar::Style { background: track.into(), bar: bar.into(), border: border(Color::TRANSPARENT, 0.0, 99.0) }
}

pub fn input(_: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: WELL.into(),
        border: border(if matches!(status, text_input::Status::Focused) { AMBER } else { rgb(0x2A2A2F) }, 1.0, 6.0),
        icon: LABEL,
        placeholder: FAINT,
        value: PRINT,
        selection: alpha(AMBER, 0.4),
    }
}

pub fn pick(_: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    pick_list::Style {
        text_color: PRINT,
        placeholder_color: FAINT,
        handle_color: LABEL,
        background: WELL.into(),
        border: border(if matches!(status, pick_list::Status::Opened) { AMBER } else { rgb(0x2A2A2F) }, 1.0, 6.0),
    }
}

pub fn menu(_: &iced::Theme) -> menu::Style {
    menu::Style {
        background: rgb(0x17171A).into(),
        border: border(rgb(0x3A3A40), 1.0, 6.0),
        text_color: PRINT,
        selected_text_color: rgb(0x1A1000),
        selected_background: AMBER.into(),
    }
}

/// A slide switch: dark when off, lit amber when on.
pub fn switch(_: &iced::Theme, status: toggler::Status) -> toggler::Style {
    let on = match status {
        toggler::Status::Active { is_toggled } | toggler::Status::Hovered { is_toggled } => is_toggled,
        toggler::Status::Disabled => false,
    };
    toggler::Style {
        background: if on { mix(AMBER, Color::BLACK, 0.25) } else { WELL },
        background_border_width: 1.0,
        background_border_color: if on { AMBER } else { rgb(0x34343A) },
        foreground: if on { Color::WHITE } else { rgb(0x9A9AA2) },
        foreground_border_width: 1.0,
        foreground_border_color: EDGE,
    }
}

pub fn scroll(_: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let active = !matches!(status, scrollable::Status::Active);
    let rail = scrollable::Rail {
        background: Some(WELL.into()),
        border: border(Color::TRANSPARENT, 0.0, 3.0),
        scroller: scrollable::Scroller { color: if active { rgb(0x6A6A72) } else { rgb(0x3A3A40) }, border: border(Color::TRANSPARENT, 0.0, 3.0) },
    };
    scrollable::Style { container: container::Style::default(), vertical_rail: rail, horizontal_rail: rail, gap: None }
}
