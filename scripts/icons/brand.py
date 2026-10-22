#!/usr/bin/env python3
"""Generate the Asena logo and the branding assets drawn from it (K07).

The mark is the Seljuk star of the start menu button (asena-start-symbolic):
two squares, one turned by 45 degrees, woven over and under each other on a
raised turquoise tile. The light matches scripts/icons/apps.py: key light from
the upper left, real thickness and a glazed ceramic surface. The user chose
this drawing (draft A) on 23 October 2026.

This script is the editable source of every file in OUTPUTS; tests/test_branding.py
checks that the committed files match it. Run from the repository root:

    python3 scripts/icons/brand.py
"""

from __future__ import annotations

import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

CX, CY = 64.0, 61.0
# Half side of the two squares whose union is the star tile.
TILE = 36.0
# Half side of the woven band squares, measured on the band's centre line.
BAND = 25.5
BAND_WIDTH = 5.4
GROOVE = 1.2
DEPTH = 5.0

TEXT = "#eef6f5"
NIGHT = ("#0d1b1e", "#071012")


def f(value: float) -> str:
    text = f"{value:.2f}".rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def square(half: float, turned: bool, cx: float = CX, cy: float = CY) -> list[tuple[float, float]]:
    """Corners clockwise from the top (turned) or the top left (upright)."""
    if turned:
        r = half * math.sqrt(2)
        return [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]
    return [(cx - half, cy - half), (cx + half, cy - half), (cx + half, cy + half),
            (cx - half, cy + half)]


def polygon(points: list[tuple[float, float]]) -> str:
    return "M" + "L".join(f"{f(x)} {f(y)}" for x, y in points) + "Z"


def star_outline(half: float, cx: float = CX, cy: float = CY) -> list[tuple[float, float]]:
    """Union of the upright and the turned square: an eight point star."""
    outer = half * math.sqrt(2)
    # Each upright side crosses the turned square at half * (sqrt(2) - 1) from
    # the axis; those crossings are the inner corners of the star.
    inner = math.hypot(half, half * (math.sqrt(2) - 1))
    points = []
    for index in range(8):
        angle = math.radians(-90 + index * 45)
        points.append((cx + outer * math.cos(angle), cy + outer * math.sin(angle)))
        between = math.radians(-90 + index * 45 + 22.5)
        points.append((cx + inner * math.cos(between), cy + inner * math.sin(between)))
    return points


def crossings(half: float) -> list[tuple[float, float]]:
    """Points where the upright and the turned band squares cross, clockwise."""
    o = half * (math.sqrt(2) - 1)
    return [
        (CX - o, CY - half), (CX + o, CY - half), (CX + half, CY - o), (CX + half, CY + o),
        (CX + o, CY + half), (CX - o, CY + half), (CX - half, CY + o), (CX - half, CY - o),
    ]


def linear(name: str, stops: list[tuple[float, str, float]], x1: float, y1: float,
           x2: float, y2: float) -> str:
    items = "".join(
        f'<stop offset="{f(o)}" stop-color="{c}" stop-opacity="{f(a)}"/>' for o, c, a in stops
    )
    return (f'<linearGradient id="{name}" gradientUnits="userSpaceOnUse" x1="{f(x1)}" '
            f'y1="{f(y1)}" x2="{f(x2)}" y2="{f(y2)}">{items}</linearGradient>')


def radial(name: str, stops: list[tuple[float, str, float]], cx: float, cy: float,
           r: float) -> str:
    items = "".join(
        f'<stop offset="{f(o)}" stop-color="{c}" stop-opacity="{f(a)}"/>' for o, c, a in stops
    )
    return (f'<radialGradient id="{name}" gradientUnits="userSpaceOnUse" cx="{f(cx)}" '
            f'cy="{f(cy)}" r="{f(r)}">{items}</radialGradient>')


def svg(body: str, defs: str, width: int = 128, height: int = 128) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}"><defs>{defs}</defs>{body}</svg>\n'
    )


def strip(x: float, y: float, axis: tuple[float, float], along: float,
          across: float) -> list[tuple[float, float]]:
    ux, uy = axis
    nx, ny = -uy, ux
    return [(x + sa * along * ux + sc * across * nx, y + sa * along * uy + sc * across * ny)
            for sa, sc in ((-1, -1), (1, -1), (1, 1), (-1, 1))]


