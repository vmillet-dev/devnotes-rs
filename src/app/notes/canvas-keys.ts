import type { SettingsStore } from '@core/services/settings/settings.store';
import { Rebindable, ShortcutGroup, acceleratorKeys } from '@core/services/shortcuts/shortcut.model';
import type { Note } from '@core/model/note.model';
import type { BoardStore } from '@core/state/board.store';
import type { FoldersStore } from '@core/state/folders.store';
import type { NoteBatchStore } from '@core/state/note-batch.store';
import type { NoteSelectionStore } from '@core/state/note-selection.store';
import type { NotesQueryStore } from '@core/state/notes-query.store';
import type { NotesStore } from '@core/state/notes.store';
import type { UndoStore } from '@core/state/undo.store';
import type { FocusDirection } from '@core/utils/grid-navigation.util';

// Data, apart from `CanvasKeyboardDirective` that binds it: the sheet, the preferences and the
// guide read this table, and importing the directive dragged its stores into their chunks.

export interface CanvasContext {
  readonly focused: Note | null;
  readonly notes: NotesStore;
  readonly batch: NoteBatchStore;
  readonly undo: UndoStore;
  readonly board: BoardStore;
  readonly canvas: NotesQueryStore;
  readonly folders: FoldersStore;
  readonly selection: NoteSelectionStore;
  readonly settings: SettingsStore;
  /** The whole rule — the fields form, a todo list's Markdown — lives in the store. */
  readonly copy: (note: Note) => void;
  readonly move: (direction: FocusDirection) => void;
}

/**
 * Documenting a key, binding it and letting it be moved are the same act: the sheet and
 * the preferences panel are both derived from this.
 *
 * Two shapes. A **rebindable** entry declares `id` and `accelerator`, and its caps are
 * derived from the second — so the key is spelled once. A **fixed** one declares `keys`
 * for the sheet and `on` for the match: the arrows are the grid's own navigation and
 * Escape is the way out of everything on screen, so neither may be moved.
 */
export interface CanvasKey {
  readonly labelKey: string;
  /** Rebindable: what the binding is stored under, and the accelerator it ships with. */
  readonly id?: string;
  readonly accelerator?: string;
  /**
   * Fires the action whatever it is bound to. Backspace has trashed a note since
   * before the key could be moved, and making it movable is no reason to take that away.
   */
  readonly aliases?: readonly string[];
  /** Fixed: the caps the sheet draws, and the `event.key`s that fire it. */
  readonly keys?: readonly string[];
  /** Absent on both shapes means the key is documented here and handled elsewhere. */
  readonly on?: readonly string[];
  /** Answers whether it acted: only then is the browser's own behaviour cancelled. */
  readonly run?: (context: CanvasContext, key: string) => boolean;
}

/** Spelled once: the table below binds it and the written guide names it. */
export const CHECK_KEY = 'X';

const DIRECTIONS: Record<string, FocusDirection> = {
  ArrowLeft: 'prev',
  ArrowRight: 'next',
  ArrowUp: 'up',
  ArrowDown: 'down',
};

/** Both answer whether they did anything, which is what decides the `preventDefault`. */
function given<T>(value: T | null | undefined, action: (value: T) => void): boolean {
  if (value === null || value === undefined) return false;

  action(value);
  return true;
}

function when(condition: boolean, action: () => void): boolean {
  if (!condition) return false;

  action();
  return true;
}

