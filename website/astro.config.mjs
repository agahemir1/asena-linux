// @ts-check
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';
import { fileURLToPath } from 'node:url';

// Logo, wallpapers and icons are read from the repository's data/ directory,
// never copied, so the site always shows what the distribution ships.
const data = fileURLToPath(new URL('../data', import.meta.url));
const evidence = fileURLToPath(new URL('../docs/reports/evidence', import.meta.url));

export default defineConfig({
  site: 'https://asenalinux.netlify.app',
  trailingSlash: 'always',
  i18n: {
    locales: ['tr', 'en'],
    defaultLocale: 'tr',
    routing: { prefixDefaultLocale: false },
  },
  integrations: [sitemap()],
  devToolbar: { enabled: false },
  image: { responsiveStyles: false },
  vite: {
    resolve: { alias: { '@data': data, '@evidence': evidence } },
    server: { fs: { allow: ['..'] } },
  },
});
