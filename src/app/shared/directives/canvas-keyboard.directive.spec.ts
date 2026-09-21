import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { BoardStore } from '@core/state/board.store';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';
import { dialogRung } from '@shared/layout/dialog/dialog.model';
import { NotesHarness, awaitQuery, createNotesHarness } from '@testing/notes-harness';
import { createNote } from '@testing/note.fixture';
import { CANVAS_SHORTCUT_GROUP, CanvasKeyboardDirective } from './canvas-keyboard.directive';

@Component({
  selector: 'app-canvas-keyboard-host',
  hostDirectives: [CanvasKeyboardDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  // ⚠️ `data-note-id` and not only a class: the directive resolves what it measured back to
  // a note through that attribute, which is what keeps the board's order out of the date
  // view's list. A stand-in without it is a card the grid cannot name.
  template: `
    <div class="card-shell" id="card-1" data-note-id="note-1"></div>
    <div class="card-shell" id="card-2" data-note-id="note-2"></div>
    <input id="field" />
  `,
})
class CanvasKeyboardHostComponent {}

const NOTES = [
  createNote({ id: 'note-1', title: 'First', content: 'first body' }),
  createNote({ id: 'note-2', title: 'Second', content: 'second body' }),
];

describe('CanvasKeyboardDirective', () => {
  let fixture: ComponentFixture<CanvasKeyboardHostComponent>;
  let harness: NotesHarness;

  function press(key: string, init: KeyboardEventInit = {}): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
    document.dispatchEvent(event);
    return event;
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    harness = await createNotesHarness([...NOTES]);
    // Standalone: created straight from the module the harness already configured.
    fixture = TestBed.createComponent(CanvasKeyboardHostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
    harness.selection.focusNote('note-1');
  });

  /** ⚠️ Documenting a key and binding it are the same act: the sheet is derived here. */
  it('documents exactly the keys it declares, in reading order', () => {
    expect(CANVAS_SHORTCUT_GROUP.id).toBe('notes.canvas');
    expect(CANVAS_SHORTCUT_GROUP.shortcuts[0].keys).toEqual(['Ctrl', 'K']);
    expect(CANVAS_SHORTCUT_GROUP.shortcuts.every((shortcut) => shortcut.labelKey.length > 0)).toBe(true);
  });

  describe('acting on the focused note', () => {
    it('copies it', async () => {
      const event = press('c');
      await fixture.whenStable();

      expect(harness.clipboard.content).toBe('first body');
      expect(event.defaultPrevented).toBe(true);
    });

    it('ticks it', () => {
      press('x');

      expect(harness.selection.checkedNoteIds()).toEqual(['note-1']);
    });

    it('pins it', async () => {
      press('p');
      await vi.waitFor(() =>
        expect(harness.canvas.visibleNotes().find((note) => note.id === 'note-1')?.pinned).toBe(true),
      );
    });

    it('opens it', () => {
      press('Enter');

      expect(harness.store.selectedNoteId()).toBe('note-1');
    });

    /** ⚠️ The ring may be on a card scrolled out of view: "copied" alone is no answer. */
    it('says which note it copied', async () => {
      press('c');
      await fixture.whenStable();

      expect(TestBed.inject(StatusNotifier).status()).toEqual({
        key: 'notes.copiedNote',
        params: { title: 'First' },
      });
    });

    it('takes the uppercase letter too, for a caps-locked keyboard', async () => {
      press('C');
      await fixture.whenStable();

      expect(harness.clipboard.content).toBe('first body');
    });
  });

  /** What the key copies is what the card's own control copies, and nothing less. */
  describe('what copying a card means', () => {
    async function focusing(note: Parameters<typeof createNote>[0]): Promise<void> {
      TestBed.resetTestingModule();
      harness = await createNotesHarness([createNote({ id: 'note-1', ...note })]);
      fixture = TestBed.createComponent(CanvasKeyboardHostComponent);
      fixture.autoDetectChanges();
      await fixture.whenStable();
      harness.selection.focusNote('note-1');
    }

    /**
     * ⚠️ A todo list has no `content` at all, so the key used to put an empty string on
     * the clipboard while the card's own button handed over the Markdown.
     */
    it('gives a todo list as its Markdown', async () => {
      await focusing({
        kind: 'checklist',
        content: '',
        items: [
          { text: 'Relire', done: true },
          { text: 'Déployer', done: false },
        ],
      });

      press('c');
      await fixture.whenStable();

      expect(harness.clipboard.content).toBe(
        ['- [x] Relire', '- [ ] Déployer'].join(String.fromCharCode(10)),
      );
    });

    /** The form the card's ⚡ opens, which the key went straight past. */
    it('asks for the fields rather than pasting the tokens', async () => {
      await focusing({
        content: 'psql -h {{host}}',
        placeholders: [{ name: 'host', defaultValue: '', value: '' }],
      });

      press('c');
      await fixture.whenStable();

      expect(TestBed.inject(PlaceholderFillStore).target()?.id).toBe('note-1');
      expect(harness.clipboard.content).toBe('');
    });
  });

  /**
   * ⚠️ The key is what puts the gesture in the shortcuts sheet at all: the sheet is derived
   * from this table, so an action with no key is an action nobody discovers. Only the light
   * half gets one — reorganising the zones overwrites sizes chosen by hand, and a keystroke
   * is the one address that cannot ask first.
   */
  describe('aligning the loose cards', () => {
    it('does nothing on the date view, where there is no board', () => {
      const board = TestBed.inject(BoardStore);
      const before = board.arrangements();

      const event = press('a');

      expect(board.arrangements()).toBe(before);
      expect(event.defaultPrevented).toBe(false);
    });

    it('is documented in the sheet the same table builds', () => {
      const listed = CANVAS_SHORTCUT_GROUP.shortcuts.find(
        (shortcut) => shortcut.labelKey === 'shortcuts.canvas.align',
      );

      expect(listed?.keys).toEqual(['A']);
    });
  });

  describe('what it refuses to act on', () => {
    /** ⚠️ A key that acted is the only one whose default is cancelled. */
    it('leaves the browser alone when nothing was focused', () => {
      harness.selection.focusNote(null);

      const event = press('c');

      expect(event.defaultPrevented).toBe(false);
      expect(harness.clipboard.content).toBe('');
    });

    it('stays out of a field being typed in', () => {
      const field: HTMLInputElement = fixture.nativeElement.querySelector('#field');
      field.focus();

      const event = new KeyboardEvent('keydown', { key: 'x', bubbles: true, cancelable: true });
      field.dispatchEvent(event);

      expect(harness.selection.checkedNoteIds()).toEqual([]);
      expect(event.defaultPrevented).toBe(false);
    });

    /** The canvas keyboard is taken for as long as anything is open over it. */
    it('says nothing while a dialog is open', () => {
      TestBed.inject(DialogStack).push({}, dialogRung('editor'));

      press('x');

      expect(harness.selection.checkedNoteIds()).toEqual([]);
    });

    /** ⚠️ A bare key stays bare: Alt is a different gesture entirely. */
    it('ignores a bound key held with Alt', () => {
      press('x', { altKey: true });

      expect(harness.selection.checkedNoteIds()).toEqual([]);
    });

    it('ignores a bare key that wants Ctrl, and the other way round', () => {
      press('z');
      expect(harness.store.lastAction()).toBeNull();

      const withCtrl = press('c', { ctrlKey: true });
      expect(withCtrl.defaultPrevented).toBe(false);
    });

    it('leaves a key it does not bind alone', () => {
      const event = press('q');

      expect(event.defaultPrevented).toBe(false);
    });
  });

  describe('Escape falls through', () => {
    it('clears the selection first', () => {
      harness.selection.toggleChecked('note-1');

      press('Escape');

      expect(harness.selection.checkedNoteIds()).toEqual([]);
    });

    it('clears the filters once nothing is selected', async () => {
      harness.canvas.setSearchQuery('first');
      // ⚠️ Waited for: the search is debounced, and pressing Escape before the query left
      // would undo a filter that had not been applied yet.
      await vi.waitFor(() => expect(harness.repository.lastQuery?.search).toBe('first'));
      const before = harness.repository.queryCount;

      press('Escape');
      await awaitQuery(harness.repository, before);

      expect(harness.repository.lastQuery?.search).toBe('');
    });

    it('does nothing at all when there is neither', () => {
      const event = press('Escape');

      expect(event.defaultPrevented).toBe(false);
    });
  });

  describe('moving the focus', () => {
    it('takes the first card when nothing is focused yet', () => {
      harness.selection.focusNote(null);

      press('ArrowRight');

      expect(harness.selection.focusedIndex()).toBe(0);
    });

    it('walks to the next card', () => {
      press('ArrowRight');

      expect(harness.selection.focusedIndex()).toBe(1);
    });

    it('claims the arrow, so the page does not scroll under the canvas', () => {
      const event = press('ArrowDown');

      expect(event.defaultPrevented).toBe(true);
    });
  });
});
