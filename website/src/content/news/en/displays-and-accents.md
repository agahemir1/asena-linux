---
title: Display layout, accent colors and Turkish F
description: Settings › Displays changes the layout and reverts it after 15 seconds unless confirmed. The accent color now reaches Flatpak apps too.
permalink: displays-and-accents
key: displays-and-accents
date: 2026-10-22
---

## Displays

Settings › Displays now changes the primary display, resolution, refresh rate, scale, rotation and the position of each screen. A change is tried first. If you do not confirm within 15 seconds, if Settings closes or if the screens change, the old layout comes back. A confirmed layout is saved for that set of screens and applied again when the same screens are connected. asena-daemon alone owns this work; Settings only asks.

We hit two problems on the way. GTK 4.18 reports stale geometry for a display whose scale returns to 1, so the shell reads display geometry from the compositor instead. wlroots 0.18 also crashed on virtual outputs without a mode; we took the upstream fix as a patched package. Acceptance testing with real monitors, including plugging and unplugging, is under way.

## Accent colors

asena-daemon now provides the backend of the Settings portal for Flatpak apps. Dark style and the accent color reach those apps instantly. The folder icons, and the Files and Settings icons, follow the accent you pick; each accent has its own drawn icon theme.

## Turkish F

Wayfire shortcuts look at the physical key, so on a Turkish F keyboard <kbd>Super</kbd> + <kbd>A</kbd> landed on a different key. Letter shortcuts are now mapped to whichever key types that letter in your first keyboard layout.
