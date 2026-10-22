#!/usr/bin/env python3
"""Generate the Asena Odak symbolic (line) icon family.

This script is the editable source of every symbolic icon in
data/icons/Asena/symbolic. Icons are drawn on a 16 unit grid with a single
1.2 unit line weight (about 1.8 px at 24 px), round caps and round joins.

GTK recolors symbolic icons by overriding the fill of path elements, so every
line is emitted as a filled outline. Knockouts use clip paths, which only use
geometry and survive recoloring. Run from the repository root:

    python3 scripts/icons/symbolic.py
"""

from __future__ import annotations

import math
from pathlib import Path

SW = 1.2
HW = SW / 2
DIM = 0.32
ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "data/icons/Asena/symbolic"


def fmt(value: float) -> str:
    text = f"{value:.3f}".rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def pt(x: float, y: float) -> str:
    return f"{fmt(x)} {fmt(y)}"


def polar(cx: float, cy: float, r: float, degrees: float) -> tuple[float, float]:
    angle = math.radians(degrees)
    return cx + r * math.cos(angle), cy + r * math.sin(angle)


def rounded_rect_path(x: float, y: float, w: float, h: float, r: float) -> str:
    r = max(0.0, min(r, w / 2, h / 2))
    if r == 0:
        return f"M{pt(x, y)}H{fmt(x + w)}V{fmt(y + h)}H{fmt(x)}Z"
    return (
        f"M{pt(x + r, y)}H{fmt(x + w - r)}A{fmt(r)} {fmt(r)} 0 0 1 {pt(x + w, y + r)}"
        f"V{fmt(y + h - r)}A{fmt(r)} {fmt(r)} 0 0 1 {pt(x + w - r, y + h)}"
        f"H{fmt(x + r)}A{fmt(r)} {fmt(r)} 0 0 1 {pt(x, y + h - r)}"
        f"V{fmt(y + r)}A{fmt(r)} {fmt(r)} 0 0 1 {pt(x + r, y)}Z"
    )


def circle_path(cx: float, cy: float, r: float) -> str:
    return (
        f"M{pt(cx + r, cy)}A{fmt(r)} {fmt(r)} 0 1 0 {pt(cx - r, cy)}"
        f"A{fmt(r)} {fmt(r)} 0 1 0 {pt(cx + r, cy)}Z"
    )


class Icon:
    """A symbolic icon assembled from stroke and fill primitives."""

    def __init__(self) -> None:
        self.parts: list[str] = []
        self.clips: list[str] = []
        self.group: list[str] | None = None
        self.group_clip: list[str] | None = None

    # Output helpers -----------------------------------------------------
    def _emit(self, d: str, opacity: float | None = None, evenodd: bool = False) -> None:
        attributes = f'd="{d}"'
        if evenodd:
            attributes += ' fill-rule="evenodd"'
        if opacity is not None and opacity < 1:
            attributes += f' opacity="{fmt(opacity)}"'
        target = self.group if self.group is not None else self.parts
        target.append(f"<path {attributes}/>")

    def knockout(self, bands: list[str]) -> "Icon":
        """Clip following parts so the given filled shapes stay clear.

        Each shape gets its own clip path and the groups are nested, so
        overlapping clearance shapes never cancel each other out.
        """
        canvas = "M-1 -1H17V17H-1Z"
        self.group_clip = []
        for shape in bands:
            index = len(self.clips)
            self.clips.append(
                f'<clipPath id="k{index}"><path clip-rule="evenodd" d="{canvas}{shape}"/></clipPath>'
            )
            self.group_clip.append(f"k{index}")
        self.group = []
        return self

    def end_knockout(self) -> "Icon":
        if self.group is not None:
            body = "".join(self.group)
            for clip in reversed(self.group_clip or []):
                body = f'<g clip-path="url(#{clip})">{body}</g>'
            self.parts.append(body)
        self.group = None
        self.group_clip = None
        return self

    # Stroke primitives --------------------------------------------------
    def line(self, x0: float, y0: float, x1: float, y1: float, opacity: float | None = None,
             width: float = SW) -> "Icon":
        self._emit(capsule(x0, y0, x1, y1, width / 2), opacity)
        return self

    def polyline(self, points: list[tuple[float, float]], closed: bool = False,
                 opacity: float | None = None, width: float = SW) -> "Icon":
        sequence = list(points) + ([points[0]] if closed else [])
        for (x0, y0), (x1, y1) in zip(sequence, sequence[1:]):
            self.line(x0, y0, x1, y1, opacity, width)
        return self

    def arc(self, cx: float, cy: float, r: float, start: float, end: float,
            opacity: float | None = None, width: float = SW) -> "Icon":
        """Arc from start to end degrees, increasing angle is clockwise on screen."""
        self._emit(annular_sector(cx, cy, r, start, end, width / 2), opacity)
        return self

    def circle(self, cx: float, cy: float, r: float, opacity: float | None = None,
               width: float = SW) -> "Icon":
        hw = width / 2
        self._emit(circle_path(cx, cy, r + hw) + circle_path(cx, cy, r - hw), opacity, True)
        return self

    def rrect(self, x: float, y: float, w: float, h: float, r: float,
              opacity: float | None = None, width: float = SW) -> "Icon":
        hw = width / 2
        outer = rounded_rect_path(x - hw, y - hw, w + width, h + width, r + hw)
        inner = rounded_rect_path(x + hw, y + hw, w - width, h - width, max(r - hw, 0))
        self._emit(outer + inner, opacity, True)
        return self

    def curve(self, p0, p1, p2, p3, opacity: float | None = None, steps: int = 12) -> "Icon":
        points = []
        for index in range(steps + 1):
            t = index / steps
            a = (1 - t) ** 3
            b = 3 * (1 - t) ** 2 * t
            c = 3 * (1 - t) * t ** 2
            d = t ** 3
            points.append((a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0],
                           a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]))
        return self.polyline(points, opacity=opacity)

    # Fill primitives ----------------------------------------------------
    def disc(self, cx: float, cy: float, r: float, opacity: float | None = None) -> "Icon":
        self._emit(circle_path(cx, cy, r), opacity)
        return self

    def box(self, x: float, y: float, w: float, h: float, r: float,
            opacity: float | None = None) -> "Icon":
        self._emit(rounded_rect_path(x, y, w, h, r), opacity)
        return self

    def fill(self, d: str, opacity: float | None = None) -> "Icon":
        self._emit(d, opacity)
        return self

    def arrowhead(self, x: float, y: float, direction: float, length: float = 2.6,
                  spread: float = 42) -> "Icon":
        """Open arrowhead with its tip at (x, y) pointing towards direction degrees."""
        back = direction + 180
        wing1 = polar(x, y, length, back - spread)
        wing2 = polar(x, y, length, back + spread)
        return self.polyline([wing1, (x, y), wing2])

    def poly(self, points: list[tuple[float, float]], opacity: float | None = None) -> "Icon":
        d = "M" + "L".join(pt(x, y) for x, y in points) + "Z"
        self._emit(d, opacity)
        return self

    def svg(self) -> str:
        self.end_knockout()
        defs = f"<defs>{''.join(self.clips)}</defs>" if self.clips else ""
        body = "".join(self.parts)
        return (
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" '
            f'viewBox="0 0 16 16" fill="#2e3440">{defs}{body}</svg>\n'
        )


def capsule(x0: float, y0: float, x1: float, y1: float, hw: float) -> str:
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy)
    if length < 1e-6:
        return circle_path(x0, y0, hw)
    nx, ny = -dy / length * hw, dx / length * hw
    r = fmt(hw)
    return (
        f"M{pt(x0 + nx, y0 + ny)}L{pt(x1 + nx, y1 + ny)}"
        f"A{r} {r} 0 0 0 {pt(x1 - nx, y1 - ny)}L{pt(x0 - nx, y0 - ny)}"
        f"A{r} {r} 0 0 0 {pt(x0 + nx, y0 + ny)}Z"
    )


