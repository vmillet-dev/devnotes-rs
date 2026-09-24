import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NotesRepository } from '@core/data/notes.repository';
import { Space } from '@core/model/space.model';
import { LanguageBadgeComponent } from '@notes/ui/language-badge/language-badge.component';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NotesStore } from '@core/state/notes.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { CopyButtonComponent } from '@notes/ui/copy-button/copy-button.component';
import { NoteCardMenuComponent } from './note-card-menu/note-card-menu.component';
import { NoteActivation, NoteCardComponent } from './note-card.component';

const NEWLINE = String.fromCharCode(10);

const SPACES: readonly Space[] = [
  { id: 'work', name: 'Work', pinned: false },
  { id: 'personal', name: 'Personal', pinned: false },
];

describe('NoteCardComponent', () => {
  let fixture: ComponentFixture<NoteCardComponent>;

  function text(selector: string): string {
    return fixture.nativeElement.querySelector(selector).textContent.replace(/\s+/g, ' ').trim();
  }

  beforeEach(() => {
    // ⚠️ Only `Date`: the zoneless scheduler needs real rAF/setTimeout for `whenStable()`.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-01-10T12:00:00Z'));

    TestBed.configureTestingModule({
      imports: [NoteCardComponent],
      // The store answers about notes the canvas holds, so the card's own note is one.
      providers: [provideAppTesting({ spaces: SPACES, notes: [createNote({ id: 'note-42' })] })],
    });
    fixture = TestBed.createComponent(NoteCardComponent);
    fixture.componentRef.setInput('note', createNote());
    fixture.autoDetectChanges();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders the title and passes the language down to the language badge', async () => {
    fixture.componentRef.setInput('note', createNote({ title: 'My note', language: 'json' }));
    await fixture.whenStable();

    expect(text('.card-title')).toBe('My note');
    const badge = fixture.debugElement.query(By.directive(LanguageBadgeComponent))
      .componentInstance as LanguageBadgeComponent;
    expect(badge.language()).toBe('json');
  });

  /**
   * Structural, not visual: jsdom lays nothing out. ⚠️ The title is **outside** the band —
   * inline after the badge and the marks it began in the middle of the card, and what was
   * left of it wrapped.
   */
  /** ⚠️ One row, not two: the marks used to have a band above the title, and it said
   *  nothing about the note that the marks themselves do not. */
  it('puts the title, the tick and the marks on one row', async () => {
    fixture.componentRef.setInput('note', createNote({ title: 'My note', attachmentCount: 2 }));
    await fixture.whenStable();

    const row = fixture.nativeElement.querySelector('.card-title-row');
    expect(row.querySelector('[data-testid="note-card-title"]')).not.toBeNull();
    expect(row.querySelector('[data-testid="note-card-check"]')).not.toBeNull();
    expect(row.querySelector('.card-marks app-language-badge')).not.toBeNull();
    expect(row.querySelector('.card-marks [data-testid="note-card-clip"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('.card-head')).toBeNull();
  });

  /**
   * ⚠️ No band at all, and that is the point: the actions hang outside the card, so a todo
   * list with nothing to mark starts on its title instead of on 26px of nothing.
   */
  it('draws no band on a todo list with nothing to mark', async () => {
    fixture.componentRef.setInput('note', createNote({ title: 'My list', kind: 'checklist' }));
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.card-head')).toBeNull();
    expect(fixture.nativeElement.querySelector('app-language-badge')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-testid="note-card-title"]')).not.toBeNull();
    // The controls are still there, outside the card.
    expect(fixture.nativeElement.querySelector('[data-testid="note-card-check"]')).not.toBeNull();
  });

  /** ⚠️ A mark among the marks: it says what the note *is*, like the badge beside it. */
  it('puts the pin with the marks rather than over the card corner', async () => {
    fixture.componentRef.setInput('note', createNote({ title: 'My note', pinned: true }));
    await fixture.whenStable();

    const pin = fixture.nativeElement.querySelector('[data-testid="note-card-pin"]');
    expect(pin).not.toBeNull();
    expect(pin.closest('.card-marks')).not.toBeNull();
    // The glyph is decorative, so the state reaches a screen reader as text beside it.
    expect(fixture.nativeElement.querySelector('.card-marks .visually-hidden').textContent).toBe(
      'Note épinglée',
    );
  });

  /** ⚠️ One ⚡, not two: the mark says the copy will ask for the values before it copies. */
  it('marks a snippet with fields once, and copies through the form', async () => {
    fixture.componentRef.setInput(
      'note',
      createNote({ title: 'psql', placeholders: [{ name: 'host', defaultValue: '', value: '' }] }),
    );
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelectorAll('[data-testid="note-card-fields"]')).toHaveLength(1);
    expect(fixture.nativeElement.querySelector('[data-testid="note-card-fill"]')).not.toBeNull();
    // And no plain copy beside it: the two never coexist.
    expect(fixture.debugElement.query(By.directive(CopyButtonComponent))).toBeNull();
  });

  /**
   * ⚠️ The click surface is a layer of its own under the card: a `<button>` cannot hold
   * the buttons the header carries, which is what used to scatter them over the text.
   */
  it('opens from a layer under the card, not from the card itself', async () => {
    fixture.componentRef.setInput('note', createNote({ title: 'My note' }));
    await fixture.whenStable();

    const surface = fixture.nativeElement.querySelector('[data-testid="note-card-open"]');
    expect(surface.tagName).toBe('BUTTON');
    expect(surface.getAttribute('aria-label')).toBe('My note');
    // Nothing inside it: everything the card shows is drawn above it.
    expect(surface.children).toHaveLength(0);
    expect(fixture.nativeElement.querySelector('.card').tagName).toBe('DIV');
  });

  it('falls back to a translated placeholder for an untitled note', async () => {
    fixture.componentRef.setInput('note', createNote({ title: '' }));
    await fixture.whenStable();

    expect(text('.card-title')).toBe('Sans titre');
  });

  /** The cut is Rust's (`PREVIEW_LINES`): the card draws what the list sent. */
  it('shows the preview it is sent, line for line', async () => {
    fixture.componentRef.setInput(
      'note',
      createNote({ content: 'one\ntwo\nthree\nfour\nfive', truncated: true }),
    );
    await fixture.whenStable();

    const lines = fixture.debugElement.queryAll(By.css('.card-snippet .line-content'));
    expect(lines.map((line) => line.nativeElement.textContent)).toEqual([
      'one',
      'two',
      'three',
      'four',
      'five',
    ]);
  });

  it('copies the whole body of a note the list cut, read at the click', async () => {
    vi.spyOn(TestBed.inject(NotesRepository), 'get').mockResolvedValue(
      createNote({ id: 'note-42', content: 'the whole body' }),
    );
    fixture.componentRef.setInput('note', createNote({ id: 'note-42', content: 'the', truncated: true }));
    await fixture.whenStable();

    const copy = fixture.debugElement.query(By.directive(CopyButtonComponent))
      .componentInstance as CopyButtonComponent;
    const value = copy.value();

    expect(typeof value === 'function' && (await value())).toBe('the whole body');
  });

  describe('what a search put it here for', () => {
    it('shows the matching line instead of the head of the body', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({
          content: 'one\ntwo\nthree\nfour',
          searchHit: { field: 'body', excerpt: 'kubectl rollout restart' },
        }),
      );
      await fixture.whenStable();

      const lines = fixture.debugElement.queryAll(By.css('.card-snippet .line-content'));
      expect(lines.map((line) => line.nativeElement.textContent)).toEqual(['kubectl rollout restart']);
    });

    it('renders a tag or an item as prose, which is what they are', async () => {
      fixture.componentRef.setInput('note', createNote({ searchHit: { field: 'tag', excerpt: 'urgent' } }));
      await fixture.whenStable();

      // Not through the highlighter: it would paint the words of a tag as keywords.
      expect(fixture.debugElement.query(By.css('.card-snippet .line-content'))).toBeNull();
      expect(text('[data-testid="note-card-hit"]')).toBe('# urgent');

      fixture.componentRef.setInput(
        'note',
        createNote({ searchHit: { field: 'item', excerpt: 'Push the tag' } }),
      );
      await fixture.whenStable();

      expect(text('[data-testid="note-card-hit"]')).toBe('Push the tag');
    });

    describe('on a todo list, which has no body to quote into', () => {
      const items = [
        { text: 'Version bumped', done: true },
        { text: 'Lockfiles agree', done: true },
        { text: 'Changelog written', done: false },
        { text: 'Dry run of the release workflow', done: false },
        { text: 'Tag pushed', done: false },
      ];

      function itemTexts(): string[] {
        return fixture.debugElement
          .queryAll(By.css('[data-testid="note-card-item"] .item-text'))
          .map((node) => node.nativeElement.textContent.trim());
      }

      it('keeps the item that matched, and spends the other row on what is left', async () => {
        fixture.componentRef.setInput(
          'note',
          createNote({
            kind: 'checklist',
            items,
            searchHit: { field: 'item', excerpt: 'Dry run of the release workflow' },
          }),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['Changelog written', 'Dry run of the release workflow']);
      });

      it('keeps a matched item that is already ticked, which nothing else would show', async () => {
        fixture.componentRef.setInput(
          'note',
          createNote({ kind: 'checklist', items, searchHit: { field: 'item', excerpt: 'Version bumped' } }),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['Version bumped', 'Changelog written']);
      });

      /** ⚠️ The excerpt is clipped at 160 characters and never equals its own text. */
      it('finds the item behind a clipped excerpt', async () => {
        const long = 'x'.repeat(200);
        fixture.componentRef.setInput(
          'note',
          createNote({
            kind: 'checklist',
            items: [...items, { text: long, done: false }],
            searchHit: { field: 'item', excerpt: `${'x'.repeat(160)}…` },
          }),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['Changelog written', long]);
      });

      /** ⚠️ A row is not at the place it holds in the note; it is what it carries that gets written. */
      it('ticks the item it shows, not the one at the same place in the list', async () => {
        const store = TestBed.inject(NotesStore);
        const setChecklist = vi.spyOn(store, 'setChecklist').mockResolvedValue(undefined);
        fixture.componentRef.setInput(
          'note',
          createNote({
            id: 'note-42',
            kind: 'checklist',
            items,
            searchHit: { field: 'item', excerpt: 'Dry run of the release workflow' },
          }),
        );
        await fixture.whenStable();

        fixture.debugElement.queryAll(By.css('[data-testid="note-card-item"]'))[0].nativeElement.click();
        await fixture.whenStable();

        const written = setChecklist.mock.calls[0][1];
        expect(written.map((item) => item.done)).toEqual([true, true, true, false, false]);
      });
    });

    it('slides the tags it shows to the one that matched', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({
          tags: ['angular', 'ci', 'urgent'],
          searchHit: { field: 'tag', excerpt: 'urgent' },
        }),
      );
      await fixture.whenStable();

      const tags = fixture.debugElement.queryAll(By.css('.card-tags span'));
      expect(tags.map((tag) => tag.nativeElement.textContent)).toEqual(['#ci', '#urgent']);
    });

    it('keeps the head of the body when the back end quoted nothing', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'one\ntwo', searchHit: null }));
      await fixture.whenStable();

      const lines = fixture.debugElement.queryAll(By.css('.card-snippet .line-content'));
      expect(lines.map((line) => line.nativeElement.textContent)).toEqual(['one', 'two']);
    });
  });

  it('colours the snippet according to the note language', async () => {
    fixture.componentRef.setInput('note', createNote({ language: 'json', content: '{"a": 1}' }));
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.card-snippet .hljs-attr'))).not.toBeNull();
  });

  it('numbers no line in the snippet', async () => {
    fixture.componentRef.setInput('note', createNote({ content: 'one\ntwo' }));
    await fixture.whenStable();

    expect(fixture.debugElement.queryAll(By.css('.card-snippet .line-no'))).toHaveLength(0);
  });

  it('shows at most 2 tags', async () => {
    fixture.componentRef.setInput('note', createNote({ tags: ['a', 'b', 'c'] }));
    await fixture.whenStable();

    const tags = fixture.debugElement.queryAll(By.css('.card-tags span'));
    expect(tags.map((tag) => tag.nativeElement.textContent)).toEqual(['#a', '#b']);
  });

  describe('the folder chip', () => {
    it('names the folder and carries its colour as a swatch', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({ folderId: 'perf', folder: { id: 'perf', name: 'Perf', colour: 'amber' } }),
      );
      await fixture.whenStable();

      expect(text('[data-testid="note-card-folder"]')).toBe('Perf');
      expect(fixture.debugElement.query(By.css('.folder-swatch')).nativeElement.className).toContain(
        'is-amber',
      );
    });

    /** ⚠️ The absence reads on its own; an "unfiled" chip would soil every loose card. */
    it('shows nothing at all when the note has no folder', async () => {
      fixture.componentRef.setInput('note', createNote({ folderId: null, folder: null }));
      await fixture.whenStable();

      expect(fixture.debugElement.query(By.css('[data-testid="note-card-folder"]'))).toBeNull();
    });

    /** A tinted pill is what a tag is, and the two must not be read as the same thing. */
    it('is a neutral pill, never a tinted one like a tag', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({ folder: { id: 'perf', name: 'Perf', colour: 'amber' }, tags: ['urgent'] }),
      );
      await fixture.whenStable();

      const chip = fixture.debugElement.query(By.css('[data-testid="note-card-folder"]'));
      expect(chip.nativeElement.className).not.toContain('is-amber');
    });
  });

  describe('footer', () => {
    it('renders an expiry footer as a countdown', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({ footer: { kind: 'expiry', at: new Date('2026-01-13T12:00:00Z') } }),
      );
      await fixture.whenStable();

      expect(text('.card-footer span')).toBe('expire dans 3j');
    });

    it('renders an age footer as a relative time', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({ footer: { kind: 'age', at: new Date('2026-01-10T11:00:00Z') } }),
      );
      await fixture.whenStable();

      expect(text('.card-footer span')).toBe('il y a 1h');
    });

    it('renders a source footer as plain text, with nothing to translate', async () => {
      fixture.componentRef.setInput('note', createNote({ footer: { kind: 'source', value: 'API Gateway' } }));
      await fixture.whenStable();

      expect(text('.card-footer span')).toBe('API Gateway');
    });

    it('marks the footer stale on the backend flag, not on a threshold of its own', async () => {
      fixture.componentRef.setInput('note', createNote({ expiringSoon: true }));
      await fixture.whenStable();

      expect(fixture.debugElement.query(By.css('.card-footer span')).classes['stale']).toBe(true);
    });
  });

  it('exposes the pinned state as text, since the design only conveys it with a pictogram', async () => {
    fixture.componentRef.setInput('note', createNote({ pinned: true }));
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.visually-hidden').textContent).toBe('Note épinglée');
  });

  it('marks itself pinned from the note, and selected from the store', async () => {
    fixture.componentRef.setInput('note', createNote({ id: 'note-42', pinned: true }));
    await vi.waitFor(() => expect(TestBed.inject(NotesQueryStore).visibleNotes()).toHaveLength(1));

    await TestBed.inject(NotesStore).openNote('note-42');
    await fixture.whenStable();

    const card = fixture.debugElement.query(By.css('.card'));
    expect(card.classes['pinned']).toBe(true);
    expect(card.classes['selected']).toBe(true);
  });

  /** ⚠️ Still true, and now trivially so: the click surface holds nothing at all. */
  it('keeps every button free of flow content, which a <button> may not contain', () => {
    expect(fixture.nativeElement.querySelectorAll('button div')).toHaveLength(0);
  });

  it('emits opened with the note id when clicked', async () => {
    fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
    await fixture.whenStable();
    let emitted: string | undefined;
    fixture.componentInstance.opened.subscribe(({ noteId }) => (emitted = noteId));

    fixture.debugElement.query(By.css('.card-open')).triggerEventHandler('click', new MouseEvent('click'));

    expect(emitted).toBe('note-42');
  });

  it('reports the click modifiers instead of deciding what they mean', async () => {
    fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
    await fixture.whenStable();
    const activations: NoteActivation[] = [];
    fixture.componentInstance.opened.subscribe((activation) => activations.push(activation));
    const card = fixture.debugElement.query(By.css('.card-open'));

    card.triggerEventHandler('click', new MouseEvent('click', { ctrlKey: true }));
    card.triggerEventHandler('click', new MouseEvent('click', { shiftKey: true }));

    expect(activations).toEqual([
      { noteId: 'note-42', toggleChecked: true, extendRange: false },
      { noteId: 'note-42', toggleChecked: false, extendRange: true },
    ]);
  });

  it('checks the note without opening it', async () => {
    fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
    await fixture.whenStable();
    let opened = 0;
    fixture.componentInstance.opened.subscribe(() => (opened += 1));

    fixture.debugElement.query(By.css('.card-check')).triggerEventHandler('click', new MouseEvent('click'));

    expect(TestBed.inject(NoteSelectionStore).checkedIds().has('note-42')).toBe(true);
    expect(opened).toBe(0);
  });

  describe('{{fields}}', () => {
    it('offers the form instead of a copy, which would paste the raw snippet', async () => {
      fixture.componentRef.setInput(
        'note',
        createNote({ id: 'note-42', placeholders: [{ name: 'host', defaultValue: '', value: '' }] }),
      );
      await fixture.whenStable();
      const openFor = vi.spyOn(TestBed.inject(PlaceholderFillStore), 'openFor');

      expect(fixture.debugElement.query(By.directive(CopyButtonComponent))).toBeNull();
      fixture.debugElement.query(By.css('.card-fill')).triggerEventHandler('click', new MouseEvent('click'));

      expect(openFor).toHaveBeenCalledWith('note-42');
    });

    it('falls back to a plain copy button for a note without fields', () => {
      expect(fixture.debugElement.query(By.css('.card-fill'))).toBeNull();
      expect(fixture.debugElement.query(By.directive(CopyButtonComponent))).not.toBeNull();
    });
  });

  describe('actions menu', () => {
    function menu(): NoteCardMenuComponent {
      return fixture.debugElement.query(By.directive(NoteCardMenuComponent))
        .componentInstance as NoteCardMenuComponent;
    }

    it('hands the menu the note space so it can be excluded from the targets', async () => {
      fixture.componentRef.setInput('note', createNote({ spaceId: 'work' }));
      await vi.waitFor(() => expect(menu().spaces()).toEqual(SPACES));

      expect(menu().currentSpaceId()).toBe('work');
    });

    it('gives the menu the placeholder title the card itself shows', async () => {
      fixture.componentRef.setInput('note', createNote({ title: '' }));
      await fixture.whenStable();

      expect(menu().noteTitle()).toBe('Sans titre');
    });

    it('attaches the note id to a move, which the menu does not know', async () => {
      fixture.componentRef.setInput('note', createNote({ id: 'note-42', spaceId: 'work' }));
      await fixture.whenStable();
      const moveNote = vi.spyOn(TestBed.inject(NotesStore), 'moveNote').mockResolvedValue();

      menu().moveRequested.emit('personal');

      expect(moveNote).toHaveBeenCalledWith('note-42', 'personal');
    });

    it('attaches the note id to a deletion', async () => {
      fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
      await fixture.whenStable();
      const deleteNote = vi.spyOn(TestBed.inject(NotesStore), 'deleteNote').mockResolvedValue();

      menu().deleteRequested.emit();

      expect(deleteNote).toHaveBeenCalledWith('note-42');
    });
  });

  describe('a checklist note', () => {
    const checklist = (items: { text: string; done: boolean }[], overrides = {}) =>
      createNote({ id: 'note-42', kind: 'checklist', content: '', items, ...overrides });

    beforeEach(async () => {
      fixture.componentRef.setInput(
        'note',
        checklist([
          { text: 'Relire', done: true },
          { text: 'Déployer', done: false },
        ]),
      );
      await fixture.whenStable();
    });

    it('shows the progress graphically and as text, since colour alone says nothing', () => {
      const fill = fixture.nativeElement.querySelector('.progress-fill') as HTMLElement;

      expect(fill.style.width).toBe('50%');
      expect(text('.progress-count')).toBe('1/2');
    });

    it('renders the items with their state', () => {
      const items = fixture.nativeElement.querySelectorAll('.card-item');

      expect(items).toHaveLength(2);
      expect(items[0].getAttribute('aria-checked')).toBe('true');
      expect(items[1].getAttribute('aria-checked')).toBe('false');
    });

    it('counts the items it could not fit rather than dropping them silently', async () => {
      fixture.componentRef.setInput(
        'note',
        checklist([1, 2, 3, 4, 5].map((n) => ({ text: `t${n}`, done: false }))),
      );
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelectorAll('.card-item')).toHaveLength(2);
      expect(text('[data-testid="note-card-more"]')).toBe('+3 autres');
    });

    describe('having more items than it has rows', () => {
      function itemTexts(): string[] {
        return [...fixture.nativeElement.querySelectorAll('.card-item .item-text')].map((node) =>
          (node as HTMLElement).textContent?.trim(),
        );
      }

      it('spends its two rows on what is still to do', async () => {
        fixture.componentRef.setInput(
          'note',
          checklist([
            { text: 'Relire', done: true },
            { text: 'Deployer', done: false },
            { text: 'Annoncer', done: false },
          ]),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['Deployer', 'Annoncer']);
      });

      it('falls back on the last done ones rather than drawing nothing', async () => {
        fixture.componentRef.setInput(
          'note',
          checklist(['Relire', 'Deployer', 'Annoncer', 'Archiver'].map((text) => ({ text, done: true }))),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['Annoncer', 'Archiver']);
      });

      it('counts every item it left out, wherever they sat', async () => {
        fixture.componentRef.setInput(
          'note',
          checklist([1, 2, 3, 4, 5].map((n) => ({ text: 't' + n, done: n === 1 }))),
        );
        await fixture.whenStable();

        expect(itemTexts()).toEqual(['t2', 't3']);
        expect(text('[data-testid="note-card-more"]')).toBe('+3 autres');
      });

      it('ticks the item it drew, not the one at the same place in the list', async () => {
        const setChecklist = vi.spyOn(TestBed.inject(NotesStore), 'setChecklist').mockResolvedValue();
        fixture.componentRef.setInput(
          'note',
          checklist([
            { text: 'Relire', done: true },
            { text: 'Deployer', done: false },
            { text: 'Annoncer', done: false },
          ]),
        );
        await fixture.whenStable();

        fixture.nativeElement.querySelectorAll('.card-item')[0].click();

        expect(setChecklist.mock.calls[0][1].map((item) => item.done)).toEqual([true, true, false]);
      });
    });

    it('shows no language badge, a checklist having no format to announce', () => {
      expect(fixture.debugElement.query(By.directive(LanguageBadgeComponent))).toBeNull();
    });

    it('saves the whole list with the ticked item flipped', () => {
      const setChecklist = vi.spyOn(TestBed.inject(NotesStore), 'setChecklist').mockResolvedValue();

      fixture.nativeElement.querySelectorAll('.card-item')[1].click();

      expect(setChecklist).toHaveBeenCalledWith('note-42', [
        { text: 'Relire', done: true },
        { text: 'Déployer', done: true },
      ]);
    });

    it('does not open the note when a box is ticked', () => {
      const opened: NoteActivation[] = [];
      fixture.componentInstance.opened.subscribe((activation) => opened.push(activation));

      fixture.nativeElement.querySelector('.card-item').click();

      expect(opened).toEqual([]);
    });

    it('hands the copy button a markdown rendering, the note having no content', async () => {
      const copy = fixture.debugElement.query(By.directive(CopyButtonComponent))
        .componentInstance as CopyButtonComponent;
      const value = copy.value();

      expect(typeof value === 'function' && (await value())).toBe(
        ['- [x] Relire', '- [ ] Déployer'].join(NEWLINE),
      );
    });

    it('shows no code viewer, a checklist having no body to colour', () => {
      expect(fixture.nativeElement.querySelector('.card-snippet')).toBeNull();
    });
  });

  /**
   * ⚠️ Real words and not only a red border: the border says something is wrong, not that
   * one more press trashes the note. `role="status"` is what announces it.
   */
  describe('armed for deletion', () => {
    const band = () => fixture.nativeElement.querySelector('[data-testid="note-card-arming"]');

    beforeEach(async () => {
      fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
      await fixture.whenStable();
    });

    it('says nothing until the first Delete', () => {
      expect(band()).toBeNull();
      expect(fixture.debugElement.query(By.css('.card')).classes['armed']).toBeFalsy();
    });

    it('says what one more press would do, and draws the card in red', async () => {
      TestBed.inject(NoteSelectionStore).armForDeletion('note-42');
      await fixture.whenStable();

      expect(band()?.textContent).toContain('Suppr');
      expect(band()?.getAttribute('role')).toBe('status');
      expect(fixture.debugElement.query(By.css('.card')).classes['armed']).toBe(true);
    });

    it('leaves the other cards alone', async () => {
      TestBed.inject(NoteSelectionStore).armForDeletion('another-note');
      await fixture.whenStable();

      expect(band()).toBeNull();
    });
  });

  /**
   * ⚠️ The card is rebuilt whenever its section changes, and the first character typed
   * into the search field switches the canvas to one flat `results` section — so the
   * effect that follows the canvas cursor runs while somebody is typing somewhere else.
   */
  describe('following the canvas cursor', () => {
    beforeEach(async () => {
      fixture.componentRef.setInput('note', createNote({ id: 'note-42' }));
      await fixture.whenStable();
      (document.activeElement as HTMLElement | null)?.blur();
    });

    it('takes the keyboard when nothing else is holding it', async () => {
      TestBed.inject(NoteSelectionStore).focusNote('note-42');
      await fixture.whenStable();

      expect(document.activeElement).toBe(fixture.nativeElement.querySelector('.card-open'));
    });

    it('leaves a field that is being typed in alone', async () => {
      const field = document.createElement('input');
      document.body.append(field);
      field.focus();

      TestBed.inject(NoteSelectionStore).focusNote('note-42');
      await fixture.whenStable();

      expect(document.activeElement).toBe(field);
      field.remove();
    });
  });
});
