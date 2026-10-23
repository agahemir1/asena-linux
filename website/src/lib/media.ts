import type { ImageMetadata } from 'astro';
import type { Lang } from '../i18n/routes';

// Screenshots from website/scripts/take-shots.sh, named <lang>-<shot>.webp.
// An English page falls back to the Turkish shot when no English one exists.
const screens = import.meta.glob<{ default: ImageMetadata }>('../assets/screens/*.webp', { eager: true });

export function screen(lang: Lang, name: string): ImageMetadata {
  const hit = screens[`../assets/screens/${lang}-${name}.webp`] ?? screens[`../assets/screens/tr-${name}.webp`];
  if (!hit) throw new Error(`Missing screenshot ${name}`);
  return hit.default;
}

// The shipped wallpapers (data/wallpapers), CC BY-SA 4.0.
const wallpapers = import.meta.glob<{ default: ImageMetadata }>('../../../data/wallpapers/*.jpg', { eager: true });

export function wallpaper(name: string): ImageMetadata {
  const hit = wallpapers[`../../../data/wallpapers/${name}.jpg`];
  if (!hit) throw new Error(`Missing wallpaper ${name}`);
  return hit.default;
}

const evidence = import.meta.glob<{ default: ImageMetadata }>('../../../docs/reports/evidence/boot-test/*.png', {
  eager: true,
});

export function bootShot(name: string): ImageMetadata {
  const hit = evidence[`../../../docs/reports/evidence/boot-test/${name}.png`];
  if (!hit) throw new Error(`Missing boot evidence ${name}`);
  return hit.default;
}

const folders = import.meta.glob<{ default: ImageMetadata }>('../../../data/icons/Asena*/scalable/places/folder*.svg', {
  eager: true,
});

/** Folder icons as drawn for an accent; teal is the base Asena theme. */
export function folderIcon(accent: string, name: string): ImageMetadata {
  const theme = accent === 'teal' ? 'Asena' : `Asena-${accent}`;
  const hit = folders[`../../../data/icons/${theme}/scalable/places/${name}.svg`];
  if (!hit) throw new Error(`Missing folder icon ${theme}/${name}`);
  return hit.default;
}

export const accents = ['blue', 'teal', 'green', 'yellow', 'orange', 'red', 'pink', 'purple', 'slate'] as const;
export type Accent = (typeof accents)[number];
