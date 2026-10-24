#!/usr/bin/env python3
"""Generate the first Asena volumetric application icons for user review.

These are layered SVG illustrations (see docs/ICONOGRAPHY.md): every object has
a visible body thickness, a shared key light from the upper left, a soft fill
light, contact shading where parts overlap and one ground shadow. The first four
icons are the review set; org.asenalinux.App is the neutral fallback used by
applications whose icon has not been approved yet.

Folders and the Files and Settings icons follow the accent color. They are
drawn in teal; every other accent gets an `Asena-<accent>` icon theme that
inherits Asena and holds the same drawings with their colored layers moved to
that accent (Yaru uses the same layout for its accent variants).

    python3 scripts/icons/apps.py
"""

from __future__ import annotations

import colorsys
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ICONS_DIR = ROOT / "data/icons"
# hicolor serves every desktop; the Asena theme copy keeps these icons ahead of
# inherited themes that ship their own generic variants.
OUTPUTS = (ICONS_DIR / "hicolor/scalable/apps", ICONS_DIR / "Asena/scalable/apps")

# Matches asena_core::theme::ACCENTS; tests/test_icons.py checks both.
ACCENTS = {
    "blue": "#3584e4",
    "teal": "#2190a4",
    "green": "#3a944a",
    "yellow": "#c88800",
    "orange": "#ed5b00",
    "red": "#e62d42",
    "pink": "#d56199",
    "purple": "#9141ac",
    "slate": "#6f8396",
}
BASE_ACCENT = "teal"
# Main tone of the teal drawings (the Files front cover); other accents put
# their own hue here.
DESIGN_TONE = "#25a7a6"

# Full-color folders, with the line icon pressed into the front cover.
PLACES = {
    "folder": None,
    "folder-open": None,
    "folder-drag-accept": None,
    "folder-documents": "x-office-document-symbolic",
    "folder-download": "go-down-symbolic",
    "folder-music": "audio-x-generic-symbolic",
    "folder-pictures": "image-x-generic-symbolic",
    "folder-videos": "video-x-generic-symbolic",
    "folder-templates": "document-new-symbolic",
    "folder-publicshare": "emblem-shared-symbolic",
    "folder-remote": "network-workgroup-symbolic",
    "user-home": "user-home-symbolic",
    "user-desktop": "video-display-symbolic",
}


def hls(color: str) -> tuple[float, float, float]:
    if len(color) == 4:
        color = "#" + "".join(channel * 2 for channel in color[1:])
    red, green, blue = (int(color[index:index + 2], 16) / 255 for index in (1, 3, 5))
    return colorsys.rgb_to_hls(red, green, blue)


def tint(color: str, accent: str) -> str:
    """Moves a teal design color to `accent`. The drawing's main tone takes the
    accent's hue, saturation scales with the accents' and lightness keeps the
    shading."""
    if accent == BASE_ACCENT:
        return color
    hue, lightness, saturation = hls(color)
    tone_hue, _, _ = hls(DESIGN_TONE)
    _, _, base_saturation = hls(ACCENTS[BASE_ACCENT])
    accent_hue, _, accent_saturation = hls(ACCENTS[accent])
    hue = (hue + accent_hue - tone_hue) % 1.0
    saturation = min(1.0, saturation * accent_saturation / base_saturation)
    rgb = colorsys.hls_to_rgb(hue, lightness, saturation)
    return "#" + "".join(f"{round(channel * 255):02x}" for channel in rgb)


def glyph(name: str) -> str:
    """The shapes of an Asena line icon, without its root element."""
    path = next((ICONS_DIR / "Asena/symbolic").glob(f"*/{name}.svg"))
    text = path.read_text(encoding="utf-8")
    inner = text[text.index(">") + 1:text.rindex("</svg>")]
    # Embedded shapes take the group's fill; ids would collide in the folder.
    if any(token in inner for token in ("<defs", "clip", "id=", "fill=")):
        raise ValueError(f"{name} cannot be embedded as a plain glyph")
    return inner


def f(value: float) -> str:
    text = f"{value:.2f}".rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def svg(body: str, defs: str) -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">'
        f"<defs>{defs}</defs>{body}</svg>\n"
    )


def linear(name: str, stops: list[tuple[float, str, float]], x1=0, y1=0, x2=0, y2=1,
           user: bool = False) -> str:
    """With `user`, the coordinates are canvas units, so several shapes share
    one continuous gradient."""
    items = "".join(
        f'<stop offset="{f(o)}" stop-color="{c}" stop-opacity="{f(a)}"/>' for o, c, a in stops
    )
    units = ' gradientUnits="userSpaceOnUse"' if user else ""
    return (f'<linearGradient id="{name}" x1="{f(x1)}" y1="{f(y1)}" x2="{f(x2)}" y2="{f(y2)}"'
            f"{units}>{items}</linearGradient>")


def radial(name: str, stops: list[tuple[float, str, float]], cx=0.5, cy=0.5, r=0.5) -> str:
    items = "".join(
        f'<stop offset="{f(o)}" stop-color="{c}" stop-opacity="{f(a)}"/>' for o, c, a in stops
    )
    return f'<radialGradient id="{name}" cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}">{items}</radialGradient>'


def ground(cx=64.0, cy=114.0, rx=46.0, ry=7.0) -> tuple[str, str]:
    defs = radial("ground", [(0, "#000", 0.34), (0.6, "#000", 0.14), (1, "#000", 0)])
    return f'<ellipse cx="{f(cx)}" cy="{f(cy)}" rx="{f(rx)}" ry="{f(ry)}" fill="url(#ground)"/>', defs


def rr(x, y, w, h, r, fill, extra="") -> str:
    return (f'<rect x="{f(x)}" y="{f(y)}" width="{f(w)}" height="{f(h)}" rx="{f(r)}" '
            f'fill="{fill}"{extra}/>')


