#!/usr/bin/env python3
"""Writes the "where the sound goes" diagram once per README language.

Run from the repository root:  python docs/images/make-diagram.py
Add a language by adding one entry to TEXT and linking the new file from its README.
"""
from pathlib import Path
from xml.sax.saxutils import escape

OUT = Path(__file__).parent

# (title, note) for each box, then the two small labels
TEXT = {
    "": {
        "label": "Hotkey or click - Kuro Sound Pad - OUTPUT to your headphones - MONITOR to VB-Cable - friends in Discord",
        "press": ("Hotkey or click", "even from inside a game"),
        "app": ("Kuro Sound Pad", "16 pads"),
        "output": ("OUTPUT", "headphones or speakers"),
        "you": ("You", "hear it at once"),
        "monitor": ("MONITOR", "VB-Cable · CABLE Input"),
        "friends": ("Friends in Discord", "mic: CABLE Output"),
        "optional": "optional",
    },
    ".th": {
        "label": "ปุ่มลัดหรือคลิก - Kuro Sound Pad - OUTPUT ไปหูฟังของคุณ - MONITOR ไป VB-Cable - เพื่อนใน Discord",
        "press": ("ปุ่มลัดหรือคลิก", "อยู่ในเกมก็กดได้"),
        "app": ("Kuro Sound Pad", "16 แผง"),
        "output": ("OUTPUT", "หูฟังหรือลำโพง"),
        "you": ("คุณ", "ได้ยินทันที"),
        "monitor": ("MONITOR", "VB-Cable · CABLE Input"),
        "friends": ("เพื่อนใน Discord", "ไมค์: CABLE Output"),
        "optional": "ไม่บังคับ",
    },
}

FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans', 'Noto Sans Thai', 'Leelawadee UI', sans-serif"
BG, CARD, LINE = "#0c0c0e", "#18181b", "#303036"
MAIN, DIM = "#f4f4f5", "#a1a1aa"
AMBER, RED, TEAL, PURPLE = "#f5a623", "#e5484d", "#2dd4bf", "#a78bfa"

W, H = 980, 344
BW, BH = 196, 108  # box size


def box(x, y, title, note, icon, accent=LINE):
    cx = x + BW / 2
    return (
        f'<rect x="{x}" y="{y}" width="{BW}" height="{BH}" rx="14" fill="{CARD}" stroke="{accent}" stroke-width="1.5"/>'
        f'<g transform="translate({cx} {y + 30})">{icon}</g>'
        f'<text x="{cx}" y="{y + 72}" font-size="16" font-weight="700" fill="{MAIN}" text-anchor="middle">{escape(title)}</text>'
        f'<text x="{cx}" y="{y + 93}" font-size="13" fill="{DIM}" text-anchor="middle">{escape(note)}</text>'
    )


def arrow(path, tip_x, tip_y, color=DIM, dash=False):
    d = ' stroke-dasharray="6 6"' if dash else ""
    return (
        f'<g fill="none" stroke="{color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">'
        f'<path d="{path}"{d}/><path d="M{tip_x - 8} {tip_y - 7} l8 7 l-8 7"/></g>'
    )


def stroke(body, color):
    return f'<g fill="none" stroke="{color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">{body}</g>'


