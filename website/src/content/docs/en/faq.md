---
title: FAQ
description: Price, downloads, GNOME and KDE, Debian, X11, Flatpak, privacy and source code.
permalink: faq
key: faq
order: 8
---

## Does Asena cost anything?

No. Asena is free software: the source code is licensed GPL-3.0-or-later and the wallpapers CC BY-SA 4.0.

## When can I download it?

The first installable ISO has not been built and we are not giving a date. When it is ready we will announce it on the [Download](/en/download/) page and in the [news feed](/en/news/rss.xml).

## Is it based on GNOME or KDE?

No. The panel, menu, notifications, lock and login screens are Asena’s own shell, running on Wayfire. The apps use GTK4 and libadwaita, so they look at home next to GNOME apps. GNOME Shell extensions do not work.

## Is it a version of Debian?

Asena is built on Debian 13 but is not part of the Debian project. Its system identity is `ID=asena` with `ID_LIKE=debian`; the release codename stays `trixie`, so third-party repositories written for Debian work. Install scripts that recognize distributions only by the `ID` field may not recognize Asena.

## Do X11 apps work?

Yes, through XWayland. There is no separate X11 session.

## Do Flatpak apps work?

Yes. One known difference: our Wayfire setup lacks the `wp-security-context-v1` protocol. Apps run, but the compositor cannot keep a sandboxed app away from privileged protocols such as screen capture.

## Why Wayfire?

To really blur what is behind the panel and menus. Wayfire 0.9 in Debian 13 can do that. The shell talks to the compositor only through standard Wayland protocols, so the shell code did not change in the move, and the compositor can be swapped again if needed.

## Why is there no Terminal in the menu?

Asena’s own terminal is not ready yet. Foot comes installed instead; open it from the menu or with <kbd>Super</kbd> + <kbd>Enter</kbd>.

## Is usage data collected?

Asena’s own components collect no usage data. They go online only to do a job: Debian repositories when checking for updates, and Debian and (if you turned it on) Flathub servers for the catalog and screenshots in Software. Apps such as Firefox have their own settings.

## Where is the source code?

The public source repository has not been opened yet. We will announce it here when it is.
