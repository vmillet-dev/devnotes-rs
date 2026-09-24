import type { ChecklistItem, NoteKind } from '@core/ipc/bindings';

export type { ChecklistItem, NoteKind };

export interface ChecklistProgress {
  readonly done: number;
  readonly total: number;
  /** 0 to 100. An empty list is 0: nothing to do is not "all done". */
  readonly percent: number;
}

export function checklistProgress(items: readonly ChecklistItem[]): ChecklistProgress {
  const done = items.filter((item) => item.done).length;
  const total = items.length;

  return { done, total, percent: total === 0 ? 0 : Math.round((done / total) * 100) };
}

/** The Markdown is not rendered here: `copyText` carries what Rust produced. */
export function noteCopyText(note: { readonly content: string; readonly copyText: string | null }): string {
  return note.copyText ?? note.content;
}
