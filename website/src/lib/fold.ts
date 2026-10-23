// Port of asena_core::text::fold (crates/asena-core/src/text.rs): map the four
// Turkish i letters to "i", lowercase, decompose (NFD) and drop combining marks.
// An accent-insensitive search key, not a sorting key.
export function fold(text: string): string {
  return text
    .replace(/[İIı]/g, 'i')
    .toLowerCase()
    .normalize('NFD')
    .replace(/\p{M}/gu, '');
}

/** What a plain case-insensitive match does, for comparison. */
export function naiveKey(text: string): string {
  return text.toLowerCase();
}