def files(accent: str = BASE_ACCENT) -> str:
    def t(color: str) -> str:
        return tint(color, accent)

    shadow, defs = ground(64, 112, 50, 7)
    defs += linear("back", [(0, t("#1d8b90"), 1), (1, t("#0f5c61"), 1)])
    defs += linear("paperA", [(0, "#f7f4ec", 1), (1, "#e2ddd1", 1)])
    defs += linear("paperB", [(0, "#ffffff", 1), (1, "#ecebe5", 1)])
    defs += linear("front", [(0, t("#46cdc6"), 1), (0.55, t("#25a7a6"), 1), (1, t("#18888c"), 1)])
    defs += linear("edge", [(0, t("#0f6a6e"), 1), (1, t("#0a4c50"), 1)])
    defs += linear("cast", [(0, "#000", 0), (1, "#000", 0.22)])
    defs += linear("sheen", [(0, "#fff", 0.38), (0.35, "#fff", 0.0)], 0, 0, 0.6, 1)
    body = shadow
    # Back cover with tab.
    body += ('<path d="M12 33a7 7 0 0 1 7-7h26l6 6h58a7 7 0 0 1 7 7v62a7 7 0 0 1-7 7H19'
             'a7 7 0 0 1-7-7z" fill="url(#back)"/>')
    body += '<path d="M19 26h26l6 6H19z" fill="#fff" opacity="0.12"/>'
    # Paper layers with their own edge thickness.
    body += '<g transform="rotate(-3 64 56)">'
    body += rr(22, 24, 82, 58, 3, "#c9c3b5")
    body += rr(22, 22.5, 82, 58, 3, "url(#paperA)")
    body += "</g>"
    body += '<g transform="rotate(2.5 64 60)">'
    body += rr(26, 31, 76, 54, 3, "#d3d0c6")
    body += rr(26, 29.5, 76, 54, 3, "url(#paperB)")
    body += ('<path d="M34 41h40M34 48h52M34 55h46" stroke="#b9c4c7" stroke-width="2.2" '
             'stroke-linecap="round" opacity="0.7"/>')
    body += "</g>"
    # Shadow the front cover casts onto the papers.
    body += rr(12, 40, 104, 10, 0, "url(#cast)")
    # Front cover: bottom thickness first, then the tilted face.
    body += ('<path d="M13 54h102l-4 51a6 6 0 0 1-6 5.5H23a6 6 0 0 1-6-5.5z" '
             'fill="url(#edge)"/>')
    body += ('<path d="M11 50a5 5 0 0 1 5-5h96a5 5 0 0 1 5 5l-4.2 50.5a6 6 0 0 1-6 5.5H21.2'
             'a6 6 0 0 1-6-5.5z" fill="url(#front)"/>')
    body += ('<path d="M11 50a5 5 0 0 1 5-5h96a5 5 0 0 1 5 5l-4.2 50.5a6 6 0 0 1-6 5.5H21.2'
             'a6 6 0 0 1-6-5.5z" fill="url(#sheen)"/>')
    body += (f'<path d="M16 45.8h96" stroke="{t("#bff3ee")}" stroke-width="1.6" '
             'stroke-linecap="round" opacity="0.85"/>')
    # Pressed label groove.
    body += (f'<path d="M46 62h36" stroke="{t("#137a7e")}" stroke-width="3.2" '
             'stroke-linecap="round" opacity="0.55"/>')
    body += (f'<path d="M46 63.4h36" stroke="{t("#7fe2dc")}" stroke-width="1.2" '
             'stroke-linecap="round" opacity="0.55"/>')
    return svg(body, defs)


def folder(accent: str = BASE_ACCENT, emblem: str | None = None) -> str:
    """A plain folder in the Files icon's materials: covers with thickness and
    one sheet between them. An emblem is pressed into the front cover."""
    def t(color: str) -> str:
        return tint(color, accent)

    shadow, defs = ground(64, 112, 50, 7)
    defs += linear("back", [(0, t("#1d8b90"), 1), (1, t("#0f5c61"), 1)])
    defs += linear("paper", [(0, "#ffffff", 1), (1, "#ecebe5", 1)])
    defs += linear("front", [(0, t("#46cdc6"), 1), (0.55, t("#25a7a6"), 1), (1, t("#18888c"), 1)])
    defs += linear("edge", [(0, t("#0f6a6e"), 1), (1, t("#0a4c50"), 1)])
    defs += linear("cast", [(0, "#000", 0), (1, "#000", 0.22)])
    defs += linear("sheen", [(0, "#fff", 0.38), (0.35, "#fff", 0.0)], 0, 0, 0.6, 1)
    body = shadow
    body += ('<path d="M12 33a7 7 0 0 1 7-7h26l6 6h58a7 7 0 0 1 7 7v62a7 7 0 0 1-7 7H19'
             'a7 7 0 0 1-7-7z" fill="url(#back)"/>')
    body += '<path d="M19 26h26l6 6H19z" fill="#fff" opacity="0.12"/>'
    body += '<g transform="rotate(1.5 64 58)">'
    body += rr(24, 31, 80, 52, 3, "#d3d0c6")
    body += rr(24, 29.5, 80, 52, 3, "url(#paper)")
    body += "</g>"
    body += rr(12, 40, 104, 10, 0, "url(#cast)")
    body += ('<path d="M13 54h102l-4 51a6 6 0 0 1-6 5.5H23a6 6 0 0 1-6-5.5z" '
             'fill="url(#edge)"/>')
    body += ('<path d="M11 50a5 5 0 0 1 5-5h96a5 5 0 0 1 5 5l-4.2 50.5a6 6 0 0 1-6 5.5H21.2'
             'a6 6 0 0 1-6-5.5z" fill="url(#front)"/>')
    body += ('<path d="M11 50a5 5 0 0 1 5-5h96a5 5 0 0 1 5 5l-4.2 50.5a6 6 0 0 1-6 5.5H21.2'
             'a6 6 0 0 1-6-5.5z" fill="url(#sheen)"/>')
    body += (f'<path d="M16 45.8h96" stroke="{t("#bff3ee")}" stroke-width="1.6" '
             'stroke-linecap="round" opacity="0.85"/>')
    if emblem:
        # 16 unit line icon at 40 px, large enough to read in a 32 px list,
        # pressed in like the Files label groove: a light lower edge under a
        # darker recess.
        shapes = glyph(emblem)
        body += (f'<g transform="translate(44 57.6) scale(2.5)" fill="{t("#7fe2dc")}" '
                 f'opacity="0.6">{shapes}</g>')
        body += (f'<g transform="translate(44 56) scale(2.5)" fill="{t("#0f6a6e")}" '
                 f'opacity="0.85">{shapes}</g>')
    return svg(body, defs)


def gear_path(cx: float, cy: float, teeth: int, outer: float, root: float) -> str:
    points = []
    step = 2 * math.pi / teeth
    for index in range(teeth):
        base = index * step - math.pi / 2
        for offset, radius in ((-0.36, root), (-0.2, outer), (0.2, outer), (0.36, root)):
            angle = base + offset * step
            points.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    return "M" + "L".join(f"{f(x)} {f(y)}" for x, y in points) + "Z"


