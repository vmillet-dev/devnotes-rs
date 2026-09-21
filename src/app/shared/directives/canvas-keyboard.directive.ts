import { Directive, ElementRef, inject } from '@angular/core';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutGroup } from '@core/services/shortcuts/shortcut.model';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';
import { FoldersStore } from '@core/state/folders.store';
import { Note } from '@core/model/note.model';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NotesStore } from '@core/state/notes.store';
import { CardBox, FocusDirection, nextFocusIndex } from '@core/utils/grid-navigation.util';

/** A card as it is on screen: where it is, and which note it is. */
interface MeasuredCard extends CardBox {
  readonly id: string;
}

interface CanvasContext {
  readonly focused: Note | null;
  readonly notes: NotesStore;
  readonly canvas: NotesQueryStore;
  readonly folders: FoldersStore;
  readonly selection: NoteSelectionStore;
  readonly settings: SettingsStore;
  /** The whole rule — the fields form, a todo list's Markdown — lives in the store. */
  readonly copy: (note: Note) => void;
  readonly move: (direction: FocusDirection) => void;
}

/** Documenting a key and binding it are the same act: the sheet is derived from this. */
interface CanvasKey {
  readonly keys: readonly string[];
  readonly labelKey: string;
  /** Absent means the key is only documented here and handled elsewhere. */
  readonly on?: readonly string[];
  /** Ctrl (or ⌘) must be held. Without it, no modifier may be. */
  readonly ctrl?: boolean;
  /** Answers whether it acted: only then is the browser's own behaviour cancelled. */
  readonly run?: (context: CanvasContext, key: string) => boolean;
}

/** ⚠️ Spelled once: the table below binds it and the written guide names it. */
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
const CANVAS_KEYS: readonly CanvasKey[] = [
  { keys: ['Ctrl', 'K'], labelKey: 'shortcuts.canvas.search' },
  {
    keys: ['Ctrl', 'B'],
    labelKey: 'shortcuts.canvas.library',
    on: ['b', 'B'],
    ctrl: true,
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
    keys: ['Enter'],
    labelKey: 'shortcuts.canvas.open',
    on: ['Enter'],
    run: ({ focused, notes }) => given(focused, (note) => notes.openNote(note.id)),
  },
  {
    keys: ['C'],
    labelKey: 'shortcuts.canvas.copy',
    on: ['c', 'C'],
    run: ({ focused, copy }) => given(focused, copy),
  },
  {
    keys: ['P'],
    labelKey: 'shortcuts.canvas.pin',
    on: ['p', 'P'],
    run: ({ focused, notes }) => given(focused, (note) => void notes.togglePinned(note.id)),
  },
  {
    keys: [CHECK_KEY],
    labelKey: 'shortcuts.canvas.check',
    on: ['x', 'X'],
    run: ({ focused, selection }) => given(focused, (note) => selection.toggleChecked(note.id)),
  },
  { keys: ['Ctrl'], labelKey: 'shortcuts.canvas.checkWithClick' },
  { keys: ['Shift'], labelKey: 'shortcuts.canvas.extendWithClick' },
  {
    keys: ['Delete'],
    labelKey: 'shortcuts.canvas.trash',
    on: ['Delete', 'Backspace'],
    // ⚠️ Twice, the way the card's own menu asks for two clicks. One press used to trash
    // whichever card the ring was on, and the ring can be on a card that is scrolled away.
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
    keys: ['Ctrl', 'Z'],
    labelKey: 'shortcuts.canvas.undo',
    on: ['z', 'Z'],
    ctrl: true,
    // Even after the banner is gone: it is the gesture one makes without looking.
    run: ({ notes }) => when(notes.lastAction() !== null, () => void notes.undoLastAction()),
  },
  {
    keys: ['Escape'],
    // ⚠️ Not `clearSelection`, which is what it was called when the selection was the
    // only thing it undid. The card's own banner names this key for the same reason.
    labelKey: 'shortcuts.canvas.stepBack',
    on: ['Escape'],
    // Falls through: the armed note first, then the selection, then the search and the
    // facets, and only then out of the folder — leaving it is the biggest, so it is last.
    run: ({ selection, canvas, folders }) =>
      when(selection.armedForDeletion() !== null, () => selection.disarm()) ||
      when(selection.hasSelection(), () => selection.clearSelection()) ||
      when(canvas.hasUserFilters(), () => canvas.clearFilters()) ||
      when(folders.activeFolderId() !== null, () => folders.selectFolder(null)),
  },
];