/** In reading order, which is also the order the sheet lists them in. */
export const CANVAS_KEYS: readonly CanvasKey[] = [
  { keys: ['Ctrl', 'K'], labelKey: 'shortcuts.canvas.search' },
  {
    id: 'canvas.library',
    accelerator: 'Ctrl+B',
    labelKey: 'shortcuts.canvas.library',
    run: ({ settings }) => {
      settings.showLibraryRail.write(!settings.showLibraryRail());
      return true;
    },
  },
  {
    keys: ['↑ ↓ ← →'],
    labelKey: 'shortcuts.canvas.move',
    on: Object.keys(DIRECTIONS),
    run: ({ move }, key) => given(DIRECTIONS[key], move),
  },
  {
    id: 'canvas.open',
    accelerator: 'Enter',
    labelKey: 'shortcuts.canvas.open',
    run: ({ focused, notes }) => given(focused, (note) => void notes.openNote(note.id)),
  },
  {
    id: 'canvas.copy',
    accelerator: 'C',
    labelKey: 'shortcuts.canvas.copy',
    run: ({ focused, copy }) => given(focused, copy),
  },
  {
    id: 'canvas.pin',
    accelerator: 'P',
    labelKey: 'shortcuts.canvas.pin',
    run: ({ focused, notes }) => given(focused, (note) => void notes.togglePinned(note.id)),
  },
  {
    id: 'canvas.check',
    accelerator: CHECK_KEY,
    labelKey: 'shortcuts.canvas.check',
    run: ({ focused, selection }) => given(focused, (note) => selection.toggleChecked(note.id)),
  },
  {
    // The light half only. Reorganising the zones overwrites sizes chosen by hand, and
    // a key is the one address that cannot ask first — it stays a notch further away, in
    // the control's own menu.
    id: 'canvas.align',
    accelerator: 'A',
    labelKey: 'shortcuts.canvas.align',
    run: ({ board, batch }) => when(board.isShowing(), () => void batch.arrangeBoard('looseCards')),
  },
  { keys: ['Ctrl'], labelKey: 'shortcuts.canvas.checkWithClick' },
  { keys: ['Shift'], labelKey: 'shortcuts.canvas.extendWithClick' },
  {
    id: 'canvas.trash',
    accelerator: 'Delete',
    aliases: ['Backspace'],
    labelKey: 'shortcuts.canvas.trash',
    // Twice, the way the card's own menu asks for two clicks: the ring can be on a card
    // scrolled out of view.
    run: ({ focused, notes, selection }) =>
      given(focused, (note) => {
        if (selection.armedForDeletion() !== note.id) {
          selection.armForDeletion(note.id);
          return;
        }

        selection.disarm();
        void notes.deleteNote(note.id);
      }),
  },
  {
    id: 'canvas.undo',
    accelerator: 'Ctrl+Z',
    labelKey: 'shortcuts.canvas.undo',
    // Even after the banner is gone: it is the gesture one makes without looking.
    run: ({ undo }) => when(undo.last() !== null, () => void undo.revert()),
  },
  {
    keys: ['Escape'],
    // Not `clearSelection`, which is what it was called when the selection was the
    // only thing it undid. The card's own banner names this key for the same reason.
    labelKey: 'shortcuts.canvas.stepBack',
    on: ['Escape'],
    // Falls through: the armed note first, then whatever the undo bar is offering, then
    // the selection, then the search and the facets, and only then out of the folder —
    // leaving it is the biggest, so it is last.
    //
    // The second rung is the only one that writes, and it reads `undo.banner()`, not
    // `undo.last()`: Escape answers only while the offer is on screen, and outside it must not
    // quietly rewrite the corpus.
    run: ({ selection, canvas, folders, undo }) =>
      when(selection.armedForDeletion() !== null, () => selection.disarm()) ||
      when(undo.banner() !== null, () => void undo.revert()) ||
      when(selection.hasSelection(), () => selection.clearSelection()) ||
      when(canvas.hasUserFilters(), () => canvas.clearFilters()) ||
      when(folders.activeFolderId() !== null, () => folders.selectFolder(null)),
  },
];

/** The same entry seen as something that can be moved, when it can. */
export function rebindable({ id, accelerator, labelKey }: CanvasKey): Rebindable | undefined {
  return id !== undefined && accelerator !== undefined ? { id, labelKey, fallback: accelerator } : undefined;
}

/** What the preferences panel edits. Derived, so this table stays the only declaration. */
export const CANVAS_ACTIONS: readonly Rebindable[] = CANVAS_KEYS.flatMap((key) => rebindable(key) ?? []);

/** The sheet's canvas group, built from the table that binds the same keys. */
export const CANVAS_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'notes.canvas',
  labelKey: 'shortcuts.groups.canvas',
  shortcuts: CANVAS_KEYS.map((key) => ({
    labelKey: key.labelKey,
    keys: key.accelerator ? acceleratorKeys(key.accelerator) : (key.keys ?? []),
    action: rebindable(key),
  })),
};
