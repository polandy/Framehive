# SPDX-FileCopyrightText: 2026 Framehive contributors
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate the Framehive logo files in this directory.

The SVG files next to this script are outputs: edit this script, not them.

    uv run --no-project --with fonttools==4.66.1 --with uharfbuzz==0.56.3 \
        assets/brand/generate.py

The wordmark is Sora SemiBold (OFL 1.1), outlined to paths so the logos render
without the font. The font is fetched from a pinned google/fonts commit and
checked against its SHA-256; nothing else touches the network.
"""

import hashlib
import io
import math
import pathlib
import urllib.request

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
import uharfbuzz

OUTPUT_DIRECTORY = pathlib.Path(__file__).resolve().parent

HONEY = "#F4A51C"
INK = "#17150F"
PAPER = "#FBF8F1"

FONT_URL = (
    "https://raw.githubusercontent.com/google/fonts/"
    "69109d1f48319d1d474a8d2f70018efc2346a3a7/ofl/sora/Sora%5Bwght%5D.ttf"
)
FONT_SHA256 = "84ff7096ae3ec6c8be47d906d1a0ba4de7f2ce78c615275c77301964a316e16c"
WORDMARK_TEXT = "framehive"
WORDMARK_WEIGHT = 600
WORDMARK_TRACKING_EM = -0.03

# The mark is drawn in a 200 x 200 box around its centre.
MARK_BOX = 200


class MarkStyle:
    """Geometry of the mark: six viewfinder corners around a honeycomb cell."""

    def __init__(self, corner_radius, arm_ratio, stroke, cell_radius, cell_stroke):
        self.corner_radius = corner_radius
        self.arm_ratio = arm_ratio
        self.stroke = stroke
        self.cell_radius = cell_radius
        self.cell_stroke = cell_stroke


# Regular sizes, and a heavier cut that stays legible at 16-32 px.
REGULAR = MarkStyle(corner_radius=80, arm_ratio=0.30, stroke=15, cell_radius=30, cell_stroke=8)
SMALL = MarkStyle(corner_radius=76, arm_ratio=0.34, stroke=22, cell_radius=34, cell_stroke=10)


def number(value):
    """Format a coordinate compactly: at most two decimals, no trailing zeros."""
    text = f"{value:.2f}".rstrip("0").rstrip(".")
    return "0" if text == "-0" else text


def hexagon(radius, centre):
    """Corners of a pointy-top hexagon, clockwise from the top."""
    cx, cy = centre
    return [
        (cx + radius * math.cos(math.radians(-90 + 60 * i)),
         cy + radius * math.sin(math.radians(-90 + 60 * i)))
        for i in range(6)
    ]


def points(corners):
    return " ".join(f"{number(x)},{number(y)}" for x, y in corners)


def viewfinder_corners(style, centre):
    """One chevron per hexagon corner, each arm a fraction of the edge."""
    corners = hexagon(style.corner_radius, centre)
    path = []
    for i, corner in enumerate(corners):
        previous, following = corners[i - 1], corners[(i + 1) % 6]
        start = lerp(corner, previous, style.arm_ratio)
        end = lerp(corner, following, style.arm_ratio)
        path.append(
            f"M{number(start[0])} {number(start[1])}"
            f"L{number(corner[0])} {number(corner[1])}"
            f"L{number(end[0])} {number(end[1])}"
        )
    return "".join(path)


def lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def mark(style, frame_colour, cell_colour, offset=(0, 0), scale=1.0):
    """The mark as an SVG group, its box placed at `offset` and scaled."""
    centre = (MARK_BOX / 2, MARK_BOX / 2)
    transform = f"translate({number(offset[0])} {number(offset[1])})"
    if scale != 1.0:
        transform += f" scale({number(scale)})"
    return (
        f'<g transform="{transform}">'
        f'<path d="{viewfinder_corners(style, centre)}" fill="none" stroke="{frame_colour}" '
        f'stroke-width="{style.stroke}" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<polygon points="{points(hexagon(style.cell_radius, centre))}" fill="{cell_colour}" '
        f'stroke="{cell_colour}" stroke-width="{style.cell_stroke}" stroke-linejoin="round"/>'
        "</g>"
    )


def load_font():
    with urllib.request.urlopen(FONT_URL) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != FONT_SHA256:
        raise SystemExit(f"font hash mismatch: {digest}")
    font = instantiateVariableFont(TTFont(io.BytesIO(data)), {"wght": WORDMARK_WEIGHT})
    buffer = io.BytesIO()
    font.save(buffer)
    return font, buffer.getvalue()


class Wordmark:
    """The outlined wordmark: one path, its size and its baseline position."""

    def __init__(self, path, width, cap_top, font_size):
        self.path = path
        self.width = width
        self.cap_top = cap_top
        self.font_size = font_size


def outline_wordmark(font, font_bytes, font_size):
    """Shape the text with HarfBuzz and outline it, baseline at y = 0."""
    units = font["head"].unitsPerEm
    scale = font_size / units
    hb_font = uharfbuzz.Font(uharfbuzz.Face(font_bytes))
    buffer = uharfbuzz.Buffer()
    buffer.add_str(WORDMARK_TEXT)
    buffer.guess_segment_properties()
    uharfbuzz.shape(hb_font, buffer, {"kern": True, "liga": False})

    glyph_set = font.getGlyphSet()
    glyph_order = font.getGlyphOrder()
    tracking = WORDMARK_TRACKING_EM * units
    pen = SVGPathPen(glyph_set, ntos=number)
    x = 0.0
    infos, positions = buffer.glyph_infos, buffer.glyph_positions
    for index, (info, position) in enumerate(zip(infos, positions)):
        name = glyph_order[info.codepoint]
        glyph_pen = TransformPen(pen, (scale, 0, 0, -scale, (x + position.x_offset) * scale, 0))
        glyph_set[name].draw(glyph_pen)
        x += position.x_advance
        if index < len(infos) - 1:
            x += tracking
    ascender_top = font["OS/2"].sCapHeight * scale
    return Wordmark(pen.getCommands(), x * scale, ascender_top, font_size)


# REUSE-IgnoreStart
SPDX_HEADER = (
    "<!-- SPDX-FileCopyrightText: 2026 Framehive contributors -->\n"
    "<!-- SPDX-License-Identifier: GPL-3.0-or-later -->\n"
    "<!-- Generated by assets/brand/generate.py; edit the script, not this file. -->\n"
)
# REUSE-IgnoreEnd


def svg_document(width, height, title, body, extra=""):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {number(width)} {number(height)}" '
        f'width="{number(width)}" height="{number(height)}" role="img" aria-label="{title}">\n'
        f"{SPDX_HEADER}<title>{title}</title>\n{extra}{body}\n</svg>\n"
    )


def horizontal_lockup(wordmark, frame_colour, text_colour):
    """Mark left, wordmark right, the wordmark's cap height centred on the mark."""
    gap = 50
    margin = 12
    mark_scale = 1.0
    height = MARK_BOX + 2 * margin
    baseline = height / 2 + wordmark.cap_top / 2
    text_x = margin + MARK_BOX * mark_scale + gap
    width = text_x + wordmark.width + margin
    body = (
        mark(REGULAR, frame_colour, HONEY, offset=(margin, margin))
        + f'<path transform="translate({number(text_x)} {number(baseline)})" '
        f'd="{wordmark.path}" fill="{text_colour}"/>'
    )
    return width, height, body


