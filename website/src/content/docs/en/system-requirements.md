---
title: System requirements
description: The hardware and installation the first release will support.
permalink: system-requirements
key: system-requirements
order: 2
---

This page lists the decisions made for the first release. The first ISO has not been built yet, so minimum memory and disk sizes have not been measured; they will be added here once they are.

## Processor and boot

- A 64-bit x86 processor (amd64).
- UEFI. Legacy BIOS mode will not be supported.
- The default kernel is XanMod LTS, which needs a processor with x86-64-v2, roughly anything from 2009 onwards. On an older processor XanMod will not start, and since that is not a load error the menu does not fall back on its own: pick the Debian kernel from the boot menu.
- Debian’s signed kernel always stays installed as a fallback.

## Secure Boot

Asena uses Debian’s signed shim, GRUB and kernel. With Secure Boot on, the Debian kernel boots; with it off, XanMod does. This was tried twice in a virtual machine and has not been tried on real hardware yet. Firmware that replaces Microsoft’s 2011 certificates with the 2023 ones still needs to be checked.

## Graphics

The desktop draws with OpenGL ES; blur and window animations run on the graphics card. On an old card, or one with a troublesome driver, turning off **Transparency effects** in Settings › Appearance and animations in Settings › Accessibility lightens the load.

## Memory

The performance targets were set against a reference machine with 4 cores, 8 GB of memory, integrated graphics and an SSD. Compressed memory swap (zram) is on by default.

## Installation

- The first release installs Asena as the only system on an empty disk.
- Installing next to Windows or another system (dual boot) and disk encryption are not in the first release.
- The installer will be Calamares. It asks for language, keyboard and time zone, and does not go online to guess your location.
- The root account is locked; the first user you create during installation is the administrator.

## Firmware

Debian’s `non-free-firmware` component and Intel and AMD processor microcode come ready in the live system and carry over to the installation. Closed drivers (`non-free`) are not added.
