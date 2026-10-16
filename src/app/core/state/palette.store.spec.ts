import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeDesktopNotifications } from '@testing/fake-desktop-notifications';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { SEARCH_DEBOUNCE_MS } from './notes-query.store';
import { PaletteStore, paletteMode } from './palette.store';

interface Harness {
  readonly store: PaletteStore;
  readonly repository: FakeNotesRepository;
  readonly http: FakeHttpRepository;
  readonly clipboard: FakeClipboard;
  readonly window: FakeAppWindow;
  readonly desktop: FakeDesktopNotifications;
}

const PLAIN = createNote({ id: 'note-1', title: 'First', content: 'plain body' });
const TEMPLATED = createNote({
  id: 'note-2',
  title: 'Templated',
  content: 'psql -h {{host}}',
  placeholders: [{ name: 'host', defaultValue: '', value: '' }],
});

function createStore(): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeNotesRepository([PLAIN, TEMPLATED]);
  const http = new FakeHttpRepository();
  const clipboard = new FakeClipboard();
  const appWindow = new FakeAppWindow();
  const desktop = new FakeDesktopNotifications();
  TestBed.configureTestingModule({
    providers: [
      provideAppTesting({
        notesRepository: repository,
        httpRepository: http,
        clipboard,
        appWindow,
        desktopNotifications: desktop,
      }),
    ],
  });

  return { store: TestBed.inject(PaletteStore), repository, http, clipboard, window: appWindow, desktop };
}

describe('paletteMode', () => {
  it('reads @ and # where the word being typed starts, > only at the head', () => {
    expect(paletteMode('json @ou')).toEqual({ kind: 'scopes', word: 'ou' });
    expect(paletteMode('#post')).toEqual({ kind: 'tags', word: 'post' });
    expect(paletteMode(' > nouv ')).toEqual({ kind: 'commands', term: 'nouv' });
    expect(paletteMode('a > b')).toEqual({ kind: 'search', term: 'a > b' });
    expect(paletteMode('mail@exemple.fr')).toEqual({ kind: 'search', term: 'mail@exemple.fr' });
  });
});