def settings(accent: str = BASE_ACCENT) -> str:
    shadow, defs = ground(64, 115, 48, 6.5)
    defs += linear("plateSide", [(0, "#9aa1ab", 1), (1, "#6f7782", 1)])
    defs += linear("plate", [(0, "#f6f7f9", 1), (1, "#d6dae0", 1)])
    defs += linear("well", [(0, "#7f8792", 1), (0.5, "#bfc5cc", 1), (1, "#eef0f3", 1)])
    defs += linear("gearSide", [(0, "#4b5361", 1), (1, "#2c323c", 1)])
    defs += linear("gear", [(0, "#f2f4f7", 1), (0.5, "#c3c9d1", 1), (1, "#8e97a3", 1)],
                   0, 0, 0.7, 1)
    defs += linear("hub", [(0, "#3a414c", 1), (1, "#6c7480", 1)])
    defs += radial("spec", [(0, "#fff", 0.75), (1, "#fff", 0)], 0.32, 0.22, 0.45)
    body = shadow
    body += rr(14, 20, 100, 94, 24, "url(#plateSide)")
    body += rr(14, 14, 100, 94, 24, "url(#plate)")
    body += ('<path d="M38 15h52" stroke="#fff" stroke-width="1.6" stroke-linecap="round" '
             'opacity="0.9"/>')
    # Recessed well; the upper inner wall is in shadow.
    body += '<circle cx="64" cy="61" r="37" fill="url(#well)"/>'
    body += '<circle cx="64" cy="62.5" r="34" fill="#e2e5e9"/>'
    # Gear body with side thickness.
    body += f'<path d="{gear_path(64, 66, 10, 30, 24.5)}" fill="url(#gearSide)" stroke-linejoin="round"/>'
    body += f'<path d="{gear_path(64, 61, 10, 30, 24.5)}" fill="url(#gear)"/>'
    body += f'<path d="{gear_path(64, 61, 10, 30, 24.5)}" fill="url(#spec)"/>'
    # Hub cavity and accent ring.
    body += '<circle cx="64" cy="61" r="12" fill="url(#hub)"/>'
    body += '<circle cx="64" cy="62.2" r="8.6" fill="#1b2027"/>'
    body += (f'<circle cx="64" cy="61" r="12" fill="none" stroke="{tint("#2bb5ad", accent)}" '
             'stroke-width="2.4"/>')
    body += (f'<path d="M55.5 52.5a12 12 0 0 1 17 0" fill="none" stroke="{tint("#a8f0e8", accent)}" '
             'stroke-width="1.2" stroke-linecap="round" opacity="0.8"/>')
    return svg(body, defs)


def terminal() -> str:
    shadow, defs = ground(64, 115, 50, 6.5)
    defs += linear("side", [(0, "#252a32", 1), (1, "#14171c", 1)])
    defs += linear("face", [(0, "#5a626e", 1), (0.12, "#454c57", 1), (1, "#2a2f37", 1)])
    defs += linear("recess", [(0, "#000", 1), (1, "#1c2128", 1)])
    defs += linear("screen", [(0, "#123236", 1), (1, "#06161a", 1)])
    defs += linear("glass", [(0, "#fff", 0.16), (0.45, "#fff", 0.03), (0.46, "#fff", 0)],
                   0, 0, 1, 1)
    body = shadow
    body += rr(10, 22, 108, 88, 15, "url(#side)")
    body += rr(10, 16, 108, 88, 15, "url(#face)")
    body += ('<path d="M24 17h80" stroke="#8d96a3" stroke-width="1.4" stroke-linecap="round" '
             'opacity="0.9"/>')
    body += rr(19, 25, 90, 70, 8, "url(#recess)")
    body += rr(21, 28, 86, 65, 6.5, "url(#screen)")
    # Prompt with a soft glow on the deep content surface.
    glyph = ('<path d="M35 47l12 10-12 10" fill="none" stroke-linecap="round" '
             'stroke-linejoin="round"/><path d="M54 68h16" stroke-linecap="round"/>')
    body += f'<g stroke="#4fe3c8" stroke-width="11" opacity="0.14">{glyph}</g>'
    body += f'<g stroke="#5ff0d2" stroke-width="5.2">{glyph}</g>'
    body += rr(21, 28, 86, 65, 6.5, "url(#glass)")
    body += ('<path d="M27 30h74" stroke="#000" stroke-width="2" stroke-linecap="round" '
             'opacity="0.45"/>')
    return svg(body, defs)


def photos() -> str:
    shadow, defs = ground(64, 115, 48, 6.5)
    defs += linear("woodSide", [(0, "#7a4b2f", 1), (1, "#55321e", 1)])
    defs += linear("wood", [(0, "#d99a6c", 1), (0.5, "#b8764b", 1), (1, "#935a37", 1)],
                   0, 0, 0.9, 1)
    defs += linear("bevel", [(0, "#6b4129", 1), (1, "#c58a5f", 1)])
    defs += linear("mat", [(0, "#f6f2ea", 1), (1, "#e6e0d4", 1)])
    defs += linear("sky", [(0, "#7cc8ef", 1), (1, "#dff3fb", 1)])
    defs += linear("far", [(0, "#6fb9b2", 1), (1, "#4f9690", 1)])
    defs += linear("near", [(0, "#2f8a82", 1), (1, "#1d5f5a", 1)])
    defs += linear("cast", [(0, "#000", 0.28), (1, "#000", 0)])
    body = shadow
    body += rr(16, 20, 100, 92, 10, "url(#woodSide)")
    body += rr(12, 14, 100, 92, 10, "url(#wood)")
    body += ('<path d="M22 15h80" stroke="#f2c7a3" stroke-width="1.4" stroke-linecap="round" '
             'opacity="0.9"/>')
    body += rr(21, 23, 82, 74, 4, "url(#bevel)")
    body += rr(23, 25, 78, 70, 3, "url(#mat)")
    # Recessed image plane.
    body += '<clipPath id="picture"><rect x="32" y="34" width="60" height="52" rx="2"/></clipPath>'
    body += '<g clip-path="url(#picture)">'
    body += rr(32, 34, 60, 52, 2, "url(#sky)")
    body += '<circle cx="76" cy="49" r="7" fill="#ffd36e"/>'
    body += '<circle cx="76" cy="49" r="11" fill="#ffd36e" opacity="0.25"/>'
    body += '<path d="M30 78l20-22 12 12 8-7 24 22v5H30z" fill="url(#far)"/>'
    body += '<path d="M30 88l18-14 14 8 10-6 22 12v4H30z" fill="url(#near)"/>'
    body += rr(32, 34, 60, 6, 0, "url(#cast)")
    body += "</g>"
    body += ('<rect x="32" y="34" width="60" height="52" rx="2" fill="none" stroke="#000" '
             'stroke-opacity="0.18" stroke-width="1"/>')
    return svg(body, defs)


