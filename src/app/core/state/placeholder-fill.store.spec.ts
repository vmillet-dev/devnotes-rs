import { TestBed } from '@angular/core/testing';
import { type MockInstance, beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { NotesHarness, createNotesHarness } from '@testing/notes-harness';
import { NotesRevision } from './notes-revision';
import { PaletteStore } from './palette.store';
import { PlaceholderFillStore } from './placeholder-fill.store';

const SNIPPET = createNote({
  id: 'note-1',
  title: 'Connexion',
  content: 'psql -h {{host}} -U {{user}}',
  placeholders: [
    { name: 'host', defaultValue: '', value: '' },
    { name: 'user', defaultValue: '', value: '' },
  ],
});

describe('PlaceholderFillStore', () => {
  let harness: NotesHarness;
  let fill: PlaceholderFillStore;
  let status: StatusNotifier;
  let kept: MockInstance<FakeNotesRepository['setPlaceholderValues']>;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    harness = await createNotesHarness([SNIPPET]);
    fill = TestBed.inject(PlaceholderFillStore);
    status = TestBed.inject(StatusNotifier);
    kept = vi.spyOn(harness.repository, 'setPlaceholderValues');
  });

  describe('the form a card opens', () => {
    it('takes its target from what the canvas is showing', () => {
      fill.openFor('note-1');

      expect(fill.target()?.id).toBe('note-1');
      expect(fill.placeholders().map((field) => field.name)).toEqual(['host', 'user']);
    });

    it('opens on nothing for a note the canvas does not hold', () => {
      fill.openFor('note-gone');

      expect(fill.target()).toBeNull();
      expect(fill.placeholders()).toEqual([]);
    });

    it('closes without copying when it is cancelled', () => {
      fill.openFor('note-1');
      fill.cancel();

      expect(fill.target()).toBeNull();
      expect(harness.clipboard.content).toBe('');
    });
  });

  describe('filling', () => {
    it('copies the filled text and keeps the values for next time', async () => {
      fill.openFor('note-1');

      await fill.submit({ host: 'prod.internal', user: 'admin' });

      expect(harness.clipboard.content).toBe('psql -h prod.internal -U admin');
      expect(kept).toHaveBeenCalledWith('note-1', { host: 'prod.internal', user: 'admin' });
      expect(fill.target()).toBeNull();
    });

    /**
     * An empty value is a field the user left alone, and it is kept as such: the
     * snippet's own default is what fills it, and copying that default into the stored
     * value would freeze it the day the default changes.
     */
    it('keeps an empty value rather than substituting the default', async () => {
      fill.openFor('note-1');

      await fill.submit({ host: '', user: 'admin' });

      expect(kept).toHaveBeenCalledWith('note-1', { host: '', user: 'admin' });
    });

    it('does nothing at all when no note is open', async () => {
      await fill.submit({ host: 'prod.internal' });

      expect(harness.clipboard.content).toBe('');
      expect(kept).not.toHaveBeenCalled();
    });

    /** "Copy as is", for a note that holds template code without being a snippet. */
    it('copies the raw body without filling or keeping anything', async () => {
      fill.openFor('note-1');

      await fill.copyRaw();

      expect(harness.clipboard.content).toBe('psql -h {{host}} -U {{user}}');
      expect(kept).not.toHaveBeenCalled();
      expect(fill.target()).toBeNull();
    });

    /** The card holds a preview: filling it would copy a command cut short. */
    it('fills the whole body, not the preview the card holds', async () => {
      await fill.copyNote({ ...SNIPPET, placeholders: [], content: 'psql', truncated: true });
      expect(harness.clipboard.content).toBe('psql -h {{host}} -U {{user}}');

      harness.repository.setView({
        sections: [
          {
            key: 'week',
            notes: [{ ...SNIPPET, content: 'psql', truncated: true }],
            hasExpiringNotes: false,
            showCreateGhost: true,
          },
        ],
      });
      TestBed.inject(NotesRevision).bump();
      await vi.waitFor(() => expect(harness.canvas.visibleNotes()[0]?.truncated).toBe(true));
      fill.openFor('note-1');

      await fill.submit({ host: 'prod.internal', user: 'admin' });

      expect(harness.clipboard.content).toBe('psql -h prod.internal -U admin');
    });
  });

  describe('the editor preview', () => {
    it('says nothing before the first request', () => {
      expect(fill.preview()).toBeNull();
    });

    it('shows the body as it would be filled', async () => {
      await fill.refreshPreview({ content: 'psql -h {{host}}', values: { host: 'prod.internal' } });

      expect(fill.preview()).toBe('psql -h prod.internal');
    });

    /**
     * Two answers can come back out of order, and the newer request is the one the
     * user is looking at: an older answer landing late would overwrite it.
     */
    it('ignores an answer that arrives after a newer request', async () => {
      const stale = fill.refreshPreview({ content: '{{host}}', values: { host: 'stale' } });
      const current = fill.refreshPreview({ content: '{{host}}', values: { host: 'fresh' } });
      await Promise.all([stale, current]);

      expect(fill.preview()).toBe('fresh');
    });

    it('drops the preview when the editor moves to another note', async () => {
      await fill.refreshPreview({ content: '{{host}}', values: { host: 'prod' } });
      expect(fill.preview()).toBe('prod');

      await harness.store.openNote('note-1');

      await vi.waitFor(() => expect(fill.preview()).toBeNull());
    });

    /** The status banner and not the button's tick: the text exists only after a round trip. */
    it('says it copied the filled body once the filling came back', async () => {
      await fill.copyFilled({ content: 'psql -h {{host}}', values: { host: 'prod.internal' } });

      expect(harness.clipboard.content).toBe('psql -h prod.internal');
      expect(status.status()?.key).toBe('placeholders.copiedFilled');
    });

    it('stays silent when the clipboard refused', async () => {
      harness.clipboard.failNext = new Error('no clipboard');

      await fill.copyFilled({ content: '{{host}}', values: { host: 'prod' } });

      expect(status.status()).toBeNull();
    });
  });

  describe('filling from the palette', () => {
    it('does nothing when the palette is not waiting on a note', async () => {
      await fill.submitForPalette({ host: 'prod' });

      expect(harness.clipboard.content).toBe('');
      expect(kept).not.toHaveBeenCalled();
    });

    /** The palette is what copies, because copying is what dismisses it. */
    it('copies through the palette and keeps the values', async () => {
      const palette = TestBed.inject(PaletteStore);
      await palette.open();
      palette.highlight(0);
      await palette.chooseHighlighted();
      expect(palette.pendingFill()?.id).toBe('note-1');

      await fill.submitForPalette({ host: 'prod.internal', user: 'admin' });

      expect(harness.clipboard.content).toBe('psql -h prod.internal -U admin');
      expect(palette.isOpen()).toBe(false);
      expect(kept).toHaveBeenCalledWith('note-1', { host: 'prod.internal', user: 'admin' });
    });
  });
});
