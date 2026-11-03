/** Syntax a reader never sees: a link's address, a task's box, the entity a leading tab is written as. */
const UNREAD = /\]\([^)]*\)|\[[ xX]\]|&#9;/g;
const WORD = /[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu;

/** The words of a Note as it reads, not as its Markdown is spelled. */
export function countWords(markdown: string): number {
  return markdown.replace(UNREAD, ' ').match(WORD)?.length ?? 0;
}
