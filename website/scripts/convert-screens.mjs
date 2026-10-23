// Turns the PNGs from take-shots.sh into the site's WebP sources.
//   node scripts/convert-screens.mjs [shots-dir]
// Desktop surfaces stay full frame so they can cross-fade over each other.
// Application and accent shots are cropped to the window: the shot is compared
// with the empty desktop of the same language and style, and everything that
// differs (window and shadow) is kept with a margin of wallpaper around it.
import sharp from 'sharp';
import { mkdir, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const source = path.resolve(process.argv[2] ?? path.join(here, '../../target/site-capture'));
const target = path.join(here, '../src/assets/screens');

const FULL_WIDTH = 2880;
const SAMPLE_WIDTH = 960;
const MARGIN = 72; // physical pixels of wallpaper kept around a window

async function sample(file) {
  const { data, info } = await sharp(file).resize(SAMPLE_WIDTH).removeAlpha().raw().toBuffer({ resolveWithObject: true });
  return { data, width: info.width, height: info.height };
}

/** Bounding box of what differs from the empty desktop, ignoring the panel and the desktop icon. */
async function windowBox(file, base, full) {
  const [a, b] = await Promise.all([sample(file), sample(base)]);
  const scale = full.width / a.width;
  const panel = Math.ceil(110 / scale);
  const icon = Math.ceil(220 / scale);
  let left = a.width, top = a.height, right = -1, bottom = -1;
  for (let y = 0; y < a.height - panel; y++) {
    for (let x = 0; x < a.width; x++) {
      if (x < icon && y < icon) continue;
      const i = (y * a.width + x) * 3;
      const delta = Math.abs(a.data[i] - b.data[i]) + Math.abs(a.data[i + 1] - b.data[i + 1]) + Math.abs(a.data[i + 2] - b.data[i + 2]);
      if (delta > 36) {
        if (x < left) left = x;
        if (x > right) right = x;
        if (y < top) top = y;
        if (y > bottom) bottom = y;
      }
    }
  }
  if (right < 0) throw new Error(`No window found in ${path.basename(file)}`);
  const x0 = Math.max(0, Math.floor(left * scale) - MARGIN);
  const y0 = Math.max(0, Math.floor(top * scale) - MARGIN);
  const x1 = Math.min(full.width, Math.ceil((right + 1) * scale) + MARGIN);
  const y1 = Math.min(full.height, Math.ceil((bottom + 1) * scale) + MARGIN);
  return { left: x0, top: y0, width: x1 - x0, height: y1 - y0 };
}

await mkdir(target, { recursive: true });
const files = (await readdir(source)).filter((name) => /^(tr|en)-[a-z0-9-]+\.png$/.test(name)).sort();
for (const name of files) {
  const file = path.join(source, name);
  const [lang] = name.split('-');
  const out = path.join(target, name.replace(/\.png$/, '.webp'));
  const meta = await sharp(file).metadata();
  let image = sharp(file);
  if (/-(app|accent)-/.test(name)) {
    const style = name.includes('-light') ? 'light' : 'dark';
    const box = await windowBox(file, path.join(source, `${lang}-desktop-${style}.png`), meta);
    image = image.extract(box);
    await image.webp({ quality: 88, effort: 6 }).toFile(out);
    console.log(`${name} -> crop ${box.width}x${box.height}`);
  } else {
    await image.resize(FULL_WIDTH).webp({ quality: 84, effort: 6 }).toFile(out);
    console.log(`${name} -> ${FULL_WIDTH}w`);
  }
}