ICON_KEY = (
    f'<rect x="-17" y="-17" width="34" height="34" rx="8" fill="none" stroke="{AMBER}" stroke-width="2.5"/>'
    f'<rect x="-9" y="-11" width="18" height="16" rx="4" fill="{AMBER}"/>'
)
ICON_PADS = (
    f'<rect x="-17" y="-17" width="15" height="15" rx="4" fill="{AMBER}"/>'
    f'<rect x="2" y="-17" width="15" height="15" rx="4" fill="{RED}"/>'
    f'<rect x="-17" y="2" width="15" height="15" rx="4" fill="{TEAL}"/>'
    f'<rect x="2" y="2" width="15" height="15" rx="4" fill="{PURPLE}"/>'
)
ICON_PHONES = (
    stroke('<path d="M-15 6 v-6 a15 15 0 0 1 30 0 v6"/>', AMBER)
    + f'<rect x="-19" y="3" width="9" height="14" rx="4" fill="{AMBER}"/><rect x="10" y="3" width="9" height="14" rx="4" fill="{AMBER}"/>'
)
ICON_EAR = (
    f'<circle cx="-12" cy="0" r="5" fill="{AMBER}"/>'
    + stroke('<path d="M-2 -8 a11 11 0 0 1 0 16"/><path d="M6 -14 a19 19 0 0 1 0 28"/>', AMBER)
)
ICON_CABLE = (
    stroke('<path d="M-11 0 C-3 -16 3 16 11 0"/>', TEAL)
    + f'<circle cx="-15" cy="0" r="6" fill="{TEAL}"/><circle cx="15" cy="0" r="6" fill="{TEAL}"/>'
)
ICON_CHAT = (
    f'<path d="M-18 -14 h36 a6 6 0 0 1 6 6 v14 a6 6 0 0 1 -6 6 h-20 l-10 9 v-9 h-6 a6 6 0 0 1 -6 -6 v-14 a6 6 0 0 1 6 -6 z" '
    f'fill="none" stroke="{TEAL}" stroke-width="2.5" stroke-linejoin="round"/>'
    f'<circle cx="-9" cy="-1" r="2.6" fill="{TEAL}"/><circle cx="0" cy="-1" r="2.6" fill="{TEAL}"/><circle cx="9" cy="-1" r="2.6" fill="{TEAL}"/>'
)


def diagram(t):
    x1, x2, x3, x4 = 24, 270, 516, 762
    top, mid, bottom = 34, 111, 188
    my = mid + BH / 2  # middle of the first two boxes
    ty, by = top + BH / 2, bottom + BH / 2
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" font-family="{FONT}" '
        f'role="img" aria-label="{escape(t["label"])}">',
        f'<rect width="{W}" height="{H}" rx="18" fill="{BG}"/>',
        # the optional second route
        f'<rect x="{x3 - 14}" y="{bottom - 14}" width="{x4 + BW - x3 + 28}" height="{BH + 28}" rx="16" fill="none" '
        f'stroke="{LINE}" stroke-width="2" stroke-dasharray="7 7"/>',
        f'<text x="{x4 + BW + 6}" y="{bottom + BH + 33}" font-size="13" fill="{DIM}" text-anchor="end">{escape(t["optional"])}</text>',
        arrow(f"M{x1 + BW + 8} {my} H{x2 - 10}", x2 - 8, my),
        arrow(f"M{x2 + BW + 8} {my - 14} H{x2 + BW + 26} V{ty} H{x3 - 10}", x3 - 8, ty, AMBER),
        arrow(f"M{x2 + BW + 8} {my + 14} H{x2 + BW + 26} V{by} H{x3 - 24}", x3 - 22, by, TEAL),
        arrow(f"M{x3 + BW + 8} {ty} H{x4 - 10}", x4 - 8, ty, AMBER),
        arrow(f"M{x3 + BW + 8} {by} H{x4 - 10}", x4 - 8, by, TEAL),
        box(x1, mid, *t["press"], ICON_KEY),
        box(x2, mid, *t["app"], ICON_PADS, AMBER),
        box(x3, top, *t["output"], ICON_PHONES),
        box(x4, top, *t["you"], ICON_EAR),
        box(x3, bottom, *t["monitor"], ICON_CABLE),
        box(x4, bottom, *t["friends"], ICON_CHAT),
        "</svg>",
    ]
    return "\n".join(parts) + "\n"


for suffix, text in TEXT.items():
    path = OUT / f"how-it-works{suffix}.svg"
    path.write_text(diagram(text), encoding="utf-8", newline="\n")
    print("wrote", path)
