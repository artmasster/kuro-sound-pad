//! Line icons, drawn here as SVG and tinted by the widget's colour.
use iced::widget::svg::{self, Svg};
use iced::{Color, Length};

macro_rules! icon {
    ($name:ident, $body:literal) => {
        pub const $name: &[u8] = concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"##,
            $body,
            "</svg>"
        )
        .as_bytes();
    };
}

icon!(LOGO, r##"<path d="M3 10v4M7.5 6v12M12 3v18M16.5 7v10M21 10v4" stroke-width="2.4"/>"##);
icon!(STOP, r##"<rect x="5" y="5" width="14" height="14" rx="2.5" fill="#fff" stroke="none"/>"##);
icon!(PLAY, r##"<path d="M7 4.5v15l12-7.5z" fill="#fff" stroke-linejoin="round"/>"##);
icon!(PLUS, r##"<path d="M12 5v14M5 12h14"/>"##);
icon!(CLOSE, r##"<path d="M6 6l12 12M18 6L6 18"/>"##);
icon!(VOLUME, r##"<path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5z" fill="#fff"/><path d="M15.5 9a4.2 4.2 0 0 1 0 6M18.3 6.3a8 8 0 0 1 0 11.4"/>"##);
icon!(KEYBOARD, r##"<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6.5 10h.01M10 10h.01M13.5 10h.01M17 10h.01M6.5 13.8h.01M17.5 13.8h.01M9.5 14h5"/>"##);
icon!(LIBRARY, r##"<path d="M3 7.5a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"##);
icon!(FILE, r##"<path d="M12 15.5V4M7.5 8.5L12 4l4.5 4.5M4.5 15v3a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-3"/>"##);
icon!(IMPORT, r##"<path d="M12 4v11.5M7.5 11l4.5 4.5 4.5-4.5M4.5 15v3a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-3"/>"##);
icon!(EDIT, r##"<path d="M4 20h4L19.5 8.5a2.1 2.1 0 0 0-3-3L5 17z"/><path d="M14.5 7.5l3 3"/>"##);
icon!(TRASH, r##"<path d="M4 7h16M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5M10 11v5M14 11v5"/>"##);
icon!(HELP, r##"<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.6 2.6 0 1 1 3.8 2.3c-.8.5-1.3 1-1.3 2M12 17h.01"/>"##);
icon!(REFRESH, r##"<path d="M20 11a8 8 0 0 0-14.5-4L4 8.5M4 4v4.5h4.5M4 13a8 8 0 0 0 14.5 4l1.5-1.5M20 20v-4.5h-4.5"/>"##);
icon!(RESET, r##"<path d="M4 4v5h5M4.6 9a8 8 0 1 1-.4 5"/>"##);
icon!(WARNING, r##"<path d="M12 4l9 15.5H3zM12 10v4.5M12 17.2h.01"/>"##);

pub fn icon<'a>(data: &'static [u8], size: f32, color: Color) -> Svg<'a> {
    iced::widget::svg(svg::Handle::from_memory(data))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
}
