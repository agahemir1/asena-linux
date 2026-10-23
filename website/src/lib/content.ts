import { getCollection, type CollectionEntry } from 'astro:content';
import { docPath, newsPath, langFromId, route, type Lang } from '../i18n/routes';

export async function docsFor(lang: Lang): Promise<CollectionEntry<'docs'>[]> {
  const entries = await getCollection('docs', (entry) => langFromId(entry.id) === lang);
  return entries.sort((a, b) => a.data.order - b.data.order);
}

export async function newsFor(lang: Lang): Promise<CollectionEntry<'news'>[]> {
  const entries = await getCollection('news', (entry) => langFromId(entry.id) === lang);
  return entries.sort((a, b) => b.data.date.getTime() - a.data.date.getTime());
}

/** The same document or post in the other language, or that language's section index. */
export async function translationOf(collection: 'docs' | 'news', key: string, lang: Lang): Promise<string> {
  const entries = await getCollection(collection, (entry) => langFromId(entry.id) === lang && entry.data.key === key);
  const match = entries[0];
  if (collection === 'docs') return match ? docPath(lang, match.data.permalink) : route('docs', lang);
  return match ? newsPath(lang, match.data.permalink) : route('news', lang);
}