def annular_sector(cx: float, cy: float, r: float, start: float, end: float, hw: float) -> str:
    span = end - start
    if span >= 359.9:
        return circle_path(cx, cy, r + hw) + circle_path(cx, cy, r - hw)
    large = 1 if span > 180 else 0
    outer, inner = r + hw, r - hw
    o0, o1 = polar(cx, cy, outer, start), polar(cx, cy, outer, end)
    i0, i1 = polar(cx, cy, inner, start), polar(cx, cy, inner, end)
    c = fmt(hw)
    return (
        f"M{pt(*o0)}A{fmt(outer)} {fmt(outer)} 0 {large} 1 {pt(*o1)}"
        f"A{c} {c} 0 0 1 {pt(*i1)}"
        f"A{fmt(inner)} {fmt(inner)} 0 {large} 0 {pt(*i0)}"
        f"A{c} {c} 0 0 1 {pt(*o0)}Z"
    )


def band(x0: float, y0: float, x1: float, y1: float, half: float) -> str:
    """A clearance band around a slash used for knockouts."""
    return capsule(x0, y0, x1, y1, half)


def ring_band(cx: float, cy: float, r: float) -> str:
    return circle_path(cx, cy, r)


ICONS: dict[str, tuple[str, object]] = {}


def icon(context: str, *names: str):
    """Register a drawing function; it runs at write time so helpers may follow it."""
    def register(function):
        for name in names:
            ICONS[name] = (context, function)
        return function
    return register


def slashed(draw, x0=2.2, y0=2.2, x1=13.8, y1=13.8) -> Icon:
    """Draw an icon with a diagonal slash and a clear gap around it."""
    result = Icon()
    result.knockout([band(x0, y0, x1, y1, HW + 1.0)])
    draw(result)
    result.end_knockout()
    result.line(x0, y0, x1, y1)
    return result


# Brand and shell ------------------------------------------------------------
@icon("actions", "asena-start-symbolic", "asena-logo-symbolic")
def start():
    # Two overlapping squares: the eight point star of Anatolian ornament.
    i = Icon()
    i.polyline([(3.4, 3.4), (12.6, 3.4), (12.6, 12.6), (3.4, 12.6)], closed=True)
    d = 6.5
    i.polyline([(8, 8 - d), (8 + d, 8), (8, 8 + d), (8 - d, 8)], closed=True)
    i.disc(8, 8, 1.35)
    return i


@icon("actions", "view-app-grid-symbolic")
def app_grid():
    i = Icon()
    for row in range(3):
        for column in range(3):
            i.disc(3.2 + column * 4.8, 3.2 + row * 4.8, 1.25)
    return i


@icon("actions", "view-grid-symbolic")
def view_grid():
    i = Icon()
    for x, y in [(2, 2), (9, 2), (2, 9), (9, 9)]:
        i.rrect(x + 0.6, y + 0.6, 4.4, 4.4, 1.2)
    return i


@icon("actions", "view-list-symbolic")
def view_list():
    i = Icon()
    for y in (3.5, 8, 12.5):
        i.disc(2.6, y, 1.0)
        i.line(5.6, y, 13.6, y)
    return i


@icon("actions", "view-pin-symbolic", "pin-symbolic")
def pin():
    i = Icon()
    i.polyline([(6, 2.2), (10, 2.2)])
    i.polyline([(6.6, 2.2), (6.6, 6.4), (4.2, 9.4), (11.8, 9.4), (9.4, 6.4), (9.4, 2.2)])
    i.line(8, 9.4, 8, 14)
    return i


@icon("actions", "open-menu-symbolic")
def open_menu():
    i = Icon()
    for y in (3.6, 8, 12.4):
        i.line(2.4, y, 13.6, y)
    return i


@icon("actions", "view-more-symbolic", "view-more-horizontal-symbolic")
def view_more():
    i = Icon()
    for x in (3, 8, 13):
        i.disc(x, 8, 1.3)
    return i


@icon("actions", "view-more-vertical-symbolic")
def view_more_vertical():
    i = Icon()
    for y in (3, 8, 13):
        i.disc(8, y, 1.3)
    return i


# Navigation -----------------------------------------------------------------
def chevron(direction: str, size: float = 5.0) -> Icon:
    i = Icon()
    h = size / 2
    if direction == "left":
        i.polyline([(8 + h * 0.6, 8 - size), (8 - h * 0.9, 8), (8 + h * 0.6, 8 + size)])
    elif direction == "right":
        i.polyline([(8 - h * 0.6, 8 - size), (8 + h * 0.9, 8), (8 - h * 0.6, 8 + size)])
    elif direction == "up":
        i.polyline([(8 - size, 8 + h * 0.6), (8, 8 - h * 0.9), (8 + size, 8 + h * 0.6)])
    else:
        i.polyline([(8 - size, 8 - h * 0.6), (8, 8 + h * 0.9), (8 + size, 8 - h * 0.6)])
    return i


ICONS["go-previous-symbolic"] = ("actions", chevron("left"))
ICONS["go-next-symbolic"] = ("actions", chevron("right"))
ICONS["go-up-symbolic"] = ("actions", chevron("up"))
ICONS["go-down-symbolic"] = ("actions", chevron("down"))
ICONS["pan-start-symbolic"] = ("actions", chevron("left", 3.6))
ICONS["pan-end-symbolic"] = ("actions", chevron("right", 3.6))
ICONS["pan-up-symbolic"] = ("actions", chevron("up", 3.6))
ICONS["pan-down-symbolic"] = ("actions", chevron("down", 3.6))


@icon("actions", "window-close-symbolic", "edit-delete-remove-symbolic")
def close():
    i = Icon()
    i.line(4, 4, 12, 12)
    i.line(12, 4, 4, 12)
    return i


@icon("actions", "edit-clear-symbolic")
def edit_clear():
    i = Icon()
    i.circle(8, 8, 6)
    i.line(5.8, 5.8, 10.2, 10.2)
    i.line(10.2, 5.8, 5.8, 10.2)
    return i


@icon("actions", "window-minimize-symbolic")
def minimize():
    return Icon().line(4, 11.5, 12, 11.5)


@icon("actions", "window-maximize-symbolic")
def maximize():
    return Icon().rrect(3.5, 3.5, 9, 9, 1.8)


@icon("actions", "window-restore-symbolic")
def restore():
    i = Icon()
    i.knockout([rounded_rect_path(1.4, 3.9, 10.7, 10.7, 2.6)])
    i.rrect(5.5, 2.5, 8, 8, 1.6)
    i.end_knockout()
    i.rrect(3, 5.5, 7.5, 7.5, 1.6)
    return i


@icon("actions", "list-add-symbolic", "tab-new-symbolic")
def add():
    i = Icon()
    i.line(8, 2.8, 8, 13.2)
    i.line(2.8, 8, 13.2, 8)
    return i


@icon("actions", "list-remove-symbolic")
def remove():
    return Icon().line(2.8, 8, 13.2, 8)


@icon("actions", "object-select-symbolic", "emblem-ok-symbolic", "emblem-default-symbolic")
def check():
    return Icon().polyline([(2.6, 8.4), (6.2, 12), (13.4, 4.4)])


@icon("actions", "edit-find-symbolic", "system-search-symbolic")
def search():
    i = Icon()
    i.circle(6.8, 6.8, 4.4)
    i.line(10.2, 10.2, 13.8, 13.8)
    return i