def edge_light(x1: float, x2: float, y: float, color: str, width: float = 1.4,
               opacity: float = 0.9) -> str:
    """The thin key-light highlight along an object's top edge."""
    return (f'<path d="M{f(x1)} {f(y)}H{f(x2)}" stroke="{color}" stroke-width="{f(width)}" '
            f'stroke-linecap="round" opacity="{f(opacity)}"/>')


def star_path(cx: float, cy: float, outer: float, inner: float, points: int = 8) -> str:
    corners = []
    for index in range(points * 2):
        radius = outer if index % 2 == 0 else inner
        angle = math.radians(-90 + index * 180 / points)
        corners.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    return "M" + "L".join(f"{f(x)} {f(y)}" for x, y in corners) + "Z"


def text_editor() -> str:
    shadow, defs = ground(64, 113, 46, 6.5)
    defs += linear("paper", [(0, "#ffffff", 1), (1, "#e9e5dc", 1)])
    defs += linear("bodyTop", [(0, "#ffd27a", 1), (1, "#f4b13e", 1)])
    defs += linear("bodyLow", [(0, "#e3901f", 1), (1, "#b86c14", 1)])
    defs += linear("metal", [(0, "#f1f3f6", 1), (0.5, "#a9b1bc", 1), (1, "#6d7581", 1)])
    defs += linear("eraser", [(0, "#f6a19a", 1), (1, "#d76a63", 1)])
    defs += linear("wood", [(0, "#f6dcb4", 1), (1, "#d7ae78", 1)])
    body = shadow
    body += '<g transform="rotate(-4 64 60)">'
    body += rr(25, 16, 74, 92, 4, "#cdc6b8")
    body += rr(25, 13, 74, 92, 4, "url(#paper)")
    body += edge_light(30, 94, 13.8, "#ffffff", 1.2)
    lines = [(35, 84, 30), (35, 88, 39), (35, 80, 48), (35, 86, 57), (35, 70, 66), (35, 84, 75)]
    body += "".join(f'<path d="M{a} {y}H{b}" stroke="#b6c0c6" stroke-width="2.4" '
                    'stroke-linecap="round"/>' for a, b, y in lines)
    body += "</g>"
    # The pencil lies across the sheet and casts a soft shadow onto it.
    pencil_shadow = '<path d="M24 101L96 44l7 6-72 57z" fill="#000" opacity="0.14"/>'
    body += pencil_shadow
    body += '<g transform="rotate(-38 64 70)">'
    body += '<path d="M12 70l12-7h4v14h-4z" fill="url(#wood)"/>'
    body += '<path d="M12 70l5-3v6z" fill="#353a42"/>'
    body += rr(28, 63, 64, 7, 0, "url(#bodyTop)")
    body += rr(28, 70, 64, 7, 0, "url(#bodyLow)")
    body += '<path d="M28 63.6H92" stroke="#fff3cf" stroke-width="1.2" opacity="0.9"/>'
    body += rr(92, 62.5, 10, 15, 1.5, "url(#metal)")
    body += ('<path d="M95 62.5v15M98.5 62.5v15" stroke="#5f6772" stroke-width="0.9" '
             'opacity="0.6"/>')
    body += '<path d="M102 62.5h5a4 4 0 0 1 4 4v7a4 4 0 0 1-4 4h-5z" fill="url(#eraser)"/>'
    body += "</g>"
    return svg(body, defs)


def notes() -> str:
    shadow, defs = ground(64, 114, 46, 6.5)
    defs += linear("pageSide", [(0, "#e0cf86", 1), (1, "#b99f4d", 1)])
    defs += linear("page", [(0, "#fff7c4", 1), (1, "#f7e07e", 1)])
    defs += linear("ring", [(0, "#f3f5f8", 1), (0.5, "#aeb6c1", 1), (1, "#6b7380", 1)], 0, 0, 1, 0)
    body = shadow
    body += rr(20, 26, 88, 84, 8, "url(#pageSide)")
    body += rr(20, 21, 88, 84, 8, "url(#page)")
    body += '<path d="M36 34v66" stroke="#e86d5f" stroke-width="1.6" opacity="0.75"/>'
    body += "".join(f'<path d="M42 {y}H98" stroke="#d3a842" stroke-width="1.6" '
                    'stroke-linecap="round" opacity="0.6"/>' for y in range(46, 100, 10))
    # Spiral binding: punched holes, then the wire loops over the top edge.
    for x in range(30, 101, 12):
        body += f'<ellipse cx="{x}" cy="30" rx="3.2" ry="2.6" fill="#7c6a2a" opacity="0.75"/>'
        body += rr(x - 2.6, 12, 5.2, 20, 2.6, "url(#ring)")
        body += f'<path d="M{x - 1.2} 13.5v9" stroke="#fff" stroke-width="1" opacity="0.8"/>'
    return svg(body, defs)


def calculator() -> str:
    shadow, defs = ground(64, 115, 44, 6.5)
    defs += linear("side", [(0, "#2a2f37", 1), (1, "#15181d", 1)])
    defs += linear("face", [(0, "#5b636f", 1), (0.1, "#454c57", 1), (1, "#2b3038", 1)])
    defs += linear("screen", [(0, "#143236", 1), (1, "#07181b", 1)])
    defs += linear("key", [(0, "#f4f6f8", 1), (1, "#c9cfd7", 1)])
    defs += linear("keySide", [(0, "#8b939e", 1), (1, "#5f6772", 1)])
    defs += linear("op", [(0, "#ffb26b", 1), (1, "#ec7a22", 1)])
    defs += linear("opSide", [(0, "#b4561a", 1), (1, "#8a3f12", 1)])
    defs += linear("glass", [(0, "#fff", 0.18), (0.5, "#fff", 0.02), (0.51, "#fff", 0)], 0, 0, 1, 1)
    body = shadow
    body += rr(22, 18, 84, 94, 14, "url(#side)")
    body += rr(22, 12, 84, 94, 14, "url(#face)")
    body += edge_light(34, 94, 13, "#8d96a3")
    body += rr(30, 20, 68, 24, 6, "#000")
    body += rr(31.5, 22, 65, 21, 5, "url(#screen)")
    # Seven-segment digits "24" on the right of the display.
    segments = ('<path d="M66 26h7M73 26v6M66 32h7M66 32v6M66 38h7'
                'M80 26v6M80 32h7M87 26v12"/>')
    body += (f'<g fill="none" stroke="#5ff0d2" stroke-width="2.2" stroke-linecap="round" '
             f'stroke-linejoin="round">{segments}</g>')
    body += rr(31.5, 22, 65, 21, 5, "url(#glass)")
    for row in range(4):
        for column in range(4):
            x = 30 + column * 17.5
            y = 52 + row * 13.5
            operator = column == 3
            body += rr(x, y + 2, 14, 10, 3, "url(#opSide)" if operator else "url(#keySide)")
            body += rr(x, y, 14, 10, 3, "url(#op)" if operator else "url(#key)")
            body += (f'<path d="M{f(x + 3)} {f(y + 1)}h8" stroke="#fff" stroke-width="0.9" '
                     'stroke-linecap="round" opacity="0.8"/>')
    return svg(body, defs)