/** The sheet's canvas group, built from the table that binds the same keys. */
export const CANVAS_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'notes.canvas',
  labelKey: 'shortcuts.groups.canvas',
  shortcuts: CANVAS_KEYS.map(({ keys, labelKey }) => ({ keys, labelKey })),
};

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;

  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
}

/**
 * A host directive of the notes page, so its element is the canvas itself — which is
 * how it measures the card grid without being handed a list of sections.
 */
@Directive({
  selector: '[appCanvasKeyboard]',
  host: {
    '(document:keydown)': 'onKeydown($event)',
  },
})
export class CanvasKeyboardDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly selection = inject(NoteSelectionStore);
  private readonly notes = inject(NotesStore);
  private readonly canvas = inject(NotesQueryStore);
  private readonly folders = inject(FoldersStore);
  private readonly fill = inject(PlaceholderFillStore);
  private readonly dialogs = inject(DialogStack);
  private readonly settings = inject(SettingsStore);

  protected onKeydown(event: KeyboardEvent): void {
    // ⚠️ `defaultPrevented` too: this listens on the document, so a control that has
    // already handled the key — the rail's resize edge — would see the canvas act on it
    // as well, and the arrows would move the card focus while the rail is being widened.
    if (this.dialogs.hasOpenDialog() || isTypingTarget(event.target) || event.defaultPrevented) return;

    const withCtrl = event.ctrlKey || event.metaKey;
    const entry = CANVAS_KEYS.find(
      (candidate) =>
        candidate.on?.includes(event.key) === true &&
        (candidate.ctrl ?? false) === withCtrl &&
        // A bare key stays bare: Alt is a different gesture entirely.
        (candidate.ctrl === true || !event.altKey),
    );
    if (!entry?.run) return;

    if (entry.run(this.context(), event.key)) {
      event.preventDefault();
    }
  }

  private context(): CanvasContext {
    return {
      focused: this.selection.focusedNote(),
      notes: this.notes,
      canvas: this.canvas,
      folders: this.folders,
      selection: this.selection,
      settings: this.settings,
      copy: (note) => void this.fill.copyNote(note),
      move: (direction) => this.moveFocus(direction),
    };
  }

  /** Measured: the column count depends on the window width. */
  private moveFocus(direction: FocusDirection): void {
    const cards = this.cardBoxes();
    const first = cards[0];
    if (!first) return;

    // ⚠️ Found by id among what was just measured, never by a position in `visibleNotes`.
    // That list is the date view's order — pinned first, then `updated_at` — and the board
    // lays its cards out zone by zone, so an index resolved there landed the focus on an
    // unrelated card: the first ArrowRight on the board jumped two cards sideways.
    const focused = this.selection.focusedNoteId();
    const current = cards.findIndex((card) => card.id === focused);
    if (current < 0) {
      this.selection.focusNote(first.id);
      return;
    }

    const next = cards[nextFocusIndex(cards, current, direction)];
    if (next) {
      this.selection.focusNote(next.id);
    }
  }

  /** In DOM order, which is the order they are on screen — the only one a grid move means. */
  private cardBoxes(): readonly MeasuredCard[] {
    return Array.from(this.host.nativeElement.querySelectorAll<HTMLElement>('.card-shell'))
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return { id: element.dataset['noteId'] ?? '', top: rect.top, left: rect.left };
      })
      .filter((card) => card.id !== '');
  }
}
