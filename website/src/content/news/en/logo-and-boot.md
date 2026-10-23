---
title: The logo and the boot, a tile from a Seljuk star
description: Asena’s logo was chosen and now appears from boot to desktop. XanMod LTS became the default kernel.
permalink: logo-and-boot
key: logo-and-boot
date: 2026-10-23T10:00:00+03:00
---

## The logo

The logo is a glazed tile drawn from the eight-pointed star in the start menu: a turquoise glaze and two interlaced squares with a cream band between them. It appears in:

- the GRUB menu (“Asena GNU/Linux”) and the live ISO’s boot menu
- the Plymouth boot splash
- fastfetch and `/etc/issue` at the console login
- the login and lock screens, and Settings › About

The logo comes as three files: a color SVG, a single-color version and an ASCII version for the terminal.

One detail: Debian’s signed GRUB looks for its configuration only under `EFI/debian`. Had we renamed that directory to Asena, the system would not have booted after the first GRUB update; we saw that happen in the virtual machine. So the EFI directory stays `debian`, while the menu says Asena.

## Wallpapers

Ten new wallpapers arrived on 22 October; the default is Asena Night. All are CC BY-SA 4.0. The [Story](/en/story/) page tells the motif behind each.

## Kernel

The default kernel is now XanMod LTS, with Debian’s signed kernel always installed as a fallback. On the test disk, XanMod 6.18 booted with Secure Boot off and Debian’s 6.12 kernel with it on. zram swap was active in both cases. This was a virtual machine test; real hardware and real firmware have not been tried yet.