@icon("actions", "zoom-in-symbolic")
def zoom_in():
    i = search()
    i.line(4.8, 6.8, 8.8, 6.8)
    i.line(6.8, 4.8, 6.8, 8.8)
    return i


@icon("actions", "zoom-out-symbolic")
def zoom_out():
    i = search()
    i.line(4.8, 6.8, 8.8, 6.8)
    return i


@icon("actions", "view-refresh-symbolic", "system-reboot-symbolic")
def refresh():
    i = Icon()
    i.arc(8, 8.4, 5.4, -40, 250)
    tip = polar(8, 8.4, 5.4, 250)
    # Clockwise tangent at the arc end.
    i.arrowhead(tip[0] + 0.6, tip[1] - 0.2, 250 + 90 - 12, 2.8)
    return i


@icon("actions", "view-reveal-symbolic")
def reveal():
    i = Icon()
    # Two arcs meeting exactly at the eye corners (2.4, 8) and (13.6, 8).
    i.arc(8, 12.73, 7.33, 220.2, 319.8)
    i.arc(8, 3.27, 7.33, 40.2, 139.8)
    i.circle(8, 8, 2.1)
    return i


@icon("actions", "document-edit-symbolic", "edit-symbolic")
def edit():
    i = Icon()
    i.polyline([(3, 13), (3.4, 10.4), (10.8, 3), (13, 5.2), (5.6, 12.6)], closed=True)
    i.line(9.2, 4.6, 11.4, 6.8)
    return i


@icon("actions", "edit-copy-symbolic")
def copy():
    i = Icon()
    i.rrect(5.5, 5.5, 8, 8.5, 1.6)
    i.polyline([(2.5, 10.5), (2.5, 3.6)])
    i.arc(4.1, 3.6, 1.6, 180, 270)
    i.polyline([(4.1, 2), (9.6, 2)])
    return i


@icon("actions", "edit-paste-symbolic")
def paste():
    i = Icon()
    i.rrect(3, 3.2, 10, 11, 1.8)
    i.rrect(5.8, 1.8, 4.4, 2.6, 0.9)
    i.line(5.6, 8, 10.4, 8)
    i.line(5.6, 10.8, 9, 10.8)
    return i


@icon("actions", "edit-cut-symbolic")
def cut():
    i = Icon()
    i.circle(4.4, 11.6, 2)
    i.circle(11.6, 11.6, 2)
    i.line(5.8, 10.2, 12, 2.4)
    i.line(10.2, 10.2, 4, 2.4)
    return i


@icon("actions", "edit-undo-symbolic")
def undo():
    i = Icon()
    i.polyline([(5.4, 2.8), (2.4, 5.8), (5.4, 8.8)])
    i.polyline([(2.4, 5.8), (9.4, 5.8)])
    i.arc(9.4, 9.2, 3.4, 270, 450)
    i.line(9.4, 12.6, 6, 12.6)
    return i


@icon("actions", "edit-redo-symbolic")
def redo():
    i = Icon()
    i.polyline([(10.6, 2.8), (13.6, 5.8), (10.6, 8.8)])
    i.polyline([(13.6, 5.8), (6.6, 5.8)])
    i.arc(6.6, 9.2, 3.4, 90, 270)
    i.line(6.6, 12.6, 10, 12.6)
    return i


@icon("actions", "user-trash-symbolic", "edit-delete-symbolic")
def trash():
    i = Icon()
    i.line(2.4, 4, 13.6, 4)
    i.polyline([(6, 4), (6, 2.2), (10, 2.2), (10, 4)])
    i.polyline([(3.8, 4), (4.6, 13.8), (11.4, 13.8), (12.2, 4)])
    i.line(6.8, 6.8, 6.8, 11)
    i.line(9.2, 6.8, 9.2, 11)
    return i


@icon("actions", "document-new-symbolic")
def document_new():
    i = document()
    i.line(8, 7.4, 8, 11.8)
    i.line(5.8, 9.6, 10.2, 9.6)
    return i


@icon("actions", "document-open-symbolic")
def document_open():
    i = Icon()
    i.polyline([(2, 12.8), (2, 3.4), (6, 3.4), (7.4, 5), (12, 5), (12, 7)])
    i.polyline([(2, 12.8), (4.2, 7.4), (14.2, 7.4), (12, 12.8)], closed=True)
    return i


@icon("actions", "document-save-symbolic", "document-save-as-symbolic")
def document_save():
    i = Icon()
    i.line(8, 2, 8, 10)
    i.polyline([(4.6, 6.8), (8, 10.2), (11.4, 6.8)])
    i.polyline([(2.4, 10.4), (2.4, 13.6), (13.6, 13.6), (13.6, 10.4)])
    return i


@icon("actions", "document-open-recent-symbolic", "preferences-system-time-symbolic",
      "appointment-soon-symbolic", "alarm-symbolic")
def clock():
    i = Icon()
    i.circle(8, 8, 6)
    i.polyline([(8, 4.4), (8, 8), (10.6, 9.6)])
    return i


@icon("actions", "media-playback-start-symbolic")
def play():
    i = Icon()
    i.polyline([(4.6, 2.8), (12.6, 8), (4.6, 13.2)], closed=True)
    return i


@icon("actions", "media-playback-pause-symbolic")
def pause():
    i = Icon()
    i.line(5.2, 3.2, 5.2, 12.8, width=1.6)
    i.line(10.8, 3.2, 10.8, 12.8, width=1.6)
    return i


@icon("actions", "media-playback-stop-symbolic")
def stop():
    return Icon().rrect(3.6, 3.6, 8.8, 8.8, 1.6)


@icon("actions", "media-skip-forward-symbolic")
def skip_forward():
    i = Icon()
    i.polyline([(3, 3.4), (10, 8), (3, 12.6)], closed=True)
    i.line(13, 3.4, 13, 12.6)
    return i


@icon("actions", "media-skip-backward-symbolic")
def skip_backward():
    i = Icon()
    i.polyline([(13, 3.4), (6, 8), (13, 12.6)], closed=True)
    i.line(3, 3.4, 3, 12.6)
    return i


@icon("actions", "media-record-symbolic")
def record():
    i = Icon()
    i.circle(8, 8, 5.6)
    i.disc(8, 8, 3)
    return i


@icon("actions", "format-text-bold-symbolic")
def bold():
    i = Icon()
    i.polyline([(4.6, 2.6), (4.6, 13.4)], width=1.6)
    i.polyline([(4.6, 2.6), (8.6, 2.6)], width=1.6)
    i.arc(8.6, 5.2, 2.6, 270, 450, width=1.6)
    i.polyline([(4.6, 7.8), (9.2, 7.8)], width=1.6)
    i.arc(9.2, 10.6, 2.8, 270, 450, width=1.6)
    i.polyline([(9.2, 13.4), (4.6, 13.4)], width=1.6)
    return i


@icon("actions", "starred-symbolic", "emblem-favorite-symbolic")
def star():
    i = Icon()
    points = []
    for index in range(10):
        radius = 6.4 if index % 2 == 0 else 2.7
        points.append(polar(8, 8.6, radius, -90 + index * 36))
    i.polyline(points, closed=True)
    return i


@icon("actions", "non-starred-symbolic")
def non_starred():
    return star()


@icon("actions", "mark-location-symbolic", "find-location-symbolic")
def location():
    i = Icon()
    i.arc(8, 6.4, 4.4, 150, 390)
    i.line(4.2, 8.6, 8, 14)
    i.line(11.8, 8.6, 8, 14)
    i.circle(8, 6.4, 1.6)
    return i