def weave(band: str, groove: str, half: float, width: float, prefix: str) -> tuple[str, str]:
    """Two band squares; the turned one passes over at every other crossing."""
    # Each corner of one band must clear the other band completely; otherwise
    # the corner sits under two neighbouring crossings and its outline breaks.
    edge = width / 2 + GROOVE
    if half * (math.sqrt(2) - 1) < edge * (1 + math.sqrt(2)) * 1.05:
        raise ValueError(f"band {width} is too wide for squares of half side {half}")
    upright = polygon(square(half, False))
    turned = polygon(square(half, True))

    def stroke(path: str) -> str:
        return (f'<path d="{path}" fill="none" stroke="{groove}" '
                f'stroke-width="{f(width + 2 * GROOVE)}" stroke-linejoin="miter"/>'
                f'<path d="{path}" fill="none" stroke="{band}" stroke-width="{f(width)}" '
                f'stroke-linejoin="miter"/>')

    # Where the turned band passes over, it is drawn again inside the overlap
    # of the two bands: the intersection of one strip along each band.
    across = edge + 0.5
    along = across * (1 + math.sqrt(2)) + 0.5
    defs = ""
    body = stroke(turned) + stroke(upright)
    for index, (x, y) in enumerate(crossings(half)):
        if index % 2:
            continue
        sx, sy = math.copysign(1, x - CX), math.copysign(1, y - CY)
        side = (1.0, 0.0) if abs(abs(y - CY) - half) < 1e-6 else (0.0, 1.0)
        diagonal = (sy / math.sqrt(2), -sx / math.sqrt(2))
        for name, axis in (("u", side), ("t", diagonal)):
            defs += (f'<clipPath id="{prefix}-{name}{index}">'
                     f'<path d="{polygon(strip(x, y, axis, along, across))}"/></clipPath>')
        body += (f'<g clip-path="url(#{prefix}-u{index})"><g clip-path="url(#{prefix}-t{index})">'
                 f"{stroke(turned)}</g></g>")
    return body, defs


def tile(prefix: str, ground: bool, weave_bands: bool = True) -> tuple[str, str]:
    """The raised turquoise tile on the 128 unit canvas."""
    star = polygon(star_outline(TILE))
    top, bottom = CY - TILE * math.sqrt(2), CY + TILE * math.sqrt(2)
    p = prefix
    defs = linear(f"{p}-side", [(0, "#0f6764", 1), (1, "#0a4645", 1)], 0, top, 0, bottom + DEPTH)
    defs += linear(f"{p}-glaze", [(0, "#62d2c9", 1), (0.55, "#25a7a6", 1), (1, "#147a7a", 1)],
                   CX - 40, top, CX + 34, bottom)
    defs += radial(f"{p}-sheen", [(0, "#ffffff", 0.42), (1, "#ffffff", 0)], CX - 20, CY - 26, 36)
    defs += linear(f"{p}-band", [(0, "#fffaf0", 1), (1, "#e3d6bc", 1)],
                   CX - 30, top, CX + 30, bottom)
    defs += linear(f"{p}-rim", [(0, "#ffffff", 0.75), (0.5, "#ffffff", 0), (1, "#ffffff", 0)],
                   CX - 30, top, CX + 20, bottom)
    defs += f'<clipPath id="{p}-face"><path d="{star}"/></clipPath>'
    body = ""
    if ground:
        defs += radial(f"{p}-ground", [(0, "#000", 0.3), (0.6, "#000", 0.12), (1, "#000", 0)],
                       CX, bottom + DEPTH, 40)
        body += (f'<ellipse cx="{f(CX)}" cy="{f(bottom + DEPTH - 1)}" rx="40" ry="5" '
                 f'fill="url(#{p}-ground)"/>')
    body += f'<path d="{star}" fill="url(#{p}-side)" transform="translate(0 {f(DEPTH)})"/>'
    body += f'<path d="{star}" fill="url(#{p}-glaze)"/>'
    body += f'<path d="{star}" fill="url(#{p}-sheen)"/>'
    if weave_bands:
        weave_body, weave_defs = weave(f"url(#{p}-band)", "#0c5654", BAND, BAND_WIDTH, p)
        defs += weave_defs
        body += f'<g clip-path="url(#{p}-face)">{weave_body}</g>'
        boss = 5.6
    else:
        # Small sizes: the weave would blur into grey, so only the boss stays.
        boss = 14.0
    # Ivory boss in the middle, slightly raised.
    body += f'<circle cx="{f(CX)}" cy="{f(CY + 0.8)}" r="{f(boss + 0.8)}" fill="#0c5654"/>'
    body += f'<circle cx="{f(CX)}" cy="{f(CY)}" r="{f(boss)}" fill="url(#{p}-band)"/>'
    body += (f'<path d="{star}" fill="none" stroke="url(#{p}-rim)" stroke-width="2.4" '
             f'clip-path="url(#{p}-face)"/>')
    return body, defs


