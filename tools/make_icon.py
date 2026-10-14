"""Render the app icon (assets/icon.png) in the Swiss style of the UI.

A "C" split down the middle and inverted across ink and paper (the contrast
pair the app measures), with the UI's single red accent. Uses the bundled
Inter Bold.

    python3 -m pip install pillow
    python3 tools/make_icon.py
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "assets/fonts/Inter-Bold.ttf"
OUT = ROOT / "assets/icon.png"

# Palette, matching src/theme.rs (light).
PAPER = (0xF6, 0xF5, 0xF1, 255)
INK = (0x11, 0x11, 0x11, 255)
RULE = (0xC9, 0xC7, 0xC0, 255)
ACCENT = (0xD0, 0x02, 0x1B, 255)

SIZE = 1024
SS = 4  # supersampling factor for anti-aliasing
# Apple's macOS icon grid: an 824 px body centred on a 1024 px canvas.
BODY = 824
INSET = (SIZE - BODY) // 2


def squircle(x0, y0, size, n=5.0, steps=720):
    """Superellipse approximating Apple's continuous-corner rounded square."""
    r = size / 2
    cx, cy = x0 + r, y0 + r
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        c, s = math.cos(t), math.sin(t)
        x = cx + r * math.copysign(abs(c) ** (2 / n), c)
        y = cy + r * math.copysign(abs(s) ** (2 / n), s)
        pts.append((x, y))
    return pts


def main():
    S = SIZE * SS
    k = lambda v: v * SS  # noqa: E731
    body = (k(INSET), k(INSET), k(INSET + BODY), k(INSET + BODY))

    # Shape mask for the icon body.
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).polygon(squircle(k(INSET), k(INSET), k(BODY)), fill=255)

    # Artwork, drawn edge to edge and clipped by the mask afterwards.
    art = Image.new("RGBA", (S, S), PAPER)
    d = ImageDraw.Draw(art)
    mid = k(SIZE // 2)
    d.rectangle((mid, body[1], body[2], body[3]), fill=INK)

    # A "C" centred on the split, inverted on each side: ink on paper to the
    # left, paper on ink to the right.
    font = ImageFont.truetype(str(FONT), k(720))
    glyph = Image.new("L", (S, S), 0)
    g = ImageDraw.Draw(glyph)
    box = g.textbbox((0, 0), "C", font=font, anchor="ls")
    y = k(SIZE // 2) - (box[1] + box[3]) / 2
    # Split the C into two equal halves of ink, not at its bounding-box centre:
    # the open side makes a C left-heavy, so a box-centred cut leaves more
    # letter on the left. Find the column where half the ink is on each side.
    g.text((0, y), "C", font=font, fill=255, anchor="ls")
    cols = list(glyph.resize((S, 1), Image.BOX).getdata())  # per-column ink
    half, run, cut = sum(cols) / 2, 0.0, 0
    for cut, c in enumerate(cols):
        run += c
        if run >= half:
            break
    glyph = Image.new("L", (S, S), 0)
    ImageDraw.Draw(glyph).text((mid - cut, y), "C", font=font, fill=255, anchor="ls")
    inverted = Image.new("RGBA", (S, S), INK)
    ImageDraw.Draw(inverted).rectangle((mid, body[1], body[2], body[3]), fill=PAPER)
    art.paste(inverted, (0, 0), glyph)

    # The single red accent, in the bottom-left corner of the paper half.
    sq, margin = k(56), k(96)
    left, bottom = body[0] + margin, body[3] - margin
    d.rectangle((left, bottom - sq, left + sq, bottom), fill=ACCENT)

    icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    icon.paste(art, (0, 0), mask)

    # Hairline edge so the paper side doesn't dissolve into light backgrounds.
    edge = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(edge).line(
        squircle(k(INSET), k(INSET), k(BODY)) + [squircle(k(INSET), k(INSET), k(BODY))[0]],
        fill=RULE,
        width=k(4),
    )
    icon = Image.alpha_composite(icon, edge)

    icon.resize((SIZE, SIZE), Image.LANCZOS).save(OUT)
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