@icon("actions", "send-to-symbolic", "emblem-shared-symbolic")
def share():
    i = Icon()
    i.circle(12, 3.6, 1.8)
    i.circle(12, 12.4, 1.8)
    i.circle(4, 8, 1.8)
    i.line(5.6, 7.1, 10.4, 4.5)
    i.line(5.6, 8.9, 10.4, 11.5)
    return i


# System ---------------------------------------------------------------------
@icon("actions", "system-shutdown-symbolic")
def shutdown():
    i = Icon()
    i.arc(8, 8.6, 5.4, -55, 235)
    i.line(8, 1.8, 8, 7.4)
    return i


@icon("actions", "system-lock-screen-symbolic", "changes-prevent-symbolic")
def lock():
    i = Icon()
    i.rrect(3, 7.2, 10, 7, 1.8)
    i.arc(8, 6.8, 3.1, 180, 360)
    i.line(4.9, 6.8, 4.9, 7.2)
    i.line(11.1, 6.8, 11.1, 7.2)
    i.line(8, 9.8, 8, 11.6)
    return i


@icon("actions", "changes-allow-symbolic")
def unlock():
    i = Icon()
    i.rrect(3, 7.2, 10, 7, 1.8)
    i.arc(8, 5.4, 3.1, 180, 340)
    i.line(4.9, 5.4, 4.9, 7.2)
    i.line(8, 9.8, 8, 11.6)
    return i


@icon("actions", "system-log-out-symbolic", "application-exit-symbolic")
def log_out():
    i = Icon()
    i.polyline([(9, 2.4), (3.6, 2.4), (3.6, 13.6), (9, 13.6)])
    i.line(6.6, 8, 13.8, 8)
    i.polyline([(11, 5.2), (13.8, 8), (11, 10.8)])
    return i


def crescent(icon_: Icon, cx=8.0, cy=8.4, r1=5.6, ox=3.0, oy=-2.6, r2=4.4) -> Icon:
    c2x, c2y = cx + ox, cy + oy
    d = math.hypot(ox, oy)
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h = math.sqrt(max(r1 * r1 - a * a, 0))
    mx, my = cx + a * ox / d, cy + a * oy / d
    p1 = (mx + h * oy / d, my - h * ox / d)
    p2 = (mx - h * oy / d, my + h * ox / d)

    def angle(px, py, qx, qy):
        return math.degrees(math.atan2(qy - py, qx - px))

    a1, a2 = angle(cx, cy, *p1), angle(cx, cy, *p2)
    # Outer arc: the long way round the big circle, away from the inner circle.
    start, end = a2, a1
    while end <= start:
        end += 360
    if end - start < 180:
        start, end = a1, a2
        while end <= start:
            end += 360
    icon_.arc(cx, cy, r1, start, end)
    b1, b2 = angle(c2x, c2y, *p1), angle(c2x, c2y, *p2)
    start, end = b1, b2
    while end <= start:
        end += 360
    if end - start > 180:
        start, end = b2, b1
        while end <= start:
            end += 360
    icon_.arc(c2x, c2y, r2, start, end)
    return icon_


@icon("actions", "night-light-symbolic", "system-suspend-symbolic", "weather-clear-night-symbolic",
      "dark-mode-symbolic")
def night_light():
    return crescent(Icon())


@icon("status", "display-brightness-symbolic", "weather-clear-symbolic", "light-mode-symbolic")
def brightness():
    i = Icon()
    i.circle(8, 8, 2.8)
    for index in range(8):
        x0, y0 = polar(8, 8, 4.9, index * 45)
        x1, y1 = polar(8, 8, 6.4, index * 45)
        i.line(x0, y0, x1, y1)
    return i


def gear(icon_: Icon, cx=8.0, cy=8.0, teeth=8, outer=6.3, root=4.9, inner=2.1) -> Icon:
    points = []
    step = 360 / teeth
    for index in range(teeth):
        base = index * step - 90
        for offset, radius in ((-step * 0.42, root), (-step * 0.2, outer),
                               (step * 0.2, outer), (step * 0.42, root)):
            points.append(polar(cx, cy, radius, base + offset))
    icon_.polyline(points, closed=True)
    icon_.circle(cx, cy, inner)
    return icon_


@icon("categories", "preferences-system-symbolic", "emblem-system-symbolic",
      "applications-system-symbolic")
def settings():
    return gear(Icon())


@icon("categories", "preferences-other-symbolic", "emblem-settings-symbolic")
def sliders():
    i = Icon()
    for y, knob in ((3.4, 10.6), (8, 5), (12.6, 9)):
        i.line(2.2, y, knob - 1.8, y)
        i.line(knob + 1.8, y, 13.8, y)
        i.circle(knob, y, 1.6)
    return i


@icon("status", "dialog-information-symbolic", "help-about-symbolic", "help-info-symbolic")
def info():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.line(8, 7.2, 8, 11.4)
    i.disc(8, 4.9, 0.85)
    return i


@icon("status", "dialog-warning-symbolic")
def warning():
    i = Icon()
    i.polyline([(8, 2), (14.4, 13.4), (1.6, 13.4)], closed=True)
    i.line(8, 6.2, 8, 9.4)
    i.disc(8, 11.4, 0.8)
    return i


@icon("status", "dialog-error-symbolic")
def error():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.line(5.6, 5.6, 10.4, 10.4)
    i.line(10.4, 5.6, 5.6, 10.4)
    return i


@icon("status", "dialog-question-symbolic", "help-browser-symbolic")
def question():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.arc(8, 6.4, 2.2, 180, 380)
    i.line(10.07, 7.15, 8, 8.7)
    i.line(8, 8.7, 8, 9.2)
    i.disc(8, 11.4, 0.8)
    return i


# Network --------------------------------------------------------------------
WIFI_CENTER = (8.0, 13.0)
WIFI_RADII = (3.5, 6.6, 9.7)


def wifi(level: int, dot: bool = True, extra=None) -> Icon:
    i = Icon()
    cx, cy = WIFI_CENTER
    for index, radius in enumerate(WIFI_RADII):
        i.arc(cx, cy, radius, 225, 315, None if index < level else DIM)
    i.disc(cx, cy - 0.2, 1.15, None if dot else DIM)
    if extra:
        extra(i)
    return i


ICONS["network-wireless-signal-excellent-symbolic"] = ("status", wifi(3))
ICONS["network-wireless-signal-good-symbolic"] = ("status", wifi(2))
ICONS["network-wireless-signal-ok-symbolic"] = ("status", wifi(1))
ICONS["network-wireless-signal-weak-symbolic"] = ("status", wifi(0))
ICONS["network-wireless-signal-none-symbolic"] = ("status", wifi(0, dot=False))
ICONS["network-wireless-symbolic"] = ("status", wifi(3))
ICONS["network-wireless-connected-symbolic"] = ("status", wifi(3))
ICONS["network-wireless-no-route-symbolic"] = ("status", wifi(0, dot=False))


@icon("status", "network-wireless-acquiring-symbolic")
def wifi_acquiring():
    i = Icon()
    cx, cy = WIFI_CENTER
    for index, radius in enumerate(WIFI_RADII):
        i.arc(cx, cy, radius, 225, 315, None if index == 1 else DIM)
    i.disc(cx, cy - 0.2, 1.15, DIM)
    return i


def wifi_slashed():
    def draw(i: Icon):
        cx, cy = WIFI_CENTER
        for radius in WIFI_RADII:
            i.arc(cx, cy, radius, 225, 315, DIM)
        i.disc(cx, cy - 0.2, 1.15, DIM)
    return slashed(draw, 2.4, 3.0, 13.6, 14.2)


ICONS["network-wireless-offline-symbolic"] = ("status", wifi_slashed())
ICONS["network-wireless-disabled-symbolic"] = ("status", wifi_slashed())
ICONS["network-wireless-hardware-disabled-symbolic"] = ("status", wifi_slashed())


