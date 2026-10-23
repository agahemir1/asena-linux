---
title: We moved to Wayfire, and the blur is real
description: Asena now runs on Wayfire 0.9 as its only compositor. What sits behind the panel and menus is truly blurred.
permalink: moving-to-wayfire
key: wayfire
date: 2026-10-21T18:00:00+03:00
---

The Odak design asked for frosted glass on the panel and transient surfaces: the window behind should really blur. GTK cannot do that; the compositor draws what is behind windows.

## The experiment

We tried the blur plugin of Wayfire 0.9.0, as packaged in Debian 13, with the Dual Kawase method. The panel, start menu, control center and notification panel all blurred what was behind them. Not one line of shell code changed, because the shell talks to the compositor only through standard Wayland protocols. Getting the same on labwc would have meant packaging newer labwc, wlroots and SceneFX ourselves.

## The decision

On 21 October Wayfire became the only compositor and labwc was removed. Settings › Appearance gained a **Transparency effects** switch. When it is off, or in high contrast, the surfaces turn opaque and the blur plugin is not loaded at all. asena-daemon writes shortcuts, input settings and the animation preference into the Wayfire configuration, and they apply immediately.

## What was tested

In the virtual machine the session, lock screen, login screen and a Flathub app (file chooser and document portal included) all worked. With software rendering (llvmpipe), CPU use during motion rose by about one and a half times. Nothing has been measured on a real graphics card yet.

## A known difference

Our Wayfire setup lacks the `wp-security-context-v1` protocol. Flatpak apps work, but the compositor cannot keep a sandboxed app away from privileged protocols such as screen capture. We are noting it plainly.
