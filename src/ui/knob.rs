//! The master volume knob, drawn as a picture for each value: a ring of lights, a body lit from above, a pointer.
//! The window turns it (drag up and down, or the wheel); this only draws.
use std::f32::consts::PI;
use std::fmt::Write;

/// The ring starts at the lower left and sweeps three quarters of a turn clockwise.
const START: f32 = PI * 0.75;
const SWEEP: f32 = PI * 1.5;
const LIGHTS: usize = 23;

/// An SVG of the knob at `value` (0 to 100). `hot` lifts it a little: the cursor is on it, or it is being turned.
pub fn svg(value: u8, hot: bool) -> Vec<u8> {
    let t = value.min(100) as f32 / 100.0;
    let (c, ring, body) = (36.0, 32.5, 24.0);
    let mut s = String::with_capacity(4096);
    s.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 72"><defs>"##);
    let (top, bottom) = if hot { ("#62636b", "#222327") } else { ("#52535a", "#1a1b1e") };
    let _ = write!(
        s,
        r##"<linearGradient id="b" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>"##
    );
    s.push_str(r##"<linearGradient id="c" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#232428"/><stop offset="1" stop-color="#34353a"/></linearGradient>"##);
    s.push_str(r##"<radialGradient id="s" cx="0.5" cy="0.5" r="0.5"><stop offset="0.7" stop-color="#000" stop-opacity="0.85"/><stop offset="1" stop-color="#000" stop-opacity="0"/></radialGradient></defs>"##);

    // The lights: green, then amber, then red at the top of the range. Unlit ones stay faintly visible.
    let lit = (t * LIGHTS as f32).round() as usize;
    for i in 0..LIGHTS {
        let a = START + SWEEP * i as f32 / (LIGHTS - 1) as f32;
        let color = if i >= LIGHTS - 3 { "#ff4438" } else if i >= LIGHTS - 8 { "#ffb627" } else { "#3ddc84" };
        let (x, y) = (c + ring * a.cos(), c + ring * a.sin());
        if i < lit {
            let _ = write!(s, r##"<circle cx="{x:.2}" cy="{y:.2}" r="2.6" fill="{color}" opacity="0.28"/><circle cx="{x:.2}" cy="{y:.2}" r="1.7" fill="{color}"/>"##);
        } else {
            let _ = write!(s, r##"<circle cx="{x:.2}" cy="{y:.2}" r="1.5" fill="{color}" opacity="0.2"/>"##);
        }
    }

    // A soft shadow under the knob, the body, the flat cap on top.
    let _ = write!(s, r##"<circle cx="{c}" cy="{}" r="{}" fill="url(#s)"/>"##, c + 3.0, body + 5.0);
    let _ = write!(s, r##"<circle cx="{c}" cy="{c}" r="{body}" fill="#060607"/><circle cx="{c}" cy="{c}" r="{}" fill="url(#b)"/>"##, body - 1.2);
    let _ = write!(s, r##"<circle cx="{c}" cy="{c}" r="{}" fill="url(#c)" stroke="#0c0c0e" stroke-width="0.8"/>"##, body - 6.0);

    // The pointer.
    let a = START + SWEEP * t;
    let (dx, dy) = (a.cos(), a.sin());
    let _ = write!(
        s,
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#fff" stroke-width="2.6" stroke-linecap="round"/></svg>"##,
        c + dx * body * 0.38,
        c + dy * body * 0.38,
        c + dx * (body - 4.5),
        c + dy * (body - 4.5),
    );
    s.into_bytes()
}

/// The value after dragging the knob `dy` pixels up from `from`.
pub fn turned(from: u8, dy: f32) -> u8 {
    (from as f32 + dy * 0.7).round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning() {
        assert_eq!(turned(50, 0.0), 50);
        assert_eq!(turned(50, 1000.0), 100);
        assert_eq!(turned(50, -1000.0), 0);
        assert!(turned(50, 10.0) > 50);
    }

    #[test]
    fn every_value_draws() {
        for v in [0, 1, 50, 99, 100, 255] {
            let svg = String::from_utf8(svg(v, v % 2 == 0)).unwrap();
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
            assert!(!svg.contains("NaN"));
        }
    }
}