def archive() -> str:
    shadow, defs = ground(64, 114, 50, 6.5)
    defs += linear("boxSide", [(0, "#8a5f35", 1), (1, "#6a4626", 1)])
    defs += linear("box", [(0, "#d8a96e", 1), (1, "#b07b45", 1)])
    defs += linear("lidSide", [(0, "#9b6d3e", 1), (1, "#7a532d", 1)])
    defs += linear("lid", [(0, "#ecc48c", 1), (1, "#cf9c5f", 1)])
    defs += linear("cast", [(0, "#000", 0.3), (1, "#000", 0)])
    defs += linear("pull", [(0, "#f3f5f8", 1), (0.5, "#b5bcc6", 1), (1, "#6f7783", 1)], 0, 0, 1, 0)
    body = shadow
    body += rr(18, 50, 92, 60, 6, "url(#boxSide)")
    body += rr(18, 44, 92, 60, 6, "url(#box)")
    body += rr(18, 44, 92, 9, 0, "url(#cast)")
    body += rr(13, 32, 102, 20, 6, "url(#lidSide)")
    body += rr(13, 27, 102, 20, 6, "url(#lid)")
    body += edge_light(20, 108, 28, "#fbe3bd")
    # Zipper: teeth down the front, a slider and its pull tab.
    body += '<path d="M64 52v48" stroke="#5c3d1f" stroke-width="6" stroke-linecap="round"/>'
    body += ('<path d="M64 54v44" stroke="#d9dee5" stroke-width="5" stroke-dasharray="2 2.2"/>')
    body += rr(58.5, 60, 11, 10, 3, "url(#pull)")
    body += ('<path d="M61 70h6l1.5 14a2 2 0 0 1-2 2.2h-5a2 2 0 0 1-2-2.2z" fill="url(#pull)" '
             'stroke="#5f6772" stroke-width="0.8"/>')
    body += rr(62, 75, 4, 6, 2, "#5f6772")
    return svg(body, defs)


def calendar() -> str:
    shadow, defs = ground(64, 115, 48, 6.5)
    defs += linear("side", [(0, "#a9b1bc", 1), (1, "#7b838e", 1)])
    defs += linear("page", [(0, "#ffffff", 1), (1, "#e8ebef", 1)])
    defs += linear("band", [(0, "#f2735f", 1), (1, "#cf4234", 1)])
    defs += linear("ring", [(0, "#f3f5f8", 1), (0.5, "#aeb6c1", 1), (1, "#6b7380", 1)], 0, 0, 1, 0)
    body = shadow
    body += rr(16, 26, 96, 86, 12, "url(#side)")
    body += rr(16, 20, 96, 86, 12, "url(#page)")
    body += '<path d="M16 32a12 12 0 0 1 12-12h72a12 12 0 0 1 12 12v12H16z" fill="url(#band)"/>'
    body += edge_light(26, 102, 21, "#ffc2b8")
    body += '<path d="M16 44h96" stroke="#000" stroke-width="1.6" opacity="0.18"/>'
    for x in (42, 86):
        body += f'<ellipse cx="{x}" cy="25" rx="3.6" ry="2.8" fill="#7d2219" opacity="0.7"/>'
        body += rr(x - 3, 9, 6, 18, 3, "url(#ring)")
    for row in range(4):
        for column in range(7):
            x = 25.5 + column * 11
            y = 54 + row * 12
            today = (row, column) == (2, 4)
            fill = "url(#band)" if today else "#d9dee5"
            body += rr(x, y, 9, 8, 2, fill)
    return svg(body, defs)


def clock() -> str:
    shadow, defs = ground(64, 113, 44, 6.5)
    defs += linear("bezelSide", [(0, "#5b636e", 1), (1, "#353b44", 1)])
    defs += linear("bezel", [(0, "#f5f7f9", 1), (0.5, "#bcc3cc", 1), (1, "#8b949f", 1)],
                   0, 0, 0.8, 1)
    defs += linear("recess", [(0, "#5a626d", 1), (1, "#cfd5dc", 1)])
    defs += linear("dial", [(0, "#ffffff", 1), (1, "#e6e9ee", 1)])
    defs += radial("glass", [(0, "#fff", 0.55), (1, "#fff", 0)], 0.32, 0.22, 0.5)
    body = shadow
    body += '<circle cx="64" cy="64" r="46" fill="url(#bezelSide)"/>'
    body += '<circle cx="64" cy="59" r="46" fill="url(#bezel)"/>'
    body += '<circle cx="64" cy="59" r="39" fill="url(#recess)"/>'
    body += '<circle cx="64" cy="60.5" r="37" fill="url(#dial)"/>'
    for hour in range(12):
        angle = math.radians(hour * 30 - 90)
        inner = 27 if hour % 3 == 0 else 30
        x1, y1 = 64 + inner * math.cos(angle), 60.5 + inner * math.sin(angle)
        x2, y2 = 64 + 33 * math.cos(angle), 60.5 + 33 * math.sin(angle)
        width = 3 if hour % 3 == 0 else 1.8
        body += (f'<path d="M{f(x1)} {f(y1)}L{f(x2)} {f(y2)}" stroke="#3a414c" '
                 f'stroke-width="{width}" stroke-linecap="round"/>')
    # 10:10, with a raised hour and minute hand and a thin second hand.
    hands = '<path d="M64 60.5L46 50M64 60.5L85 46" stroke-linecap="round"/>'
    body += f'<g stroke="#000" stroke-width="5" opacity="0.18" transform="translate(1 2)">{hands}</g>'
    body += f'<g stroke="#2b3038" stroke-width="4.4">{hands}</g>'
    body += ('<path d="M64 60.5L74 80" stroke="#e5533f" stroke-width="1.8" '
             'stroke-linecap="round"/>')
    body += '<circle cx="64" cy="60.5" r="4" fill="#2b3038"/>'
    body += '<circle cx="64" cy="60.5" r="1.8" fill="#e5533f"/>'
    body += '<circle cx="64" cy="60.5" r="37" fill="url(#glass)"/>'
    return svg(body, defs)


