import { Directive, ElementRef, inject } from '@angular/core';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { canvasKeystrokeFromEvent } from '@core/services/shortcuts/shortcut.model';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';
import { FoldersStore } from '@core/state/folders.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NoteBatchStore } from '@core/state/note-batch.store';
import { NotesStore } from '@core/state/notes.store';
import { UndoStore } from '@core/state/undo.store';
import { BoardStore } from '@core/state/board.store';
import { CardBox, FocusDirection, nextFocusIndex } from '@core/utils/grid-navigation.util';
import { CANVAS_KEYS, CanvasContext, CanvasKey, rebindable } from './canvas-keys';

/** A card as it is on screen: where it is, and which note it is. */
interface MeasuredCard extends CardBox {
  readonly id: string;
}

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
  private readonly batch = inject(NoteBatchStore);
  private readonly undo = inject(UndoStore);
  private readonly board = inject(BoardStore);
  private readonly canvas = inject(NotesQueryStore);
  private readonly folders = inject(FoldersStore);
  private readonly fill = inject(PlaceholderFillStore);
  private readonly dialogs = inject(DialogStack);
  private readonly settings = inject(SettingsStore);
  private readonly bindings = inject(ShortcutBindingsStore);

  protected onKeydown(event: KeyboardEvent): void {
    // ⚠️ `defaultPrevented` too: this listens on the document, so a control that has
    // already handled the key — the rail's resize edge — would see the canvas act on it
    // as well, and the arrows would move the card focus while the rail is being widened.
    if (this.dialogs.hasOpenDialog() || isTypingTarget(event.target) || event.defaultPrevented) return;

    const chord = canvasKeystrokeFromEvent(event);
    const entry = CANVAS_KEYS.find((candidate) => this.answersTo(candidate, event, chord));
    if (!entry?.run) return;

    if (entry.run(this.context(), event.key)) {
      event.preventDefault();
    }
  }

  /** A rebindable entry answers to whatever it is bound to; a fixed one to its own keys. */
  private answersTo(candidate: CanvasKey, event: KeyboardEvent, chord: string | null): boolean {
    const action = rebindable(candidate);
    if (action) {
      return (
        chord !== null &&
        (this.bindings.binding(action) === chord || candidate.aliases?.includes(chord) === true)
      );
    }

    // A bare key stays bare: Ctrl, ⌘ and Alt are different gestures entirely.
    return candidate.on?.includes(event.key) === true && !event.ctrlKey && !event.metaKey && !event.altKey;
  }

  private context(): CanvasContext {
    return {
      focused: this.selection.focusedNote(),
      notes: this.notes,
      batch: this.batch,
      undo: this.undo,
      board: this.board,
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

    // Found by id among what was just measured, never by a position in `visibleNotes`.
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
