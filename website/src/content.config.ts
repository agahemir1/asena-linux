import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';

// Files live under <lang>/; `key` pairs a page with its translation. The URL part is
// `permalink`, not `slug`, which the glob loader would take as the entry id.
const docs = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/docs' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    permalink: z.string(),
    key: z.string(),
    order: z.number(),
  }),
});

const news = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/news' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    permalink: z.string(),
    key: z.string(),
    date: z.coerce.date(),
  }),
});

export const collections = { docs, news };
