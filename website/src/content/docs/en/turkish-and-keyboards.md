---
title: Turkish and keyboards
description: Turkish search and sorting, keyboard layouts, Turkish F and language settings.
permalink: turkish-and-keyboards
key: turkish-and-keyboards
order: 7
---

## Turkish search

In Turkish, the lowercase of capital İ is i, and the lowercase of capital I is ı. Many programs do not know this: lowercasing “İndirilenler” (Downloads) yields an i followed by a combining dot, and a search for “indir” misses it.

Asena’s search first treats the four letters İ, I, ı and i as one, then lowercases and drops accents such as the cedilla and the dots. So:

- typing “indir” finds **İndirilenler**
- typing “oynatici” finds **Oynatıcı**
- typing “GORUNUM” finds **Görünüm**

The same search runs in the start menu, Settings, Files and Software. Try it yourself in the [demo on the home page](/en/).

## Sorting

Search ignores accents, sorting does not. Alphabetical lists on screen follow the Turkish locale: Ç after C, Ğ after G, Ş after S.

## Keyboard layouts

The default layouts are Turkish Q and English (US); switch between them with <kbd>Alt</kbd> + <kbd>Shift</kbd>. To add Turkish F or another layout, open Settings › Keyboard, where key repeat is too.

Letter shortcuts follow the letter in your first layout, so they land on the right key on Turkish F as well. Details: [Keyboard shortcuts](/en/docs/keyboard-shortcuts/).

## Language

Settings › Region and language changes the interface language. Apps you open afterwards use it right away; the panel and menu switch the next time you log in.

The live system will start in Turkish, with a Turkish Q keyboard and Istanbul time, and the boot menu will have an English entry too.

## Date and time

Settings › Date and time can set the clock automatically from the network. The panel clock’s format is there too.
