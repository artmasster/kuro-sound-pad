"""Draws the picture on the left of the installer's first and last page: the logo and a small set of pads.

    python -I scripts/installer-art.py <out folder>

Writes wizard-100.bmp, -125, -150, -175 and -200: one per screen scale, each drawn at its own size rather than one
bitmap stretched (a stretched logo looks rough). The installer shows them unstretched on a page of the same black, so
the picture has no edge and a few pixels' difference from the page's size does not show.
"""
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONTS = ROOT / "assets" / "fonts"
# The picture at 100 %: narrower than the page's picture column (164 px), which keeps it clear of the text beside it
# at every scale.
W, H = 150, 314
# Drawn this many times larger, then scaled down, for smooth edges.
SUPER = 4

BLACK = (0x0B, 0x0B, 0x0D)
PRINT = (0xEC, 0xEC, 0xEF)
LABEL = (0x85, 0x85, 0x8E)
AMBER = (0xFF, 0xB6, 0x27)
ROWS = [(0xFF, 0xB0, 0x20), (0xFF, 0x5E, 0x4F), (0x22, 0xD3, 0xEE), (0xA8, 0x68, 0xFF)]
PAD_TOP, PAD_BOTTOM, PAD_LINE = (0x2E, 0x2F, 0x35), (0x1D, 0x1E, 0x22), (0x3B, 0x3C, 0x42)
# The pads that are "playing".
LIT = {(0, 1), (2, 2)}


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def spaced(draw, x, y, text, font, fill, gap):
    for ch in text:
        draw.text((x, y), ch, font=font, fill=fill)
        x += draw.textlength(ch, font=font) + gap
    return x


def spaced_width(draw, text, font, gap):
    return sum(draw.textlength(ch, font=font) for ch in text) + gap * (len(text) - 1)


def pad(img, x, y, size, color, lit, k):
    """One pad: a body that fades downwards, an edge, and its light strip."""
    top, bottom, line = PAD_TOP, PAD_BOTTOM, PAD_LINE
    if lit:
        top, bottom, line = mix(top, color, 0.42), mix(bottom, color, 0.18), color
        ring = Image.new("RGBA", img.size, (0, 0, 0, 0))
        ImageDraw.Draw(ring).rounded_rectangle((x - 3 * k, y - 3 * k, x + size + 3 * k, y + size + 3 * k), 8 * k, fill=color + (90,))
        img.alpha_composite(ring)
    body = Image.new("RGBA", (size, size))
    for row in range(size):
        ImageDraw.Draw(body).line((0, row, size, row), fill=mix(top, bottom, row / max(size - 1, 1)) + (255,))
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), 5 * k, fill=255)
    img.paste(body, (x, y), mask)
    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle((x, y, x + size - 1, y + size - 1), 5 * k, outline=line + (255,), width=max(1, round((1.6 if lit else 1) * k)))
    strip = (255, 255, 255) if lit else color
    draw.rounded_rectangle((x + 5 * k, y + size - 7 * k, x + size - 5 * k, y + size - 4 * k), 2 * k, fill=strip + (255,))


def render(scale):
    k = round(scale * SUPER * 1000) / 1000
    w, h = round(W * scale), round(H * scale)
    img = Image.new("RGBA", (round(W * k), round(H * k)), BLACK + (255,))
    draw = ImageDraw.Draw(img)
    cx = W * k / 2

    # The mark: five bars, like the app's own icon.
    bars = [(3, 10, 14), (7.5, 6, 18), (12, 3, 21), (16.5, 7, 17), (21, 10, 14)]
    unit = 34 * k / 24
    left, top = cx - 12 * unit, 38 * k
    half = 1.25 * unit
    for bx, y0, y1 in bars:
        draw.rounded_rectangle((left + bx * unit - half, top + y0 * unit - half, left + bx * unit + half, top + y1 * unit + half), half, fill=AMBER + (255,))

    bold = ImageFont.truetype(str(FONTS / "ChakraPetch-Bold.ttf"), round(35 * k))
    semi = ImageFont.truetype(str(FONTS / "ChakraPetch-SemiBold.ttf"), round(11.5 * k))
    name_w = spaced_width(draw, "KURO", bold, 1.5 * k)
    spaced(draw, cx - name_w / 2, 78 * k, "KURO", bold, PRINT + (255,), 1.5 * k)
    sub_w = spaced_width(draw, "SOUND PAD", semi, 2.6 * k)
    spaced(draw, cx - sub_w / 2, 121 * k, "SOUND PAD", semi, LABEL + (255,), 2.6 * k)

    # Sixteen pads, each row with its own light.
    size, gap = round(26 * k), round(5 * k)
    grid = 4 * size + 3 * gap
    gx, gy = round(cx - grid / 2), round(160 * k)
    for r in range(4):
        for c in range(4):
            pad(img, gx + c * (size + gap), gy + r * (size + gap), size, ROWS[r], (r, c) in LIT, k)

    return img.convert("RGB").resize((w, h), Image.LANCZOS)


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for percent in (100, 125, 150, 175, 200):
        picture = render(percent / 100)
        picture.save(out / f"wizard-{percent}.bmp")
        print(f"wizard-{percent}.bmp {picture.width}x{picture.height}")


main()
