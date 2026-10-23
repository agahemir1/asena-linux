import rss from '@astrojs/rss';
import type { APIContext } from 'astro';
import { newsFor } from '../../lib/content';
import { newsPath, route } from '../../i18n/routes';

export async function GET(context: APIContext) {
  const posts = await newsFor('tr');
  return rss({
    title: 'Asena Linux haberleri',
    description: 'Asena Linux geliştirme günlüğü',
    site: new URL(route('news', 'tr'), context.site),
    customData: '<language>tr</language>',
    items: posts.map((post) => ({
      title: post.data.title,
      description: post.data.description,
      pubDate: post.data.date,
      link: newsPath('tr', post.data.permalink),
    })),
  });
}
