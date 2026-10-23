import rss from '@astrojs/rss';
import type { APIContext } from 'astro';
import { newsFor } from '../../../lib/content';
import { newsPath, route } from '../../../i18n/routes';

export async function GET(context: APIContext) {
  const posts = await newsFor('en');
  return rss({
    title: 'Asena Linux news',
    description: 'The Asena Linux development log',
    site: new URL(route('news', 'en'), context.site),
    customData: '<language>en</language>',
    items: posts.map((post) => ({
      title: post.data.title,
      description: post.data.description,
      pubDate: post.data.date,
      link: newsPath('en', post.data.permalink),
    })),
  });
}