@icon("status", "network-wireless-encrypted-symbolic")
def wifi_encrypted():
    i = Icon()
    i.knockout([rounded_rect_path(8.2, 7.4, 8.2, 9.4, 1.6)])
    cx, cy = WIFI_CENTER
    for radius in WIFI_RADII:
        i.arc(cx, cy, radius, 225, 315)
    i.disc(cx, cy - 0.2, 1.15)
    i.end_knockout()
    i.box(10, 11.2, 5, 3.8, 0.9)
    i.arc(12.5, 11.2, 1.5, 180, 360, width=1.0)
    return i


def wired(opacity=None) -> Icon:
    i = Icon()
    i.rrect(5.8, 2, 4.4, 3.6, 0.9, opacity)
    i.line(8, 5.6, 8, 8.6, opacity)
    i.polyline([(3.6, 10.6), (3.6, 8.6), (12.4, 8.6), (12.4, 10.6)], opacity=opacity)
    i.rrect(1.6, 10.6, 4, 3.4, 0.9, opacity)
    i.rrect(10.4, 10.6, 4, 3.4, 0.9, opacity)
    return i


ICONS["network-wired-symbolic"] = ("status", wired())
ICONS["network-wired-acquiring-symbolic"] = ("status", wired(DIM))


def wired_slashed():
    def draw(i: Icon):
        part = wired(DIM)
        i.group.extend(part.parts)
    return slashed(draw)


ICONS["network-wired-disconnected-symbolic"] = ("status", wired_slashed())
ICONS["network-offline-symbolic"] = ("status", wired_slashed())
ICONS["network-wired-offline-symbolic"] = ("status", wired_slashed())


@icon("categories", "web-browser-symbolic", "network-workgroup-symbolic",
      "preferences-desktop-locale-symbolic", "applications-internet-symbolic")
def globe():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.line(1.8, 8, 14.2, 8)
    # Meridians through both poles, bulging to x = 5 and x = 11.
    i.arc(12.91, 8, 7.91, 128.4, 231.6)
    i.arc(3.09, 8, 7.91, -51.6, 51.6)
    return i


@icon("status", "airplane-mode-symbolic")
def airplane():
    i = Icon()
    i.polyline([(8, 1.6), (8.9, 3), (8.9, 6.4), (14, 9.4), (14, 10.6), (8.9, 9), (8.9, 12),
                (10.8, 13.4), (10.8, 14.4), (8, 13.6), (5.2, 14.4), (5.2, 13.4), (7.1, 12),
                (7.1, 9), (2, 10.6), (2, 9.4), (7.1, 6.4), (7.1, 3)], closed=True)
    return i


# Bluetooth ------------------------------------------------------------------
BT = [(4.4, 5), (11.4, 10.8), (8, 13.8), (8, 2.2), (11.4, 5.2), (4.4, 11)]


@icon("status", "bluetooth-active-symbolic", "bluetooth-symbolic")
def bluetooth():
    return Icon().polyline(BT)


@icon("status", "bluetooth-connected-symbolic")
def bluetooth_connected():
    i = Icon().polyline(BT)
    i.disc(2.2, 8, 0.9)
    i.disc(13.8, 8, 0.9)
    return i


def bluetooth_slashed():
    def draw(i: Icon):
        i.polyline(BT, opacity=DIM)
    return slashed(draw)


ICONS["bluetooth-disabled-symbolic"] = ("status", bluetooth_slashed())
ICONS["bluetooth-hardware-disabled-symbolic"] = ("status", bluetooth_slashed())


# Sound ----------------------------------------------------------------------
SPEAKER = [(1.6, 5.9), (4.2, 5.9), (7.6, 2.8), (7.6, 13.2), (4.2, 10.1), (1.6, 10.1)]


def speaker(waves: int, muted: bool = False) -> Icon:
    i = Icon()
    i.polyline(SPEAKER, closed=True)
    if muted:
        i.line(10.6, 5.8, 14.6, 9.8)
        i.line(14.6, 5.8, 10.6, 9.8)
        return i
    for index, radius in enumerate((3.4, 6.0)):
        i.arc(7.8, 8, radius, -48, 48, None if index < waves else DIM)
    return i


ICONS["audio-volume-high-symbolic"] = ("status", speaker(2))
ICONS["audio-volume-medium-symbolic"] = ("status", speaker(1))
ICONS["audio-volume-low-symbolic"] = ("status", speaker(0))
ICONS["audio-volume-muted-symbolic"] = ("status", speaker(0, True))
ICONS["audio-speakers-symbolic"] = ("devices", speaker(2))
ICONS["multimedia-volume-control-symbolic"] = ("devices", speaker(2))


def microphone(opacity=None) -> Icon:
    i = Icon()
    i.rrect(5.9, 1.6, 4.2, 7.8, 2.1, opacity)
    i.arc(8, 7.4, 4.3, 0, 180, opacity)
    i.line(8, 11.7, 8, 14.2, opacity)
    i.line(5.6, 14.2, 10.4, 14.2, opacity)
    return i


ICONS["microphone-sensitivity-high-symbolic"] = ("status", microphone())
ICONS["microphone-sensitivity-medium-symbolic"] = ("status", microphone())
ICONS["microphone-sensitivity-low-symbolic"] = ("status", microphone())
ICONS["audio-input-microphone-symbolic"] = ("devices", microphone())


def microphone_muted():
    def draw(i: Icon):
        i.group.extend(microphone(DIM).parts)
    return slashed(draw)


ICONS["microphone-sensitivity-muted-symbolic"] = ("status", microphone_muted())
ICONS["microphone-disabled-symbolic"] = ("status", microphone_muted())


# Battery --------------------------------------------------------------------
BOLT = [(8.9, 3.6), (5.4, 8.6), (7.9, 8.6), (7.1, 12.4), (10.6, 7.4), (8.1, 7.4)]


def battery(level: int | None, charging: bool = False) -> Icon:
    i = Icon()
    i.rrect(1.6, 4.4, 11.8, 7.2, 1.7)
    i.box(13.9, 6.4, 1.3, 3.2, 0.5)
    if level is None:
        i.line(7.5, 6.4, 7.5, 7.4)
        i.disc(7.5, 9.4, 0.7)
        return i
    width = 8.8 * level / 100
    if charging:
        expanded = [(8 + (x - 8) * 1.55, 8 + (y - 8) * 1.4) for x, y in BOLT]
        i.knockout(["M" + "L".join(pt(x, y) for x, y in expanded) + "Z"])
    if width > 0.2:
        i.box(3.1, 5.9, max(width, 1.2), 4.2, 0.6)
    if charging:
        i.end_knockout()
        i.poly(BOLT)
    return i


for _level in range(0, 101, 10):
    ICONS[f"battery-level-{_level}-symbolic"] = ("status", battery(_level))
    ICONS[f"battery-level-{_level}-charging-symbolic"] = ("status", battery(_level, True))
ICONS["battery-level-100-charged-symbolic"] = ("status", battery(100, True))
ICONS["battery-full-charged-symbolic"] = ("status", battery(100, True))
ICONS["battery-symbolic"] = ("status", battery(60))
ICONS["battery-full-symbolic"] = ("status", battery(100))
ICONS["battery-good-symbolic"] = ("status", battery(70))
ICONS["battery-low-symbolic"] = ("status", battery(30))
ICONS["battery-caution-symbolic"] = ("status", battery(10))
ICONS["battery-empty-symbolic"] = ("status", battery(0))
ICONS["battery-missing-symbolic"] = ("status", battery(None))