def player() -> str:
    shadow, defs = ground(64, 112, 46, 6.5)
    defs += radial("disc", [(0, "#454b54", 1), (0.55, "#24282e", 1), (1, "#121417", 1)])
    defs += linear("sheen", [(0, "#fff", 0), (0.45, "#fff", 0.22), (0.55, "#fff", 0.22),
                              (1, "#fff", 0)], 0, 0, 1, 1)
    defs += linear("label", [(0, "#ffa36a", 1), (1, "#e2583c", 1)])
    body = shadow
    body += '<circle cx="64" cy="64" r="46" fill="#0b0d10"/>'
    body += '<circle cx="64" cy="60" r="46" fill="url(#disc)"/>'
    body += "".join(f'<circle cx="64" cy="60" r="{r}" fill="none" stroke="#000" '
                    'stroke-width="0.8" opacity="0.45"/>' for r in (24, 28, 32, 36, 40, 43))
    body += "".join(f'<circle cx="64" cy="60" r="{r}" fill="none" stroke="#fff" '
                    'stroke-width="0.6" opacity="0.07"/>' for r in (26, 30, 34, 38, 42))
    body += '<circle cx="64" cy="60" r="46" fill="url(#sheen)"/>'
    body += '<circle cx="64" cy="61" r="19" fill="#000" opacity="0.35"/>'
    body += '<circle cx="64" cy="60" r="19" fill="url(#label)"/>'
    body += '<path d="M51 54a19 19 0 0 1 22-10" stroke="#ffd2b2" stroke-width="1.2" fill="none" opacity="0.8"/>'
    play = '<path d="M58.5 50.5v19l16-9.5z" stroke-linejoin="round"/>'
    body += f'<g fill="#7d2b1c" stroke="#7d2b1c" stroke-width="2" opacity="0.35" transform="translate(0.8 1.4)">{play}</g>'
    body += f'<g fill="#fff8f2" stroke="#fff8f2" stroke-width="2">{play}</g>'
    return svg(body, defs)


def capture() -> str:
    shadow, defs = ground(64, 113, 50, 6.5)
    defs += linear("side", [(0, "#8e96a1", 1), (1, "#5f6772", 1)])
    defs += linear("frame", [(0, "#eef1f4", 1), (1, "#bcc3cc", 1)])
    defs += linear("sky", [(0, "#4f8fd0", 1), (1, "#9fd0ef", 1)])
    defs += linear("hill", [(0, "#3f8f8a", 1), (1, "#25625e", 1)])
    body = shadow
    body += rr(10, 24, 108, 82, 12, "url(#side)")
    body += rr(10, 18, 108, 82, 12, "url(#frame)")
    body += edge_light(22, 106, 19, "#ffffff")
    body += rr(18, 26, 92, 66, 6, "#1c2127")
    body += '<clipPath id="screen"><rect x="20" y="28" width="88" height="62" rx="4.5"/></clipPath>'
    body += '<g clip-path="url(#screen)">'
    body += rr(20, 28, 88, 62, 0, "url(#sky)")
    body += '<path d="M18 80l24-20 16 12 14-10 38 26v4H18z" fill="url(#hill)"/>'
    # Everything outside the selection is dimmed.
    body += ('<path d="M20 28h88v62H20zM40 42v34h48V42z" fill="#000" opacity="0.38" '
             'fill-rule="evenodd"/>')
    body += "</g>"
    body += ('<rect x="40" y="42" width="48" height="34" fill="none" stroke="#fff" '
             'stroke-width="1.4" stroke-dasharray="3 2.4"/>')
    corners = ('<path d="M36 50v-12h12M80 38h12v12M92 68v12H80M48 80H36V68" fill="none" '
               'stroke-linecap="round" stroke-linejoin="round"/>')
    body += f'<g stroke="#000" stroke-width="5.5" opacity="0.25" transform="translate(0.6 1.2)">{corners}</g>'
    body += f'<g stroke="#ffffff" stroke-width="4.2">{corners}</g>'
    return svg(body, defs)


def camera() -> str:
    shadow, defs = ground(64, 113, 52, 6.5)
    defs += linear("side", [(0, "#262a31", 1), (1, "#121418", 1)])
    defs += linear("body", [(0, "#575f6b", 1), (0.12, "#454c57", 1), (1, "#2a2f37", 1)])
    defs += linear("ringSide", [(0, "#5d6570", 1), (1, "#353b44", 1)])
    defs += linear("ring", [(0, "#f2f4f7", 1), (0.5, "#b4bbc5", 1), (1, "#7d8590", 1)],
                   0, 0, 0.8, 1)
    defs += radial("lens", [(0, "#3f8db3", 1), (0.45, "#1a4a66", 1), (1, "#06121b", 1)],
                   0.42, 0.38, 0.6)
    defs += linear("button", [(0, "#ff9b84", 1), (1, "#d9483a", 1)])
    body = shadow
    body += rr(36, 28, 40, 18, 6, "url(#side)")
    body += rr(36, 22, 40, 18, 6, "url(#body)")
    body += rr(18, 30, 16, 8, 3, "#8a2b22")
    body += rr(18, 27, 16, 8, 3, "url(#button)")
    body += rr(10, 42, 108, 66, 12, "url(#side)")
    body += rr(10, 36, 108, 66, 12, "url(#body)")
    body += edge_light(22, 106, 37, "#8d96a3")
    body += rr(92, 44, 16, 9, 3, "#c9d0d8")
    body += rr(92, 44, 16, 4, 2, "#ffffff", ' opacity="0.6"')
    body += '<circle cx="62" cy="74" r="30" fill="url(#ringSide)"/>'
    body += '<circle cx="62" cy="70" r="30" fill="url(#ring)"/>'
    body += '<circle cx="62" cy="70" r="24" fill="#0b0e12"/>'
    body += '<circle cx="62" cy="71" r="19" fill="url(#lens)"/>'
    body += '<circle cx="62" cy="71" r="19" fill="none" stroke="#000" stroke-width="2" opacity="0.5"/>'
    body += '<ellipse cx="55" cy="63" rx="6" ry="4" fill="#fff" opacity="0.55" transform="rotate(-30 55 63)"/>'
    body += '<path d="M70 82a12 12 0 0 0 6-8" stroke="#b58cff" stroke-width="1.8" fill="none" stroke-linecap="round" opacity="0.7"/>'
    return svg(body, defs)


