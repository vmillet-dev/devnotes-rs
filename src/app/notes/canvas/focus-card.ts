import { NoteSelectionStore } from '@core/state/note-selection.store';

/**
 * The card of `id`, or the first on screen, given the keyboard: the canvas's state and the real
 * focus together. ⚠️ The card's own effect focuses only on a change of id, and the id may not change.
 */
export function focusCard(selection: NoteSelectionStore, id: string | null): boolean {
  const shells = Array.from(document.querySelectorAll<HTMLElement>('[data-testid="note-card"]'));
  const shell = shells.find((candidate) => candidate.dataset['noteId'] === id) ?? shells[0];
  const button = shell?.querySelector<HTMLElement>('[data-testid="note-card-open"]');
  if (!shell || !button) return false;

  selection.focusNote(shell.dataset['noteId'] ?? null);
  button.focus();
  return document.activeElement === button;
}