# Power profiles -------------------------------------------------------------
@icon("status", "power-profile-balanced-symbolic")
def balanced():
    i = Icon()
    i.line(8, 2.4, 8, 13.6)
    i.line(2.4, 5, 13.6, 5)
    i.polyline([(1.6, 10), (3.6, 5.4), (5.6, 10)])
    i.polyline([(10.4, 10), (12.4, 5.4), (14.4, 10)])
    i.line(1.6, 10, 5.6, 10)
    i.line(10.4, 10, 14.4, 10)
    i.line(5.6, 13.6, 10.4, 13.6)
    return i


@icon("status", "power-profile-performance-symbolic")
def performance():
    i = Icon()
    i.arc(8, 10, 6, 180, 360)
    i.line(8, 10, 11.4, 5.6)
    i.disc(8, 10, 1.2)
    i.line(2, 10, 2, 12.4)
    i.line(14, 10, 14, 12.4)
    return i


@icon("status", "power-profile-power-saver-symbolic")
def power_saver():
    i = Icon()
    i.curve((3, 13), (2.4, 6), (7, 2.4), (13.4, 2.6))
    i.curve((13.4, 2.6), (13.8, 9.4), (9.6, 13.4), (3, 13))
    i.line(3, 13, 9, 7)
    return i


# Notifications --------------------------------------------------------------
def bell(opacity=None) -> Icon:
    i = Icon()
    i.arc(8, 6.6, 3.9, 180, 360, opacity)
    i.polyline([(4.1, 6.6), (4.1, 9.8), (2.6, 11.8), (13.4, 11.8), (11.9, 9.8), (11.9, 6.6)],
               opacity=opacity)
    i.arc(8, 12.6, 1.7, 15, 165, opacity)
    i.line(8, 1.6, 8, 2.7, opacity)
    return i


ICONS["notifications-symbolic"] = ("status", bell())
ICONS["preferences-system-notifications-symbolic"] = ("categories", bell())
ICONS["notification-symbolic"] = ("status", bell())


def bell_slashed():
    def draw(i: Icon):
        i.group.extend(bell(DIM).parts)
    return slashed(draw)


ICONS["notifications-disabled-symbolic"] = ("status", bell_slashed())


# Devices --------------------------------------------------------------------
@icon("devices", "video-display-symbolic", "preferences-desktop-display-symbolic",
      "computer-symbolic")
def display():
    i = Icon()
    i.rrect(1.8, 2.4, 12.4, 8.4, 1.6)
    i.line(8, 10.8, 8, 13.4)
    i.line(4.8, 13.4, 11.2, 13.4)
    return i


@icon("devices", "input-keyboard-symbolic")
def keyboard():
    i = Icon()
    i.rrect(1.4, 3.6, 13.2, 8.8, 1.6)
    for row, y in enumerate((6.2, 8.4)):
        for column in range(5):
            i.disc(3.8 + column * 2.1 + row * 0.6, y, 0.6)
    i.line(5.4, 10.4, 10.6, 10.4)
    return i


@icon("devices", "input-mouse-symbolic")
def mouse():
    i = Icon()
    i.rrect(4, 1.8, 8, 12.4, 4)
    i.line(8, 3.8, 8, 6.2)
    return i


@icon("devices", "input-touchpad-symbolic")
def touchpad():
    i = Icon()
    i.rrect(1.8, 2.4, 12.4, 11.2, 1.8)
    i.line(1.8, 10, 14.2, 10)
    i.line(8, 10, 8, 13.6)
    return i


@icon("devices", "drive-harddisk-symbolic", "drive-harddisk-system-symbolic")
def harddisk():
    i = Icon()
    i.rrect(1.8, 4.2, 12.4, 7.6, 1.8)
    i.line(1.8, 8.6, 14.2, 8.6)
    i.disc(11.4, 10.2, 0.75)
    return i


@icon("devices", "media-removable-symbolic", "drive-removable-media-symbolic")
def removable():
    i = Icon()
    i.rrect(4.2, 6.2, 7.6, 8, 1.4)
    i.polyline([(5.6, 6.2), (5.6, 1.8), (10.4, 1.8), (10.4, 6.2)])
    i.line(7.2, 3.6, 7.2, 4.4)
    i.line(8.8, 3.6, 8.8, 4.4)
    return i


@icon("devices", "camera-photo-symbolic", "applets-screenshooter-symbolic")
def camera():
    i = Icon()
    i.polyline([(1.8, 5), (4.6, 5), (5.8, 3), (10.2, 3), (11.4, 5), (14.2, 5), (14.2, 13),
                (1.8, 13)], closed=True)
    i.circle(8, 8.8, 2.5)
    return i


@icon("devices", "camera-web-symbolic", "camera-video-symbolic")
def webcam():
    i = Icon()
    i.circle(8, 6.6, 4.8)
    i.circle(8, 6.6, 1.8)
    i.polyline([(5, 14), (8, 11.4), (11, 14)])
    i.line(4.4, 14, 11.6, 14)
    return i


@icon("devices", "printer-symbolic")
def printer():
    i = Icon()
    i.polyline([(4.4, 5.6), (4.4, 2), (11.6, 2), (11.6, 5.6)])
    i.rrect(1.8, 5.6, 12.4, 6.2, 1.6)
    i.rrect(4.4, 9.6, 7.2, 4.6, 0.8)
    return i


@icon("devices", "phone-symbolic", "smartphone-symbolic")
def phone():
    i = Icon()
    i.rrect(4.4, 1.6, 7.2, 12.8, 1.8)
    i.line(7, 12, 9, 12)
    return i


@icon("devices", "audio-headphones-symbolic", "audio-headset-symbolic")
def headphones():
    i = Icon()
    i.arc(8, 8.8, 5.8, 180, 360)
    i.rrect(2.2, 9.4, 2.6, 4.6, 1)
    i.rrect(11.2, 9.4, 2.6, 4.6, 1)
    return i


# Places ---------------------------------------------------------------------
def folder(extra=None) -> Icon:
    i = Icon()
    i.polyline([(1.8, 12.4), (1.8, 3.6), (6, 3.6), (7.4, 5.2), (14.2, 5.2), (14.2, 12.4)],
               closed=True)
    i.line(1.8, 7.2, 14.2, 7.2)
    if extra:
        extra(i)
    return i


ICONS["folder-symbolic"] = ("places", folder())
ICONS["folder-open-symbolic"] = ("places", folder())
ICONS["inode-directory-symbolic"] = ("mimetypes", folder())
ICONS["folder-documents-symbolic"] = ("places", folder(lambda i: i.line(5.6, 9.8, 10.4, 9.8)))
ICONS["folder-download-symbolic"] = ("places", folder(
    lambda i: i.polyline([(6, 8.8), (8, 10.6), (10, 8.8)])))
ICONS["folder-pictures-symbolic"] = ("places", folder(
    lambda i: i.polyline([(5, 11), (7.2, 8.8), (8.6, 10.2), (9.6, 9.2), (11, 11)])))
ICONS["folder-music-symbolic"] = ("places", folder(
    lambda i: (i.disc(7, 10.6, 1.0), i.line(7.8, 10.4, 7.8, 8.4), i.line(7.8, 8.4, 9.8, 8.8))))
ICONS["folder-videos-symbolic"] = ("places", folder(
    lambda i: i.polyline([(6.8, 8.6), (9.8, 9.8), (6.8, 11)], closed=True)))
ICONS["folder-publicshare-symbolic"] = ("places", folder(
    lambda i: (i.circle(8, 9.6, 1.4))))
ICONS["folder-templates-symbolic"] = ("places", folder(
    lambda i: i.rrect(6.4, 8.6, 3.2, 2.6, 0.4)))
