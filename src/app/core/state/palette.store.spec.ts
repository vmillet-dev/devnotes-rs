import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeDesktopNotifications } from '@testing/fake-desktop-notifications';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { SEARCH_DEBOUNCE_MS } from './notes-query.store';
import { PaletteStore } from './palette.store';

interface Harness {
  readonly store: PaletteStore;
  readonly repository: FakeNotesRepository;
  readonly clipboard: FakeClipboard;
  readonly window: FakeAppWindow;
  readonly desktop: FakeDesktopNotifications;
}

function createStore(): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeNotesRepository([
    createNote({ id: 'note-1', title: 'First', content: 'plain body' }),
    createNote({
      id: 'note-2',
      title: 'Templated',
      content: 'psql -h {{host}}',
      placeholders: [{ name: 'host', defaultValue: '', value: '' }],
    }),
  ]);
  const clipboard = new FakeClipboard();
  const appWindow = new FakeAppWindow();
  const desktop = new FakeDesktopNotifications();
  TestBed.configureTestingModule({
    providers: [
      provideAppTesting({
        notesRepository: repository,
        clipboard,
        appWindow,
        desktopNotifications: desktop,
      }),
    ],
  });

  return {
    store: TestBed.inject(PaletteStore),
    repository,
    clipboard,
    window: appWindow,
    desktop,
  };
}

describe('PaletteStore', () => {
  let harness: Harness;

  beforeEach(async () => {
    // Only the debounce timers: a faked `requestAnimationFrame` would hang Angular's
    // zoneless scheduler.
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    harness = createStore();
    // ⚠️ Loaded, not merely configured: the desktop toast is a **translated string**,
    // and `translate` hands back the key itself until the language is in.
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    await harness.store.open();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('searches every space, ignoring the canvas filters', async () => {
    expect(harness.repository.lastQuery?.spaceId).toBeNull();
    expect(harness.repository.lastQuery?.filter).toBe('all');
    expect(harness.repository.lastQuery?.tags).toEqual([]);
  });

  it('opens on the most recent notes rather than an empty list', () => {
    expect(harness.store.results()).toHaveLength(2);
    expect(harness.store.highlightedNote()?.id).toBe('note-1');
  });

  it('defers the query while typing', async () => {
    const before = harness.repository.queryCount;

    harness.store.setQuery('psql');
    expect(harness.repository.queryCount).toBe(before);

    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(harness.repository.lastQuery?.search).toBe('psql');
  });

  it('stops at the ends of the list instead of wrapping', () => {
    harness.store.moveHighlight(-1);
    expect(harness.store.highlighted()).toBe(0);

    harness.store.moveHighlight(1);
    harness.store.moveHighlight(1);
    expect(harness.store.highlighted()).toBe(1);
  });

  it('copies the chosen snippet and gets out of the way', async () => {
    await harness.store.chooseHighlighted();

    expect(harness.clipboard.content).toBe('plain body');
    expect(harness.store.isOpen()).toBe(false);
    expect(harness.window.hidden).toBe(1);
  });

  it('asks for the fields before copying a templated snippet', async () => {
    harness.store.highlight(1);

    await harness.store.chooseHighlighted();

    expect(harness.store.pendingFill()?.id).toBe('note-2');
    expect(harness.clipboard.content).toBe('');
    expect(harness.store.isOpen()).toBe(true);
  });

  it('drops the pending fill when the palette closes', async () => {
    harness.store.highlight(1);
    await harness.store.chooseHighlighted();

    harness.store.close();

    expect(harness.store.pendingFill()).toBeNull();
  });

  it('keeps the window in place when the clipboard refuses', async () => {
    harness.clipboard.failNext = new Error('no clipboard');

    await harness.store.chooseHighlighted();

    expect(harness.store.isOpen()).toBe(true);
    expect(harness.window.hidden).toBe(0);
  });

  /**
   * ⚠️ The window is what every other acknowledgement is drawn on, and this path takes
   * it away. Without a word from the desktop the application simply vanished, which reads
   * as a crash on a copy that worked (#285).
   */
  it('says on the desktop which note it took, once the window has gone', async () => {
    await harness.store.chooseHighlighted();

    expect(harness.desktop.sent).toHaveLength(1);
    expect(harness.desktop.sent[0]?.body).toContain('First');
  });

  it('says nothing when the copy did not happen', async () => {
    harness.clipboard.failNext = new Error('no clipboard');

    await harness.store.chooseHighlighted();

    expect(harness.desktop.sent).toEqual([]);
  });

  /** A desktop that will not show it is not a copy that failed. */
  it('copies and hides all the same when the toast is refused', async () => {
    harness.desktop.permission = 'denied';

    await harness.store.chooseHighlighted();

    expect(harness.clipboard.content).toBe('plain body');
    expect(harness.window.hidden).toBe(1);
    expect(harness.desktop.sent).toEqual([]);
  });
  describe('creating from what was typed', () => {
    it('offers nothing to create on an empty query', () => {
      expect(harness.store.canCreate()).toBe(false);
      expect(harness.store.optionCount()).toBe(2);
    });

    it('appends the create row after the results', async () => {
      harness.store.setQuery('psql');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.store.canCreate()).toBe(true);
      expect(harness.store.optionCount()).toBe(harness.store.results().length + 1);
      expect(harness.store.isCreateHighlighted()).toBe(false);
    });

    it('walks onto the create row and stops there', async () => {
      harness.store.setQuery('psql');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      harness.store.moveHighlight(1);
      harness.store.moveHighlight(1);
      harness.store.moveHighlight(1);

      expect(harness.store.isCreateHighlighted()).toBe(true);
      expect(harness.store.highlighted()).toBe(harness.store.results().length);
    });

    it('highlights the create row first when nothing matches', async () => {
      harness.repository.setView({ sections: [] });
      harness.store.setQuery('rien ne correspond');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.store.isCreateHighlighted()).toBe(true);
    });

    it('hands the typed text over and closes', async () => {
      harness.repository.setView({ sections: [] });
      harness.store.setQuery('  penser à migrer la base  ');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.store.takeNewNoteContent()).toBe('penser à migrer la base');
      expect(harness.store.isOpen()).toBe(false);
    });

    it('hands nothing over while a snippet is highlighted', async () => {
      harness.store.setQuery('psql');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.store.takeNewNoteContent()).toBeNull();
      expect(harness.store.isOpen()).toBe(true);
    });

    it('copies rather than creating when a snippet is chosen', async () => {
      harness.store.setQuery('plain');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      await harness.store.chooseHighlighted();

      expect(harness.clipboard.content).toBe('plain body');
    });
  });
});
