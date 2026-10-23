---
title: Software, with Debian and Flathub in one catalog
description: Asena’s software app and the package service behind it are ready. System updates install on restart.
permalink: software
key: software
date: 2026-10-23T14:00:00+03:00
---

Software was one of the five apps hidden in the 21 October audit. It is back in the menu, and it really works.

## The catalog

Debian’s app catalog (DEP-11) and each Flatpak remote are read separately; the same app is merged into one entry and you pick where to install it from. Search handles Turkish letters correctly and lists follow Turkish sorting. Screenshots download only when you open an app’s page.

## Debian packages

A new package service, `org.asenalinux.Packages`, installs and removes packages. You first see what will change, then polkit asks for your password. The service refuses invalid names, removing protected packages such as `sudo`, and stale plans, and runs only one APT job at a time.

## Flatpak

Flatpak apps install for your user. The permissions they ask for are shown before installing. You turn Flathub on yourself, with a retry if the network fails. A Cancel button is there while installs, removals and updates run.

## Updates

Once a day Asena checks, downloads updates in the background and shows a single notification. System updates install on restart, before the desktop starts, and the result shows in your next session. Flatpak updates happen during the session. There is no `unattended-upgrades` and the computer never restarts on its own.

## What was tested

In the virtual machine, the `hello` package was installed and removed. Seven pending Debian updates (`tzdata` 2026c to 2026e among them) installed on restart. After a test package whose install script fails on purpose, the system still booted, and the package service detected the half-finished job and repaired it.
