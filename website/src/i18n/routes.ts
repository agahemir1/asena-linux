export const languages = ['tr', 'en'] as const;
export type Lang = (typeof languages)[number];

export const localeNames: Record<Lang, string> = { tr: 'Türkçe', en: 'English' };
export const htmlLang: Record<Lang, string> = { tr: 'tr', en: 'en' };
export const ogLocale: Record<Lang, string> = { tr: 'tr_TR', en: 'en_US' };

/** Every page that exists in both languages, keyed by a language-neutral name. */
export const routes = {
  home: { tr: '/', en: '/en/' },
  desktop: { tr: '/masaustu/', en: '/en/desktop/' },
  apps: { tr: '/uygulamalar/', en: '/en/apps/' },
  story: { tr: '/hikaye/', en: '/en/story/' },
  why: { tr: '/neden-asena/', en: '/en/why-asena/' },
  download: { tr: '/indir/', en: '/en/download/' },
  docs: { tr: '/belgeler/', en: '/en/docs/' },
  news: { tr: '/haberler/', en: '/en/news/' },
} as const satisfies Record<string, Record<Lang, string>>;

export type RouteKey = keyof typeof routes;

export function route(key: RouteKey, lang: Lang): string {
  return routes[key][lang];
}

export function docPath(lang: Lang, slug: string): string {
  return `${routes.docs[lang]}${slug}/`;
}

export function newsPath(lang: Lang, slug: string): string {
  return `${routes.news[lang]}${slug}/`;
}

export function langFromId(id: string): Lang {
  const prefix = id.split('/')[0];
  return prefix === 'en' ? 'en' : 'tr';
}