def emblem() -> str:
    """The logo alone, for compositions: splash screens, the greeter, the website."""
    body, defs = tile("asena", ground=False)
    return svg(body, defs)


def app_icon() -> str:
    """The logo as an icon, standing on the same ground shadow as the app icons."""
    body, defs = tile("asena", ground=True)
    return svg(body, defs)


def small_icon() -> str:
    """16 to 32 px: the tile and a large boss, without the weave."""
    body, defs = tile("asena", ground=False, weave_bands=False)
    return svg(body, defs)


def splash() -> str:
    """Live GRUB background, 800x600 like live-build's gfxmode. The menu starts
    at 52 % of the height, so the logo and the name stay in the upper half."""
    width, height = 800, 600
    scale = 1.15
    logo, logo_defs = tile("splash", ground=False)
    defs = linear("night", [(0, NIGHT[0], 1), (1, NIGHT[1], 1)], 0, 0, 0, height)
    defs += radial("glow", [(0, "#25a7a6", 0.22), (1, "#25a7a6", 0)], 400, 140, 190)
    defs += logo_defs
    body = f'<rect width="{width}" height="{height}" fill="url(#night)"/>'
    body += f'<rect width="{width}" height="{height}" fill="url(#glow)"/>'
    x = width / 2 - CX * scale
    y = 140 - CY * scale
    body += f'<g transform="translate({f(x)} {f(y)}) scale({f(scale)})">{logo}</g>'
    body += (f'<text x="400" y="268" text-anchor="middle" fill="{TEXT}" '
             'font-family="Inter, DejaVu Sans, sans-serif" font-size="34" font-weight="600" '
             'letter-spacing="0.5">Asena Linux</text>')
    return svg(body, defs, width, height)


def ascii_logo() -> str:
    """fastfetch logo: $1 is the turquoise tile, $2 the ivory bands and boss.
    Terminal cells are about twice as tall as wide, so the grid has twice as
    many columns as rows."""
    rows, columns = 21, 42
    outer = TILE * math.sqrt(2) + 1
    star = star_outline(TILE)
    band_half = 2.4

    def inside(x: float, y: float) -> bool:
        count = 0
        for (x1, y1), (x2, y2) in zip(star, star[1:] + star[:1]):
            if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                count += 1
        return count % 2 == 1

    def cell(x: float, y: float) -> tuple[str, str]:
        dx, dy = x - CX, y - CY
        upright = abs(max(abs(dx), abs(dy)) - BAND)
        turned = abs((abs(dx) + abs(dy)) / math.sqrt(2) - BAND)
        if not inside(x, y):
            return " ", ""
        if math.hypot(dx, dy) <= 5.5:
            return "O", "$2"
        if turned <= band_half and turned <= upright:
            return ("/" if dx * dy > 0 else "\\"), "$2"
        if upright <= band_half:
            return ("=" if abs(dy) > abs(dx) else "|"), "$2"
        return ":", "$1"

    lines = []
    for row in range(rows):
        y = CY - outer + (row + 0.5) * 2 * outer / rows
        line, color = "", ""
        for column in range(columns):
            x = CX - outer + (column + 0.5) * 2 * outer / columns
            character, want = cell(x, y)
            if want and want != color:
                line += want
                color = want
            line += character
        lines.append(line.rstrip())
    return "\n".join(lines) + "\n"


OUTPUTS = {
    "data/branding/asena-emblem.svg": emblem,
    "data/icons/hicolor/scalable/apps/asena-logo.svg": app_icon,
    "data/icons/hicolor/16x16/apps/asena-logo.svg": small_icon,
    "data/icons/hicolor/24x24/apps/asena-logo.svg": small_icon,
    "data/icons/hicolor/32x32/apps/asena-logo.svg": small_icon,
    "data/branding/asena-logo.txt": ascii_logo,
    "iso/config/bootloaders/splash.svg": splash,
}


def main() -> None:
    for relative, draw in OUTPUTS.items():
        path = ROOT / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(draw().encode("utf-8"))


if __name__ == "__main__":
    main()
