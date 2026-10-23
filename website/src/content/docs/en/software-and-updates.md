---
title: Software and updates
description: Installing Debian packages and Flatpak apps with Software, updates and repairs.
permalink: software-and-updates
key: software-and-updates
order: 6
---

## The Software app

Software has four parts: **Explore** (categories), search, **Installed** and **Updates**. Click an app to see its description, license, website and screenshots.

The catalog comes from two sources: Debian’s app catalog (DEP-11) and, if enabled, Flathub. When the same app exists in both, it shows as one entry and you choose where to install it from. Packages without an app record, such as command-line tools, do not appear in the catalog; install those with `apt`, as on any Debian system.

## Installing and removing Debian packages

Software first shows what will change on the system: the packages to install, upgrade and remove. If you agree, it asks for your password. Asena’s package service does the work; polkit checks the permission.

The service refuses some requests: invalid package names, removing protected packages the system needs to keep working (such as `sudo`), and plans that went stale after you approved them. Only one package job runs at a time.

## Flatpak and Flathub

You turn Flathub on yourself in Software; it is never added behind your back. Flatpak apps install for your user. The permissions an app asks for (your files, the network, devices and so on) are listed before you install it. A Cancel button is always there while an install, removal or update runs.

## Updates

- Once a day Asena checks for updates, downloads any in the background and shows a single notification.
- System updates install when you restart, before the desktop starts. The **Restart and install** button on the Updates page starts this right away.
- The result shows in Software in your next session.
- Flatpak apps and runtimes update during your session.
- The computer never restarts on its own, and `unattended-upgrades` is not installed.

## Repair

If a package job is cut off, for example by a power cut, Software notices and shows a repair bar. Repairing also asks for your password.

## Command line

Asena is a Debian system and `apt` works as usual. The first user created during installation is in the `sudo` group.