def app_icon(size, full_bleed):
    """Ink tile with the light mark; full bleed keeps the mark in the safe zone."""
    if full_bleed:
        tile = f'<rect width="{size}" height="{size}" fill="{INK}"/>'
        mark_size = size * 0.62
    else:
        radius = size * 0.225
        tile = f'<rect width="{size}" height="{size}" rx="{number(radius)}" fill="{INK}"/>'
        mark_size = size * 0.72
    offset = (size - mark_size) / 2
    return tile + mark(REGULAR, PAPER, HONEY, offset=(offset, offset), scale=mark_size / MARK_BOX)


def favicon():
    """The heavy cut; its frame follows the browser's light or dark scheme."""
    style = (
        "<style>.frame{stroke:" + INK + "}"
        "@media (prefers-color-scheme: dark){.frame{stroke:" + PAPER + "}}</style>\n"
    )
    body = mark(SMALL, INK, HONEY).replace(f'stroke="{INK}"', 'class="frame"', 1)
    return svg_document(MARK_BOX, MARK_BOX, "Framehive", body, extra=style)


def write(name, content):
    (OUTPUT_DIRECTORY / name).write_text(content, encoding="utf-8")
    print(f"wrote {name}")


def main():
    font, font_bytes = load_font()
    wordmark = outline_wordmark(font, font_bytes, font_size=140)

    write("logo-mark.svg", svg_document(MARK_BOX, MARK_BOX, "Framehive", mark(REGULAR, INK, HONEY)))
    write("logo-mark-dark.svg", svg_document(MARK_BOX, MARK_BOX, "Framehive", mark(REGULAR, PAPER, HONEY)))
    for scheme, frame_colour, text_colour in (("light", INK, INK), ("dark", PAPER, PAPER)):
        width, height, body = horizontal_lockup(wordmark, frame_colour, text_colour)
        write(f"logo-horizontal-{scheme}.svg", svg_document(width, height, "Framehive", body))
    write("app-icon.svg", svg_document(512, 512, "Framehive", app_icon(512, full_bleed=False)))
    write("app-icon-maskable.svg", svg_document(512, 512, "Framehive", app_icon(512, full_bleed=True)))
    write("favicon.svg", favicon())


if __name__ == "__main__":
    main()