def documents() -> str:
    shadow, defs = ground(62, 114, 46, 6.5)
    defs += linear("paper", [(0, "#ffffff", 1), (1, "#e9e5dc", 1)])
    defs += linear("rim", [(0, "#f3f5f8", 1), (0.5, "#aab2bd", 1), (1, "#5f6772", 1)], 0, 0, 1, 1)
    defs += radial("lens", [(0, "#ffffff", 0.55), (0.7, "#cfe8f6", 0.35), (1, "#9ccbe6", 0.45)],
                   0.4, 0.35, 0.6)
    defs += linear("handle", [(0, "#4a515c", 1), (1, "#1f242b", 1)], 0, 0, 1, 0)
    body = shadow
    body += '<g transform="rotate(-7 60 62)">'
    body += rr(26, 16, 70, 90, 4, "#c8c1b2")
    body += rr(26, 13, 70, 90, 4, "#f4f1ea")
    body += "</g>"
    body += rr(22, 18, 72, 92, 4, "#cdc6b8")
    body += rr(22, 15, 72, 92, 4, "url(#paper)")
    body += edge_light(27, 89, 15.8, "#ffffff", 1.2)
    body += rr(30, 24, 56, 9, 2, "#e5534b")
    body += rr(30, 40, 26, 20, 2, "#c9d3da")
    body += "".join(f'<path d="M{a} {y}H{b}" stroke="#b6c0c6" stroke-width="2.4" '
                    'stroke-linecap="round"/>' for a, b, y in
                    [(61, 86, 43), (61, 82, 50), (61, 86, 57), (30, 86, 68), (30, 78, 76)])
    # Magnifier: handle under the rim, a thick metal rim, tinted glass.
    body += ('<path d="M95 95l14 14" stroke="#000" stroke-width="12" stroke-linecap="round" '
             'opacity="0.2" transform="translate(1 2)"/>')
    body += '<path d="M95 95l14 14" stroke="url(#handle)" stroke-width="11" stroke-linecap="round"/>'
    body += '<circle cx="82" cy="82" r="19" fill="#000" opacity="0.18" transform="translate(1 2)"/>'
    body += '<circle cx="82" cy="82" r="19" fill="url(#rim)"/>'
    body += '<circle cx="82" cy="82" r="14.5" fill="#ffffff"/>'
    body += ('<path d="M72 80H92M72 87H88" stroke="#9aa6ae" stroke-width="3.2" '
             'stroke-linecap="round"/>')
    body += '<circle cx="82" cy="82" r="14.5" fill="url(#lens)"/>'
    body += ('<path d="M73 76a11 11 0 0 1 8-5" stroke="#fff" stroke-width="2" fill="none" '
             'stroke-linecap="round" opacity="0.9"/>')
    return svg(body, defs)


def store() -> str:
    shadow, defs = ground(64, 114, 48, 6.5)
    defs += linear("front", [(0, "#ff9a63", 1), (1, "#e2573a", 1)])
    defs += linear("gusset", [(0, "#c4452c", 1), (1, "#97321f", 1)])
    defs += linear("fold", [(0, "#ffc19a", 1), (1, "#ff9a63", 1)])
    body = shadow
    # Rope handles behind and in front of the bag opening.
    body += ('<path d="M46 42v-8a18 18 0 0 1 36 0v8" fill="none" stroke="#7a2a1a" '
             'stroke-width="5" stroke-linecap="round"/>')
    body += '<path d="M96 42l12 4v58l-12 6z" fill="url(#gusset)"/>'
    body += '<path d="M20 42h76v68H24a4 4 0 0 1-4-4z" fill="url(#front)"/>'
    body += '<path d="M20 42h76v9H20z" fill="url(#fold)"/>'
    body += '<path d="M20 51h76" stroke="#000" stroke-width="1.2" opacity="0.15"/>'
    body += edge_light(22, 94, 42.8, "#ffe0cc")
    for x in (40, 76):
        body += f'<circle cx="{x}" cy="47" r="2.6" fill="#5c1d11"/>'
    body += ('<path d="M40 47v-12a18 18 0 0 1 36 0v12" fill="none" stroke="#a63a24" '
             'stroke-width="4.6" stroke-linecap="round"/>')
    body += ('<path d="M41.5 40v-5a16.5 16.5 0 0 1 21-15.8" fill="none" stroke="#ff9f7f" '
             'stroke-width="1.2" stroke-linecap="round" opacity="0.8"/>')
    # The Asena star pressed into the front: a light lower edge under a darker
    # recess, like the folder emblems.
    star = star_path(58, 79, 17, 12.5)
    body += f'<path d="{star}" fill="#ffd0b5" transform="translate(0 1.4)" opacity="0.75"/>'
    body += f'<path d="{star}" fill="#b8422b" opacity="0.8"/>'
    return svg(body, defs)


def weather() -> str:
    shadow, defs = ground(66, 113, 46, 6)
    defs += radial("sun", [(0, "#fff3b0", 1), (0.55, "#ffc23d", 1), (1, "#f0901a", 1)],
                   0.38, 0.32, 0.65)
    defs += radial("glow", [(0, "#ffd36e", 0.45), (1, "#ffd36e", 0)])
    defs += linear("cloud", [(0, "#ffffff", 1), (0.6, "#eef2f6", 1), (1, "#c5d0dc", 1)],
                   0, 54, 0, 104, user=True)
    defs += linear("under", [(0, "#9fb0c2", 0), (1, "#8597ab", 0.55)], 0, 84, 0, 104, user=True)
    body = shadow
    body += '<circle cx="46" cy="44" r="38" fill="url(#glow)"/>'
    body += '<circle cx="46" cy="44" r="25" fill="url(#sun)"/>'
    body += ('<path d="M31 34a18 18 0 0 1 14-10" stroke="#fff8d6" stroke-width="2" fill="none" '
             'stroke-linecap="round" opacity="0.9"/>')
    cloud = ('<circle cx="58" cy="72" r="20"/><circle cx="83" cy="62" r="24"/>'
             '<circle cx="102" cy="80" r="14"/><rect x="34" y="72" width="80" height="32" rx="16"/>')
    body += f'<g fill="#000" opacity="0.12" transform="translate(2 4)">{cloud}</g>'
    body += f'<g fill="url(#cloud)">{cloud}</g>'
    body += f'<g fill="url(#under)">{cloud}</g>'
    body += ('<path d="M44 66a16 16 0 0 1 14-13M68 52a22 22 0 0 1 26-8" stroke="#fff" '
             'stroke-width="2.2" fill="none" stroke-linecap="round" opacity="0.95"/>')
    return svg(body, defs)