describe('PaletteStore', () => {
  let harness: Harness;

  beforeEach(async () => {
    // Only the debounce timers: a faked `requestAnimationFrame` would hang Angular's
    // zoneless scheduler.
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    harness = createStore();
    // Loaded, not merely configured: the desktop toast is a **translated string**,
    // and `translate` hands back the key itself until the language is in.
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    await harness.store.open(true);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('searches every space, ignoring the canvas filters', () => {
    expect(harness.repository.lastQuery?.spaceId).toBeNull();
    expect(harness.repository.lastQuery?.filter).toBe('all');
    expect(harness.repository.lastQuery?.tags).toEqual([]);
  });

  it('opens on the recent notes and requests rather than an empty list', () => {
    expect(harness.store.recent()).toBe(true);
    expect(harness.store.notes().map((note) => note.id)).toEqual(['note-1', 'note-2']);
    expect(harness.http.calls).toContainEqual({ command: 'search', args: [''] });
  });

  it('defers the search while typing', async () => {
    const before = harness.repository.queryCount;

    harness.store.setText('psql');
    expect(harness.repository.queryCount).toBe(before);

    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(harness.repository.lastQuery?.search).toBe('psql');
    expect(harness.http.calls.at(-1)).toEqual({ command: 'search', args: ['psql'] });
  });

  it('asks nothing of Rust while @, # or > lists something to choose', async () => {
    const before = harness.repository.queryCount;

    harness.store.setText('@ou');
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

    expect(harness.repository.queryCount).toBe(before);
    expect(harness.store.notes()).toEqual([]);
  });

  describe('chips', () => {
    it('takes the word that asked for one out of the field', () => {
      harness.store.setText('json @ou');

      harness.store.addChip({ kind: 'scope', scope: 'tools' });

      expect(harness.store.text()).toBe('json');
      expect(harness.store.chips()).toEqual([{ kind: 'scope', scope: 'tools' }]);
      expect([...harness.store.searched()]).toEqual(['tools']);
    });

    it('adds kinds up, and a space or a tag narrows to notes', () => {
      harness.store.addChip({ kind: 'scope', scope: 'requests' });
      harness.store.addChip({ kind: 'tag', tag: 'postgres' });

      expect([...harness.store.searched()].sort()).toEqual(['notes', 'requests']);
    });

    it('asks Rust for the space and the tags chosen, and no requests without their chip', async () => {
      harness.store.addChip({ kind: 'space', id: 'space-9', name: 'Recette' });
      harness.store.addChip({ kind: 'tag', tag: 'postgres' });
      const asked = harness.http.calls.length;
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.repository.lastQuery?.spaceId).toBe('space-9');
      expect(harness.repository.lastQuery?.tags).toEqual(['postgres']);
      expect(harness.http.calls).toHaveLength(asked);
    });

    it('lets a second space take the place of the first, and the same chip in only once', () => {
      harness.store.addChip({ kind: 'space', id: 'a', name: 'Travail' });
      harness.store.addChip({ kind: 'space', id: 'b', name: 'Perso' });
      harness.store.addChip({ kind: 'scope', scope: 'notes' });
      harness.store.addChip({ kind: 'scope', scope: 'notes' });

      expect(harness.store.chips()).toEqual([
        { kind: 'space', id: 'b', name: 'Perso' },
        { kind: 'scope', scope: 'notes' },
      ]);
    });

    it('widens again once a chip is removed', () => {
      harness.store.addChip({ kind: 'scope', scope: 'tools' });

      harness.store.removeChip(0);

      expect(harness.store.chips()).toEqual([]);
      expect(harness.store.searched().size).toBe(4);
    });
  });

  describe('copying a note', () => {
    it('copies it and puts the window away, over another application', async () => {
      await harness.store.copy(PLAIN);

      expect(harness.clipboard.content).toBe('plain body');
      expect(harness.store.isOpen()).toBe(false);
      expect(harness.window.hidden).toBe(1);
    });

    it('only closes with DevNotes already in front, and says so in the window', async () => {
      await harness.store.open(false);

      await harness.store.copy(PLAIN);

      expect(harness.clipboard.content).toBe('plain body');
      expect(harness.window.hidden).toBe(0);
      expect(harness.desktop.sent).toEqual([]);
      expect(TestBed.inject(StatusNotifier).status()).toEqual({
        key: 'palette.copiedNote',
        params: { title: 'First' },
      });
    });

    it('copies the whole body of a result the list cut', async () => {
      await harness.store.copy(
        createNote({ id: 'note-1', title: 'First', content: 'plain', truncated: true }),
      );

      expect(harness.clipboard.content).toBe('plain body');
    });

    it('asks for the fields before copying a templated snippet', async () => {
      await harness.store.copy(TEMPLATED);

      expect(harness.store.pendingFill()?.id).toBe('note-2');
      expect(harness.clipboard.content).toBe('');
      expect(harness.store.isOpen()).toBe(true);
    });

    it('copies a templated snippet as it is from its whole body, not from the list', async () => {
      await harness.store.copyRaw({ ...TEMPLATED, content: 'psql', truncated: true });

      expect(harness.clipboard.content).toBe('psql -h {{host}}');
      expect(harness.store.isOpen()).toBe(false);
    });

    it('copies nothing and stays open when the whole body cannot be read', async () => {
      harness.repository.failNext = new Error('bridge down');

      await harness.store.copyRaw({ ...TEMPLATED, content: 'psql', truncated: true });

      expect(harness.clipboard.content).toBe('');
      expect(harness.store.isOpen()).toBe(true);
    });

    it('drops the pending fill when the palette closes', async () => {
      await harness.store.copy(TEMPLATED);

      harness.store.close();

      expect(harness.store.pendingFill()).toBeNull();
    });

    it('keeps the window in place when the clipboard refuses', async () => {
      harness.clipboard.failNext = new Error('no clipboard');

      await harness.store.copy(PLAIN);

      expect(harness.store.isOpen()).toBe(true);
      expect(harness.window.hidden).toBe(0);
      expect(harness.desktop.sent).toEqual([]);
    });

    /** The window is gone, so the desktop is the only place left to say the copy happened. */
    it('says on the desktop which note it took, once the window has gone', async () => {
      await harness.store.copy(PLAIN);

      expect(harness.desktop.sent).toHaveLength(1);
      expect(harness.desktop.sent[0]?.body).toContain('First');
    });

    /** A desktop that will not show it is not a copy that failed. */
    it('copies and hides all the same when the toast is refused', async () => {
      harness.desktop.permission = 'denied';

      await harness.store.copy(PLAIN);

      expect(harness.clipboard.content).toBe('plain body');
      expect(harness.window.hidden).toBe(1);
      expect(harness.desktop.sent).toEqual([]);
    });
  });
});