ICONS["folder-remote-symbolic"] = ("places", folder(
    lambda i: (i.line(5.6, 9.8, 10.4, 9.8), i.disc(8, 9.8, 0.8))))


@icon("places", "user-home-symbolic", "go-home-symbolic")
def home():
    i = Icon()
    i.polyline([(1.8, 7.6), (8, 2.2), (14.2, 7.6)])
    i.polyline([(3.4, 6.4), (3.4, 13.8), (12.6, 13.8), (12.6, 6.4)])
    i.polyline([(6.6, 13.8), (6.6, 10), (9.4, 10), (9.4, 13.8)])
    return i


@icon("places", "user-desktop-symbolic")
def desktop():
    return display()


@icon("places", "user-bookmarks-symbolic", "bookmark-new-symbolic")
def bookmark():
    return Icon().polyline([(4, 1.8), (12, 1.8), (12, 14), (8, 10.6), (4, 14)], closed=True)


# Mime types -----------------------------------------------------------------
def document(extra=None) -> Icon:
    i = Icon()
    i.polyline([(3, 1.8), (9.4, 1.8), (13, 5.4), (13, 14.2), (3, 14.2)], closed=True)
    i.polyline([(9.4, 1.8), (9.4, 5.4), (13, 5.4)])
    if extra:
        extra(i)
    return i


def lines(i: Icon):
    i.line(5.6, 8, 10.4, 8)
    i.line(5.6, 10.6, 10.4, 10.6)


ICONS["text-x-generic-symbolic"] = ("mimetypes", document(lines))
ICONS["x-office-document-symbolic"] = ("mimetypes", document(lines))
ICONS["emblem-documents-symbolic"] = ("emblems", document(lines))
ICONS["document-viewer-symbolic"] = ("mimetypes", document(lines))
ICONS["text-x-script-symbolic"] = ("mimetypes", document(
    lambda i: (i.polyline([(6.8, 7.6), (5.2, 9.4), (6.8, 11.2)]),
               i.polyline([(9.2, 7.6), (10.8, 9.4), (9.2, 11.2)]))))
ICONS["application-x-executable-symbolic"] = ("mimetypes", document(
    lambda i: gear(i, 8, 9.6, 6, 2.9, 2.2, 0.8)))
ICONS["application-x-addon-symbolic"] = ICONS["application-x-executable-symbolic"]
ICONS["package-x-generic-symbolic"] = ("mimetypes", Icon()
                                       .polyline([(8, 1.8), (14, 4.8), (14, 11.2), (8, 14.2),
                                                  (2, 11.2), (2, 4.8)], closed=True)
                                       .polyline([(2, 4.8), (8, 7.8), (14, 4.8)])
                                       .line(8, 7.8, 8, 14.2)
                                       .line(5, 3.3, 11, 6.3, DIM))


@icon("mimetypes", "image-x-generic-symbolic", "applications-graphics-symbolic", "preferences-desktop-wallpaper-symbolic")
def image():
    i = Icon()
    i.rrect(1.8, 2.4, 12.4, 11.2, 1.8)
    i.circle(10.6, 5.8, 1.3)
    i.polyline([(1.8, 11.4), (5.8, 7.4), (10, 11.6), (11.4, 10.2), (14.2, 13)])
    return i


@icon("mimetypes", "audio-x-generic-symbolic", "multimedia-player-symbolic")
def music():
    i = Icon()
    i.circle(4.4, 11.8, 2.2)
    i.circle(11.6, 10.2, 2.2)
    i.line(6.6, 11.8, 6.6, 3.6)
    i.line(13.8, 10.2, 13.8, 2)
    i.polyline([(6.6, 3.6), (13.8, 2)])
    i.polyline([(6.6, 6.2), (13.8, 4.6)])
    return i


@icon("mimetypes", "video-x-generic-symbolic")
def video():
    i = Icon()
    i.rrect(1.6, 3, 9, 10, 1.6)
    i.polyline([(10.6, 6.6), (14.4, 4.4), (14.4, 11.6), (10.6, 9.4)])
    return i


@icon("mimetypes", "x-office-calendar-symbolic",
      "office-calendar-symbolic")
def calendar():
    i = Icon()
    i.rrect(2, 3.2, 12, 11, 1.8)
    i.line(2, 6.6, 14, 6.6)
    i.line(5.2, 1.6, 5.2, 4.2)
    i.line(10.8, 1.6, 10.8, 4.2)
    for x, y in ((5.2, 9.4), (8, 9.4), (10.8, 9.4), (5.2, 11.8), (8, 11.8)):
        i.disc(x, y, 0.7)
    return i


@icon("apps", "accessories-calculator-symbolic")
def calculator():
    i = Icon()
    i.rrect(3, 1.6, 10, 12.8, 1.8)
    i.rrect(5, 3.6, 6, 2.6, 0.5)
    for row in range(3):
        for column in range(3):
            i.disc(5.6 + column * 2.4, 8.4 + row * 2, 0.65)
    return i


@icon("apps", "utilities-terminal-symbolic")
def terminal():
    i = Icon()
    i.rrect(1.6, 2.4, 12.8, 11.2, 1.8)
    i.polyline([(4.4, 6), (6.6, 8), (4.4, 10)])
    i.line(8.2, 10.4, 11.4, 10.4)
    return i


@icon("apps", "accessories-text-editor-symbolic")
def text_editor():
    i = document()
    i.line(5.6, 8, 10.4, 8)
    i.line(5.6, 10.6, 8.4, 10.6)
    return i


@icon("apps", "system-file-manager-symbolic")
def files():
    return folder()


@icon("apps", "accessories-notes-symbolic")
def notes():
    i = Icon()
    i.polyline([(2.6, 2), (13.4, 2), (13.4, 10.2), (9.6, 14), (2.6, 14)], closed=True)
    i.polyline([(9.6, 14), (9.6, 10.2), (13.4, 10.2)])
    i.line(5.2, 5.4, 10.8, 5.4)
    i.line(5.2, 8, 9, 8)
    return i


@icon("apps", "package-x-generic-alt-symbolic")
def archive():
    i = Icon()
    i.rrect(2, 2.4, 12, 3.4, 1)
    i.polyline([(3, 5.8), (3, 13.6), (13, 13.6), (13, 5.8)])
    i.line(6.4, 8.4, 9.6, 8.4)
    return i


@icon("apps", "system-software-install-symbolic")
def store():
    i = Icon()
    i.polyline([(2.2, 5.4), (13.8, 5.4), (12.8, 14), (3.2, 14)], closed=True)
    i.arc(8, 5.4, 2.8, 180, 360)
    return i


@icon("apps", "software-update-available-symbolic", "system-software-update-symbolic")
def update():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.line(8, 4.6, 8, 11.4)
    i.polyline([(5.2, 7.4), (8, 4.6), (10.8, 7.4)])
    return i


# Weather --------------------------------------------------------------------
def circle_intersection(c1, r1, c2, r2, upper=True):
    dx, dy = c2[0] - c1[0], c2[1] - c1[1]
    d = math.hypot(dx, dy)
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h = math.sqrt(max(r1 * r1 - a * a, 0))
    mx, my = c1[0] + a * dx / d, c1[1] + a * dy / d
    first = (mx + h * dy / d, my - h * dx / d)
    second = (mx - h * dy / d, my + h * dx / d)
    pick = min if upper else max
    return pick(first, second, key=lambda p: p[1])


def heading(center, point) -> float:
    return math.degrees(math.atan2(point[1] - center[1], point[0] - center[0]))


def forward(start: float, end: float) -> tuple[float, float]:
    while end <= start:
        end += 360
    return start, end


CLOUD = (((4.8, 9.6), 2.4), ((8.4, 7.8), 3.6), ((11.6, 9.8), 2.2))
CLOUD_BASE = 12.0