def fallback() -> str:
    shadow, defs = ground(64, 114, 44, 6)
    defs += linear("side", [(0, "#8c939d", 1), (1, "#666d77", 1)])
    defs += linear("face", [(0, "#f4f5f7", 1), (1, "#cfd4da", 1)])
    defs += linear("deboss", [(0, "#9aa1ab", 1), (1, "#e9ecef", 1)])
    body = shadow
    body += rr(18, 22, 92, 88, 22, "url(#side)")
    body += rr(18, 16, 92, 88, 22, "url(#face)")
    body += ('<path d="M40 17h48" stroke="#fff" stroke-width="1.6" stroke-linecap="round"/>')
    points = []
    for index in range(16):
        radius = 23 if index % 2 == 0 else 17
        angle = math.radians(-90 + index * 22.5)
        points.append((64 + radius * math.cos(angle), 59 + radius * math.sin(angle)))
    star = "M" + "L".join(f"{f(x)} {f(y)}" for x, y in points) + "Z"
    body += f'<path d="{star}" fill="url(#deboss)" transform="translate(0 1.2)"/>'
    body += f'<path d="{star}" fill="#dfe3e8"/>'
    return svg(body, defs)


def system_monitor() -> str:
    shadow, defs = ground(64, 114, 43, 6)
    defs += linear("frame", [(0, "#dce4ec", 1), (1, "#788b9e", 1)])
    defs += linear("screen", [(0, "#1c3f49", 1), (1, "#10252f", 1)])
    defs += linear("stand", [(0, "#c7d4df", 1), (1, "#788b9e", 1)])
    body = shadow + rr(53, 82, 22, 24, 4, "url(#stand)")
    body += '<path d="M42 105h44l9 7H33Z" fill="url(#stand)"/>'
    body += rr(12, 20, 104, 72, 11, "#596d80")
    body += rr(12, 14, 104, 72, 11, "url(#frame)")
    body += rr(18, 20, 92, 57, 6, "url(#screen)")
    body += '<path d="M22 49h17l8-15 11 29 10-39 12 29 8-10h18" fill="none" stroke="#76e2b8" stroke-width="4" stroke-linejoin="round"/>'
    body += '<path d="M24 16h78" stroke="#fff" stroke-width="1.4" opacity=".8"/>'
    return svg(body, defs)


def backup_drive() -> str:
    shadow, defs = ground(64, 114, 42, 6)
    defs += linear("drive", [(0, "#dce6ed", 1), (1, "#96aaba", 1)])
    defs += linear("side", [(0, "#71899a", 1), (1, "#4b6275", 1)])
    body = shadow + rr(23, 25, 82, 85, 14, "url(#side)")
    body += rr(23, 16, 82, 84, 14, "url(#drive)")
    body += '<path d="M38 18h52" stroke="#fff" stroke-width="1.5"/>'
    body += '<circle cx="64" cy="56" r="24" fill="#147c72"/>'
    body += '<path d="M77 50a15 15 0 1 0 0 14M77 41v12H65" fill="none" stroke="#e0fff1" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>'
    body += '<path d="M37 89h32" stroke="#5c7585" stroke-width="3" stroke-linecap="round"/>'
    body += '<circle cx="91" cy="89" r="3" fill="#36ac93"/>'
    return svg(body, defs)


def disks() -> str:
    shadow, defs = ground(64, 114, 43, 6)
    defs += linear("case", [(0, "#d6dce3", 1), (1, "#8a9cab", 1)])
    defs += linear("platter", [(0, "#f8fbff", 1), (.35, "#a6bed0", 1), (.55, "#e9f1f8", 1), (1, "#839aaf", 1)])
    body = shadow + rr(22, 20, 84, 91, 10, "#536979")
    body += rr(22, 14, 84, 91, 10, "url(#case)")
    body += '<circle cx="63" cy="55" r="30" fill="#5c7181"/>'
    body += '<circle cx="63" cy="53" r="28" fill="url(#platter)" stroke="#f4f8fb"/>'
    body += '<circle cx="63" cy="53" r="9" fill="#627d91" stroke="#e9f0f7" stroke-width="3"/>'
    body += '<path d="M91 91 70 62l-5 2 14 32Z" fill="#435c6d"/>'
    body += '<path d="m88 85-18-25-5 2 14 29Z" fill="#e4ebf1"/>'
    body += '<circle cx="87" cy="91" r="7" fill="#8b9ead" stroke="#f1f5f9" stroke-width="2"/>'
    for x, y in ((29, 22), (99, 22), (29, 97)):
        body += f'<circle cx="{x}" cy="{y}" r="2" fill="#5b7182"/>'
    return svg(body, defs)


ICONS = {
    "org.asenalinux.Files": files,
    "org.asenalinux.Settings": settings,
    "org.asenalinux.Terminal": terminal,
    "org.asenalinux.Photos": photos,
    "org.asenalinux.Text": text_editor,
    "org.asenalinux.Notes": notes,
    "org.asenalinux.Calculator": calculator,
    "org.asenalinux.Archive": archive,
    "org.asenalinux.Calendar": calendar,
    "org.asenalinux.Clock": clock,
    "org.asenalinux.Player": player,
    "org.asenalinux.Capture": capture,
    "org.asenalinux.Camera": camera,
    "org.asenalinux.Documents": documents,
    "org.asenalinux.Store": store,
    "org.asenalinux.Weather": weather,
    "org.asenalinux.SystemMonitor": system_monitor,
    "org.asenalinux.Backup": backup_drive,
    "org.asenalinux.Disks": disks,
    "org.asenalinux.App": fallback,
}


ACCENT_ICONS = {
    "org.asenalinux.Files": files,
    "org.asenalinux.Settings": settings,
}


def accent_index(accent: str) -> str:
    return f"""[Icon Theme]
Name=Asena-{accent}
Comment=Asena icons with {accent} folders
Inherits=Asena,Adwaita,hicolor
Hidden=true
Directories=scalable/apps,scalable/places

[scalable/apps]
Context=Applications
Size=128
MinSize=8
MaxSize=1024
Type=Scalable

[scalable/places]
Context=Places
Size=128
MinSize=8
MaxSize=1024
Type=Scalable
"""


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


def write_places(theme: Path, accent: str) -> None:
    for name, emblem in PLACES.items():
        write(theme / "scalable/places" / f"{name}.svg", folder(accent, emblem))


if __name__ == "__main__":
    for output in OUTPUTS:
        for name, draw in ICONS.items():
            write(output / f"{name}.svg", draw())
        print(f"Wrote {len(ICONS)} application icons to {output.relative_to(ROOT)}")
    write_places(ICONS_DIR / "Asena", BASE_ACCENT)
    print(f"Wrote {len(PLACES)} {BASE_ACCENT} folders to data/icons/Asena")
    for accent in ACCENTS:
        if accent == BASE_ACCENT:
            continue
        theme = ICONS_DIR / f"Asena-{accent}"
        write(theme / "index.theme", accent_index(accent))
        for name, draw in ACCENT_ICONS.items():
            write(theme / "scalable/apps" / f"{name}.svg", draw(accent))
        write_places(theme, accent)
    print(f"Wrote {len(ACCENTS) - 1} accent icon themes")
