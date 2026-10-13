/** Case and accents aside, as the notes' search does in Rust: "hachage" finds "Hachage". */
export function foldForSearch(text: string): string {
  return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}