def cloud(icon_: Icon, dx=0.0, dy=0.0, opacity=None) -> Icon:
    circles = [((x + dx, y + dy), r) for (x, y), r in CLOUD]
    (lc, lr), (mc, mr), (rc, rr) = circles
    lm = circle_intersection(lc, lr, mc, mr)
    mrp = circle_intersection(mc, mr, rc, rr)
    base = CLOUD_BASE + dy
    icon_.arc(lc[0], lc[1], lr, *forward(90, heading(lc, lm)), opacity)
    icon_.arc(mc[0], mc[1], mr, *forward(heading(mc, lm), heading(mc, mrp)), opacity)
    icon_.arc(rc[0], rc[1], rr, *forward(heading(rc, mrp), 90), opacity)
    icon_.line(lc[0], base, rc[0], base, opacity)
    return icon_


def cloud_clearance(dx=0.0, dy=0.0, margin=1.1) -> list[str]:
    shapes = [circle_path(x + dx, y + dy, r + margin) for (x, y), r in CLOUD]
    shapes.append(rounded_rect_path(CLOUD[0][0][0] + dx, 9.0 + dy,
                                    CLOUD[2][0][0] - CLOUD[0][0][0], CLOUD_BASE - 9.0 + margin, 0))
    return shapes


@icon("status", "weather-overcast-symbolic")
def overcast():
    return cloud(Icon())


@icon("status", "weather-few-clouds-symbolic")
def few_clouds():
    i = Icon()
    dx, dy = 1.4, 1.6
    i.knockout(cloud_clearance(dx, dy))
    i.circle(5.4, 5.6, 2.3)
    for angle in (150, 195, 240, 285, 330):
        x0, y0 = polar(5.4, 5.6, 3.8, angle)
        x1, y1 = polar(5.4, 5.6, 4.9, angle)
        i.line(x0, y0, x1, y1)
    i.end_knockout()
    return cloud(i, dx, dy)


@icon("status", "weather-showers-symbolic", "weather-showers-scattered-symbolic")
def showers():
    i = cloud(Icon(), 0, -2)
    for x in (5.4, 8.4, 11.4):
        i.line(x, 12.4, x - 0.8, 14.2)
    return i


@icon("status", "weather-storm-symbolic")
def storm():
    i = cloud(Icon(), 0, -2.2)
    i.polyline([(8.8, 11), (7, 13), (9, 13), (7.6, 15)])
    return i


@icon("status", "weather-snow-symbolic")
def snow():
    i = cloud(Icon(), 0, -2)
    for x, y in ((5.4, 13), (8.4, 14.4), (11.4, 13)):
        i.disc(x, y, 0.75)
    return i


@icon("status", "weather-fog-symbolic")
def fog():
    i = Icon()
    for y, x0, x1 in ((4.6, 3, 13), (8, 1.8, 11.4), (11.4, 4.6, 14.2)):
        i.line(x0, y, x1, y)
    return i


# People ---------------------------------------------------------------------
@icon("status", "avatar-default-symbolic", "system-users-symbolic", "user-info-symbolic",
      "user-available-symbolic", "person-symbolic")
def person():
    i = Icon()
    i.circle(8, 5.2, 2.9)
    i.arc(8, 15.4, 5.8, 200, 340)
    return i


@icon("categories", "preferences-desktop-accessibility-symbolic",
      "org.gnome.Settings-accessibility-symbolic")
def accessibility():
    i = Icon()
    i.disc(8, 2.8, 1.3)
    i.polyline([(2.6, 5.6), (8, 6.6), (13.4, 5.6)])
    i.line(8, 6.6, 8, 9.6)
    i.polyline([(5.4, 14), (8, 9.6), (10.6, 14)])
    return i


@icon("categories", "preferences-system-privacy-symbolic", "security-high-symbolic")
def privacy():
    i = Icon()
    i.polyline([(8, 1.8), (13.4, 3.8), (13.4, 7.6)])
    i.polyline([(8, 1.8), (2.6, 3.8), (2.6, 7.6)])
    i.curve((2.6, 7.6), (2.6, 11), (5.4, 13.2), (8, 14.2))
    i.curve((13.4, 7.6), (13.4, 11), (10.6, 13.2), (8, 14.2))
    return i


@icon("categories", "preferences-desktop-theme-symbolic", "preferences-color-symbolic",
      "applications-graphics-alt-symbolic")
def theme():
    i = Icon()
    i.circle(8, 8, 6.2)
    i.fill("M8 3.6A4.4 4.4 0 0 1 8 12.4Z")
    return i


@icon("categories", "preferences-desktop-default-applications-symbolic")
def default_apps():
    i = Icon()
    i.rrect(2, 2, 5, 5, 1.4)
    i.rrect(9, 2, 5, 5, 1.4)
    i.rrect(2, 9, 5, 5, 1.4)
    i.polyline([(9.2, 11.6), (10.8, 13.2), (14, 9.8)])
    return i


@icon("categories", "preferences-desktop-apps-symbolic", "applications-other-symbolic")
def apps():
    i = Icon()
    for x, y in ((2, 2), (9, 2), (2, 9), (9, 9)):
        i.rrect(x + 0.5, y + 0.5, 4.5, 4.5, 1.4)
    return i


@icon("categories", "preferences-desktop-keyboard-shortcuts-symbolic")
def shortcuts():
    return keyboard()


@icon("categories", "applications-utilities-symbolic", "applications-accessories-symbolic")
def utilities():
    i = Icon()
    i.rrect(1.8, 5.6, 12.4, 8, 1.6)
    i.polyline([(5.6, 5.6), (5.6, 3), (10.4, 3), (10.4, 5.6)])
    i.line(1.8, 9.2, 14.2, 9.2)
    i.line(8, 8.2, 8, 10.2)
    return i


@icon("categories", "applications-multimedia-symbolic")
def multimedia():
    return play()


@icon("categories", "applications-office-symbolic")
def office():
    return document(lines)


@icon("categories", "applications-games-symbolic", "input-gaming-symbolic")
def games():
    i = Icon()
    i.rrect(1.6, 4.4, 12.8, 7.6, 3.8)
    i.line(4, 8.2, 6.8, 8.2)
    i.line(5.4, 6.8, 5.4, 9.6)
    i.disc(10.4, 7.4, 0.75)
    i.disc(11.8, 9, 0.75)
    return i


@icon("categories", "applications-science-symbolic", "applications-education-symbolic")
def science():
    i = Icon()
    i.polyline([(6, 1.8), (10, 1.8)])
    i.polyline([(6.8, 1.8), (6.8, 6.4), (2.6, 13.4), (13.4, 13.4), (9.2, 6.4), (9.2, 1.8)])
    i.line(4.6, 10.2, 11.4, 10.2)
    return i


@icon("categories", "applications-development-symbolic", "applications-engineering-symbolic")
def development():
    i = Icon()
    i.polyline([(5, 4.4), (1.8, 8), (5, 11.6)])
    i.polyline([(11, 4.4), (14.2, 8), (11, 11.6)])
    i.line(9.4, 2.8, 6.6, 13.2)
    return i


def write(names: list[str] | None = None) -> int:
    count = 0
    for name, (context, drawing) in sorted(ICONS.items()):
        if names and name not in names:
            continue
        if callable(drawing):
            drawing = drawing()
        directory = OUTPUT / context
        directory.mkdir(parents=True, exist_ok=True)
        (directory / f"{name}.svg").write_text(drawing.svg(), encoding="utf-8", newline="\n")
        count += 1
    return count


if __name__ == "__main__":
    print(f"Wrote {write()} symbolic icons to {OUTPUT.relative_to(ROOT)}")
