import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { CodeViewerComponent } from '@notes/ui/code-viewer/code-viewer.component';
import { LifecycleBadgeComponent } from './lifecycle-badge/lifecycle-badge.component';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';
import { LANGUAGE_LABELS } from '@core/model/language.model';
import { Folder } from '@core/model/folder.model';
import { Note, NotePatch } from '@core/model/note.model';
import { Space } from '@core/model/space.model';
import { AttachmentsStore } from '@core/state/attachments.store';
import { NotesStore } from '@core/state/notes.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { createNote as noteFixture } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ExternalLinksService } from '@core/services/links/external-links.service';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { NotesRepository } from '@core/data/notes.repository';
import { PRETTIER_ADAPTER } from '@core/services/format/formatter.service';
import { FakePrettier } from '@testing/fake-prettier';
import { NoteEditorOverlayComponent } from './note-editor-overlay.component';
import { RichTextEditorComponent } from './rich-text-editor/rich-text-editor.component';

/** A snippet unless a test says otherwise: a Note has the rich editor, tested apart. */
const createNote = (overrides: Partial<Note> = {}): Note => noteFixture({ language: 'sh', ...overrides });

const SPACES: readonly Space[] = [
  { id: 'space-1', name: 'Perso', pinned: false },
  { id: 'work', name: 'Work', pinned: false },
];

const FOLDERS: readonly Folder[] = [
  {
    id: 'perf',
    spaceId: 'space-1',
    name: 'Perf',
    colour: 'amber',
    createdAt: new Date('2026-01-01T10:00:00Z'),
  },
];

describe('NoteEditorOverlayComponent', () => {
  let fixture: ComponentFixture<NoteEditorOverlayComponent>;

  /**
   * What the overlay asked of the store that owns the open note. Recorded rather than
   * carried out: the note on screen is a fixture no repository holds.
   */
  let asked: {
    patches: ((patch: NotePatch) => void)[];
    closes: number;
    deletions: number;
    filings: (string | null)[];
    values: Record<string, string>[];
    priorities: string[];
  };

  function listenToTheStore(): void {
    asked = { patches: [], closes: 0, deletions: 0, filings: [], values: [], priorities: [] };
    const store = TestBed.inject(NotesStore);
    vi.spyOn(store, 'applyPatch').mockImplementation(async (_id, patch) => {
      for (const listener of asked.patches) listener(patch);
    });
    vi.spyOn(store, 'closeOverlay').mockImplementation(() => {
      asked.closes += 1;
    });
    vi.spyOn(store, 'deleteNote').mockImplementation(async () => {
      asked.deletions += 1;
    });
    vi.spyOn(store, 'fileNote').mockImplementation(async (_id, folderId) => {
      asked.filings.push(folderId);
    });
    vi.spyOn(store, 'setPlaceholderValues').mockImplementation(async (_id, values) => {
      asked.values.push(values);
    });
    vi.spyOn(store, 'setPriority').mockImplementation(async (_id, priority) => {
      asked.priorities.push(priority);
    });
  }

  function titleInput(): HTMLInputElement {
    return fixture.nativeElement.querySelector('.overlay-title-input');
  }

  function bodyEditor(): HTMLTextAreaElement {
    return fixture.nativeElement.querySelector('.overlay-body-editor');
  }

  function codeViewer(): CodeViewerComponent {
    return fixture.debugElement.query(By.directive(CodeViewerComponent))
      .componentInstance as CodeViewerComponent;
  }

  function toolbarButton(selector: string): HTMLButtonElement {
    return fixture.nativeElement.querySelector(selector);
  }

  function fullscreenButton(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.icon-btn:not(.close-btn)');
  }

  function text(selector: string): string {
    return fixture.nativeElement.querySelector(selector).textContent.replace(/\s+/g, ' ').trim();
  }

  /** What successive patches carried for one field. */
  function patched<K extends keyof NotePatch>(field: K): NonNullable<NotePatch[K]>[] {
    const values: NonNullable<NotePatch[K]>[] = [];
    asked.patches.push((patch) => {
      const value = patch[field];
      if (value !== undefined) values.push(value as NonNullable<NotePatch[K]>);
    });
    return values;
  }

  async function type(element: HTMLInputElement | HTMLTextAreaElement, value: string): Promise<void> {
    element.value = value;
    element.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  function createOverlay(): void {
    fixture = TestBed.createComponent(NoteEditorOverlayComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
  }

  function preferences(): PreferencesService {
    return TestBed.inject(PreferencesService);
  }

  /** Rebuilds the overlay so a preference seeded in storage is read at construction. */
  function recreateOverlay(): void {
    fixture.nativeElement.remove();
    fixture.destroy();
    createOverlay();
  }

  beforeEach(() => {
    // Only `Date`: the zoneless scheduler needs real rAF/setTimeout for `whenStable()`.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-01-10T12:00:00Z'));

    TestBed.configureTestingModule({
      imports: [NoteEditorOverlayComponent],
      providers: [provideAppTesting({ spaces: SPACES, folders: FOLDERS })],
    });
    listenToTheStore();
    createOverlay();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
    vi.useRealTimers();
  });

  it('renders nothing when there is no note', () => {
    expect(fixture.debugElement.query(By.css('.dialog-backdrop'))).toBeNull();
  });

  it('renders the note title, tags, lifecycle and code content', async () => {
    const note = createNote({
      title: 'My note',
      tags: ['a', 'b'],
      content: '{"x":1}',
      language: 'json',
      lifecycle: { kind: 'permanent' },
    });
    fixture.componentRef.setInput('note', note);
    await fixture.whenStable();

    expect(titleInput().value).toBe('My note');

    const pills = fixture.debugElement
      .queryAll(By.directive(TagPillComponent))
      .map((pill) => pill.componentInstance as TagPillComponent);
    expect(pills.map((pill) => pill.label())).toEqual(['a', 'b']);
    expect(pills.every((pill) => pill.active() && !pill.interactive())).toBe(true);

    const badge = fixture.debugElement.query(By.directive(LifecycleBadgeComponent))
      .componentInstance as LifecycleBadgeComponent;
    expect(badge.lifecycle()).toEqual({ kind: 'permanent' });

    const viewer = fixture.debugElement.query(By.directive(CodeViewerComponent))
      .componentInstance as CodeViewerComponent;
    expect(viewer.content()).toBe('{"x":1}');
    expect(viewer.language()).toBe('json');
  });

  it('invites the user to write when the note is empty', async () => {
    fixture.componentRef.setInput('note', createNote({ content: '' }));
    await fixture.whenStable();

    expect(bodyEditor().placeholder).toBe('Commencer à écrire…');
  });

  it('computes the language label, line count and byte size in the footer', async () => {
    fixture.componentRef.setInput('note', createNote({ content: 'line one\nline two', language: 'js' }));
    await fixture.whenStable();

    expect(text('.overlay-footer span')).toBe('JS · 2 lignes · 17 octets · UTF-8 · LF');
  });

  describe('accessibility', () => {
    beforeEach(async () => {
      fixture.componentRef.setInput('note', createNote({ title: 'My note' }));
      await fixture.whenStable();
    });

    it('declares itself a modal dialog named by its title field', () => {
      const panel = fixture.nativeElement.querySelector('.dialog-panel');

      expect(panel.getAttribute('role')).toBe('dialog');
      expect(panel.getAttribute('aria-modal')).toBe('true');
      expect(panel.getAttribute('aria-labelledby')).toBe(titleInput().id);
    });

    it('moves focus into the dialog when it opens', () => {
      expect(fixture.nativeElement.querySelector('.dialog-panel').contains(document.activeElement)).toBe(
        true,
      );
    });

    it('labels the title field, the body and the close button', () => {
      expect(titleInput().getAttribute('aria-label')).toBe('Titre de la note');
      expect(bodyEditor().getAttribute('aria-label')).toBe('Contenu de la note');
      expect(fixture.nativeElement.querySelector('.close-btn').getAttribute('aria-label')).toBe(
        "Fermer l'éditeur",
      );
    });

    it('exposes the pin button as a toggle', () => {
      expect(toolbarButton('[data-testid="editor-pin"]').getAttribute('aria-pressed')).toBe('false');
    });

    it('names the tag removal buttons after the tag they remove', async () => {
      fixture.componentRef.setInput('note', createNote({ tags: ['urgent'] }));
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('.tag-remove').getAttribute('aria-label')).toBe(
        'Retirer le tag urgent',
      );
    });
  });

  describe('title editing', () => {
    it('emits the new title on blur rather than on every keystroke', async () => {
      fixture.componentRef.setInput('note', createNote({ title: 'Before' }));
      await fixture.whenStable();
      const emitted = patched('title');

      await type(titleInput(), 'After');
      expect(emitted).toEqual([]);

      titleInput().dispatchEvent(new Event('blur'));

      expect(emitted).toEqual(['After']);
    });

    it('resets the draft when another note is opened', async () => {
      fixture.componentRef.setInput('session', 1);
      fixture.componentRef.setInput('note', createNote({ id: 'a', title: 'First' }));
      await fixture.whenStable();
      await type(titleInput(), 'Typed but not confirmed');

      // Opening another note is what moves the session; saving this one is not.
      fixture.componentRef.setInput('session', 2);
      fixture.componentRef.setInput('note', createNote({ id: 'b', title: 'Second' }));
      await fixture.whenStable();

      expect(titleInput().value).toBe('Second');
    });
  });

  describe('body editing', () => {
    it('is editable straight away, without a preview to click through first', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'before' }));
      await fixture.whenStable();

      expect(bodyEditor().value).toBe('before');
    });

    it('keeps the highlighted layer in sync with the draft as the user types', async () => {
      fixture.componentRef.setInput('note', createNote({ content: '{"a":1}', language: 'json' }));
      await fixture.whenStable();

      await type(bodyEditor(), '{"a":2}');

      expect(codeViewer().content()).toBe('{"a":2}');
      expect(codeViewer().language()).toBe('json');
    });

    it('hides the highlighted layer from screen readers, which already read the textarea', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'x' }));
      await fixture.whenStable();

      const viewer = fixture.debugElement.query(By.directive(CodeViewerComponent));
      expect(viewer.nativeElement.getAttribute('aria-hidden')).toBe('true');
    });

    it('emits the new content on blur', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'before' }));
      await fixture.whenStable();
      const emitted = patched('content');

      await type(bodyEditor(), 'after');
      expect(emitted).toEqual([]);

      bodyEditor().dispatchEvent(new Event('blur'));
      await fixture.whenStable();

      expect(emitted).toEqual(['after']);
    });

    it('updates the footer stats as the user types, before anything is saved', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'a' }));
      await fixture.whenStable();

      await type(bodyEditor(), 'one\ntwo');

      expect(text('.overlay-footer span')).toBe('SH · 2 lignes · 7 octets · UTF-8 · LF');
    });

    it('still defers plain typing to the blur', async () => {
      fixture.componentRef.setInput('note', createNote({ content: '' }));
      await fixture.whenStable();
      const emitted = patched('content');

      bodyEditor().value = 'a';
      bodyEditor().dispatchEvent(new InputEvent('input', { inputType: 'insertText' }));
      await fixture.whenStable();

      expect(emitted).toEqual([]);
    });

    it('leaves the body on Escape instead of closing the whole overlay', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'before' }));
      await fixture.whenStable();
      const emitted = patched('content');

      bodyEditor().focus();
      await type(bodyEditor(), 'after');
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
      await fixture.whenStable();

      expect(emitted).toEqual(['after']);
      expect(asked.closes).toBe(0);
      expect(document.activeElement).not.toBe(bodyEditor());
    });
  });

  describe('Tab in the code field', () => {
    /** jsdom has no editing commands: this one does what the WebView's `insertText` does. */
    beforeEach(() => {
      Object.defineProperty(document, 'execCommand', {
        configurable: true,
        value: (command: string, _ui: boolean, text: string) => {
          const field = bodyEditor();
          field.setRangeText(
            command === 'delete' ? '' : text,
            field.selectionStart,
            field.selectionEnd,
            'end',
          );
          field.dispatchEvent(new InputEvent('input', { inputType: 'insertText' }));
          return true;
        },
      });
    });

    afterEach(() => {
      Reflect.deleteProperty(document, 'execCommand');
    });

    async function openCode(content: string, language: Note['language']): Promise<void> {
      fixture.componentRef.setInput('note', createNote({ content, language }));
      await fixture.whenStable();
    }

    function press(init: KeyboardEventInit): KeyboardEvent {
      const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true, ...init });
      bodyEditor().dispatchEvent(event);
      return event;
    }

    it('indents by the note language, and keeps the focus in the field', async () => {
      await openCode('if ready:\nstart()', 'py');
      bodyEditor().setSelectionRange(10, 10);

      const tab = press({});
      await fixture.whenStable();

      expect(tab.defaultPrevented).toBe(true);
      expect(bodyEditor().value).toBe('if ready:\n    start()');
      expect(text('.overlay-footer span')).toContain('21 octets');
    });

    it('indents with what Preferences chose over the language', async () => {
      TestBed.inject(SettingsStore).codeIndent.write('tab');
      await openCode('start()', 'py');
      bodyEditor().setSelectionRange(0, 0);

      press({});

      expect(bodyEditor().value).toBe('\tstart()');
    });

    it('takes a level back with Shift+Tab, on every line selected', async () => {
      await openCode('  a\n  b', 'sh');
      bodyEditor().setSelectionRange(0, 6);

      press({ shiftKey: true });

      expect(bodyEditor().value).toBe('a\nb');
    });

    it('leaves Ctrl+Tab to the window', async () => {
      await openCode('a', 'sh');

      expect(press({ ctrlKey: true }).defaultPrevented).toBe(false);
    });
  });

  describe('formatting with Prettier', () => {
    let prettier: FakePrettier;
    /** What the field held before each edit, for the fake `undo`. */
    let history: string[];
    let commands: string[];

    /** jsdom has no editing commands: these do what the WebView's do, undo included. */
    beforeEach(() => {
      prettier = TestBed.inject(PRETTIER_ADAPTER) as FakePrettier;
      history = [];
      commands = [];
      Object.defineProperty(document, 'execCommand', {
        configurable: true,
        value: (command: string, _ui: boolean, text: string) => {
          commands.push(command);
          const field = bodyEditor();
          if (command === 'undo') {
            field.value = history.pop() ?? field.value;
          } else {
            history.push(field.value);
            field.setRangeText(
              command === 'delete' ? '' : text,
              field.selectionStart,
              field.selectionEnd,
              'end',
            );
          }
          field.dispatchEvent(new InputEvent('input'));
          return true;
        },
      });
    });

    afterEach(() => {
      Reflect.deleteProperty(document, 'execCommand');
    });

    async function openCode(content: string, language: Note['language'] = 'ts'): Promise<void> {
      fixture.componentRef.setInput('note', createNote({ content, language }));
      await fixture.whenStable();
    }

    function formatButton(): HTMLButtonElement {
      return fixture.nativeElement.querySelector('[data-testid="editor-format"]');
    }

    async function format(): Promise<void> {
      formatButton().click();
      await fixture.whenStable();
    }

    function notice(): string | null {
      const element = fixture.nativeElement.querySelector('[data-testid="editor-format-notice"]');
      return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
    }

    it('offers the button on a language Prettier formats, and says why not elsewhere', async () => {
      await openCode('a', 'ts');
      expect(formatButton().getAttribute('aria-disabled')).toBe('false');
      expect(formatButton().title).toBe('Formater avec Prettier (Maj+Alt+F)');
      expect(text('[data-testid="editor-prettier"]')).toBe('Prettier 3 · réglages de la bibliothèque');

      await openCode('select 1', 'sql');
      expect(formatButton().getAttribute('aria-disabled')).toBe('true');
      expect(formatButton().title).toBe('Prettier ne sait pas formater SQL');
      expect(fixture.nativeElement.querySelector('[data-testid="editor-prettier"]')).toBeNull();

      await format();
      expect(prettier.requests).toEqual([]);
    });

    it('sends the draft, the caret and the indentation the Tab key uses', async () => {
      TestBed.inject(SettingsStore).codeIndent.write('tab');
      await openCode('f(a,b)');
      bodyEditor().setSelectionRange(2, 2);

      await format();

      expect(prettier.requests).toEqual([
        expect.objectContaining({
          text: 'f(a,b)',
          cursor: 2,
          parser: 'typescript',
          options: expect.objectContaining({ useTabs: true }),
        }),
      ]);
    });

    it('writes the result through the field, marks the lines and says how many', async () => {
      prettier.answer = {
        kind: 'formatted',
        text: 'f(\n  a,\n  b,\n);',
        cursor: 3,
        changedLines: [0, 1, 2, 3],
      };
      await openCode('f(a,b)');

      await format();

      expect(bodyEditor().value).toBe('f(\n  a,\n  b,\n);');
      expect(commands).toEqual(['insertText']);
      expect(bodyEditor().selectionStart).toBe(3);
      expect([...codeViewer().markedLines()]).toEqual([0, 1, 2, 3]);
      expect(notice()).toBe('✓ Formaté avec Prettier · 4 lignes modifiées Annuler Ctrl Z');
    });

    it('lets the next keystroke clear the marks and the notice', async () => {
      prettier.answer = { kind: 'formatted', text: 'b', cursor: 1, changedLines: [0] };
      await openCode('a');
      await format();

      await type(bodyEditor(), 'bc');

      expect(codeViewer().markedLines().size).toBe(0);
      expect(notice()).toBeNull();
    });

    it("undoes through the field's own undo", async () => {
      prettier.answer = { kind: 'formatted', text: 'b', cursor: 1, changedLines: [0] };
      await openCode('a');
      await format();

      fixture.nativeElement.querySelector('[data-testid="editor-format-undo"]').click();
      await fixture.whenStable();

      expect(commands).toEqual(['insertText', 'undo']);
      expect(bodyEditor().value).toBe('a');
      expect(notice()).toBeNull();
    });

    it('formats on Shift+Alt+F from anywhere in the editor', async () => {
      await openCode('a');

      const event = new KeyboardEvent('keydown', {
        key: 'F',
        shiftKey: true,
        altKey: true,
        bubbles: true,
        cancelable: true,
      });
      titleInput().dispatchEvent(event);
      await fixture.whenStable();

      expect(event.defaultPrevented).toBe(true);
      expect(prettier.requests).toHaveLength(1);
    });

    it('answers by position when Alt prints another character', async () => {
      await openCode('a');

      bodyEditor().dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Ï', code: 'KeyF', shiftKey: true, altKey: true, bubbles: true }),
      );
      bodyEditor().dispatchEvent(new KeyboardEvent('keydown', { key: 'F', shiftKey: true, bubbles: true }));
      await fixture.whenStable();

      expect(prettier.requests).toHaveLength(1);
    });

    it('says where a syntax error is and puts the caret there, the text untouched', async () => {
      prettier.answer = { kind: 'syntax', line: 2, column: 8 };
      await openCode('const a = {\n  b: 1,,\n}');

      await format();

      expect(bodyEditor().value).toBe('const a = {\n  b: 1,,\n}');
      expect(bodyEditor().selectionStart).toBe(19);
      expect(notice()).toBe("Erreur de syntaxe ligne 2, colonne 8 : le texte n'a pas changé.");
    });

    it('says so when a field would not have survived, or nothing needed doing', async () => {
      await openCode('{{a}}');

      prettier.answer = { kind: 'fields' };
      await format();
      expect(notice()).toBe("Un champ n'aurait pas survécu au formatage : le texte n'a pas changé.");

      prettier.answer = { kind: 'unchanged' };
      await format();
      expect(notice()).toBe('Déjà formaté : rien à changer.');
      expect(commands).toEqual([]);
    });

    it('drops an answer about a text typed over while Prettier ran', async () => {
      prettier.answer = { kind: 'formatted', text: 'formatted', cursor: 0, changedLines: [0] };
      await openCode('a');
      vi.spyOn(prettier, 'run').mockImplementation(async () => {
        bodyEditor().value = 'typed meanwhile';
        return prettier.answer;
      });

      await format();

      expect(bodyEditor().value).toBe('typed meanwhile');
      expect(commands).toEqual([]);
    });

    it('puts the notice away after a few seconds, and leaves the marks', async () => {
      vi.useFakeTimers({ toFake: ['Date', 'setTimeout', 'clearTimeout'] });
      prettier.answer = { kind: 'formatted', text: 'b', cursor: 1, changedLines: [0] };
      await openCode('a');
      await format();

      vi.advanceTimersByTime(6000);
      fixture.detectChanges();

      expect(notice()).toBeNull();
      expect(codeViewer().markedLines().size).toBe(1);
    });

    it('names the line endings in the footer', async () => {
      await openCode('a\r\nb');

      expect(text('.overlay-footer span')).toContain('UTF-8 · CRLF');
    });
  });

  describe('a Note', () => {
    function rich(): RichTextEditorComponent {
      return fixture.debugElement.query(By.directive(RichTextEditorComponent))
        .componentInstance as RichTextEditorComponent;
    }

    async function openText(content = ''): Promise<void> {
      fixture.componentRef.setInput('note', createNote({ kind: 'note', language: 'txt', content }));
      await fixture.whenStable();
      await vi.waitFor(() =>
        expect(fixture.debugElement.query(By.directive(RichTextEditorComponent))).not.toBeNull(),
      );
    }

    it('is written in the rich editor, and no code field is drawn', async () => {
      await openText('**bold**');

      expect(bodyEditor()).toBeNull();
      expect(rich().content()).toBe('**bold**');
    });

    it('says what it is where a snippet names its format', async () => {
      await openText();

      expect(fixture.nativeElement.querySelector('[data-testid="editor-kind"]').textContent.trim()).toBe(
        'Note',
      );
    });

    /** The draft's words, not its Markdown: they follow the typing before anything is saved. */
    it('counts its words in the footer, and says how it is stored', async () => {
      await openText('**Ship** it');
      const stats = (): string =>
        fixture.nativeElement.querySelector('[data-testid="editor-stats"]').textContent.trim();

      expect(stats()).toBe('Note · 2 mots · enregistrée en Markdown');

      rich().changed.emit('## Standup\n\n- [x] ship it today');
      await fixture.whenStable();
      expect(stats()).toContain('4 mots');
    });

    it('keeps what is typed as the draft, and writes it on blur', async () => {
      await openText();
      const emitted = patched('content');

      rich().changed.emit('## Standup');
      await fixture.whenStable();
      expect(emitted).toEqual([]);

      rich().blurred.emit();
      await fixture.whenStable();
      expect(emitted).toEqual(['## Standup']);
    });

    /** Code keeps its characters and gets its language: the note becomes a snippet. */
    it('hands a paste of code over as a snippet in the language Rust read in it', async () => {
      await openText();
      (TestBed.inject(NotesRepository) as unknown as FakeNotesRepository).detectedLanguage = 'sh';
      const contents = patched('content');
      const languages = patched('language');
      const kinds = patched('kind');

      rich().pastedIntoEmpty.emit('#!/bin/sh\necho hi');
      await vi.waitFor(() => expect(languages).toEqual(['sh']));

      expect(contents).toEqual(['#!/bin/sh\necho hi']);
      expect(kinds).toEqual(['snippet']);
    });

    it('keeps a paste of prose in the rich editor, as a Note', async () => {
      await openText();
      const contents = patched('content');
      const languages = patched('language');

      rich().pastedIntoEmpty.emit('Meeting notes');
      await vi.waitFor(() => expect(contents).toEqual(['Meeting notes']));

      expect(languages).toEqual([]);
      expect(rich().content()).toBe('Meeting notes');
    });

    it('opens a Ctrl+clicked link through the service that only opens the web', async () => {
      await openText('[runbook](https://example.com)');
      const open = vi.spyOn(TestBed.inject(ExternalLinksService), 'open').mockResolvedValue(true);

      rich().linkOpened.emit('https://example.com');

      expect(open).toHaveBeenCalledWith('https://example.com');
    });

    it('turns a pasted image into an attachment, as the code field does', async () => {
      await openText();
      const attach = vi.spyOn(TestBed.inject(AttachmentsStore), 'addPastedImage').mockResolvedValue();

      rich().imagePasted.emit();

      expect(attach).toHaveBeenCalled();
    });

    it('leaves the text on Escape instead of closing the whole overlay', async () => {
      await openText('draft');
      vi.spyOn(rich(), 'hasFocus').mockReturnValue(true);
      const blur = vi.spyOn(rich(), 'blur');

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

      expect(blur).toHaveBeenCalled();
      expect(asked.closes).toBe(0);
    });
  });

  describe('pasting an image', () => {
    function pasteEvent(types: string[], files: File[] = []): Event {
      const event = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(event, 'clipboardData', { value: { types, files } });
      return event;
    }

    /** What is asserted is that the editor hands the paste over, not what attaching does. */
    async function openNote(): Promise<ReturnType<typeof vi.fn>> {
      fixture.componentRef.setInput('note', createNote({ content: '' }));
      await fixture.whenStable();
      const pasted = vi.fn().mockResolvedValue(undefined);
      vi.spyOn(TestBed.inject(AttachmentsStore), 'addPastedImage').mockImplementation(pasted);
      return pasted;
    }

    it('turns an image into an attachment request, since a textarea cannot hold one', async () => {
      const pasted = await openNote();
      const event = pasteEvent(['Files'], [new File([], 'shot.png', { type: 'image/png' })]);

      bodyEditor().dispatchEvent(event);
      await fixture.whenStable();

      expect(pasted).toHaveBeenCalledOnce();
      expect(event.defaultPrevented).toBe(true);
    });

    it('recognises an image announced by type alone', async () => {
      const pasted = await openNote();

      bodyEditor().dispatchEvent(pasteEvent(['image/png']));
      await fixture.whenStable();

      expect(pasted).toHaveBeenCalledOnce();
    });

    it('leaves a paste carrying text to the field, which handles it natively', async () => {
      const pasted = await openNote();
      const event = pasteEvent(['text/plain', 'image/png']);

      bodyEditor().dispatchEvent(event);
      await fixture.whenStable();

      expect(pasted).not.toHaveBeenCalled();
      expect(event.defaultPrevented).toBe(false);
    });

    it('ignores a paste that carries neither text nor an image', async () => {
      const pasted = await openNote();

      bodyEditor().dispatchEvent(pasteEvent(['application/pdf']));
      await fixture.whenStable();

      expect(pasted).not.toHaveBeenCalled();
    });
  });

  describe('source editing', () => {
    function sourceInput(): HTMLInputElement {
      return fixture.nativeElement.querySelector('.overlay-source-input');
    }

    it('shows the stored context and emits the new one on blur', async () => {
      fixture.componentRef.setInput('note', createNote({ source: 'Before' }));
      await fixture.whenStable();
      const emitted = patched('source');

      expect(sourceInput().value).toBe('Before');

      await type(sourceInput(), 'API Gateway / Auth');
      expect(emitted).toEqual([]);

      sourceInput().dispatchEvent(new Event('blur'));

      expect(emitted).toEqual(['API Gateway / Auth']);
    });

    it('confirms the pending context before closing', async () => {
      fixture.componentRef.setInput('note', createNote({ source: '' }));
      await fixture.whenStable();
      const emitted = patched('source');

      await type(sourceInput(), 'API');
      fixture.debugElement.query(By.css('.close-btn')).triggerEventHandler('click');

      expect(emitted).toEqual(['API']);
    });

    it('resets the draft when another note is opened', async () => {
      fixture.componentRef.setInput('session', 1);
      fixture.componentRef.setInput('note', createNote({ id: 'a', source: 'First' }));
      await fixture.whenStable();
      await type(sourceInput(), 'Edited');

      fixture.componentRef.setInput('session', 2);
      fixture.componentRef.setInput('note', createNote({ id: 'b', source: 'Second' }));
      await fixture.whenStable();

      expect(sourceInput().value).toBe('Second');
    });
  });

  describe('closing', () => {
    it('confirms the pending title before closing', async () => {
      fixture.componentRef.setInput('note', createNote({ title: 'Before' }));
      await fixture.whenStable();
      const titles = patched('title');

      await type(titleInput(), 'After');
      fixture.debugElement.query(By.css('.close-btn')).triggerEventHandler('click');

      expect(titles).toEqual(['After']);
    });

    it('confirms the pending body before closing', async () => {
      fixture.componentRef.setInput('note', createNote({ content: 'before' }));
      await fixture.whenStable();
      const contents = patched('content');

      await type(bodyEditor(), 'after');
      fixture.debugElement.query(By.css('.close-btn')).triggerEventHandler('click');

      expect(contents).toEqual(['after']);
    });

    it('emits closed when the close button is clicked', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      fixture.debugElement.query(By.css('.close-btn')).triggerEventHandler('click');

      expect(asked.closes).toBe(1);
    });

    it('emits closed when Escape is pressed while a note is open', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

      expect(asked.closes).toBe(1);
    });

    it('does not emit closed on Escape when there is no note open', () => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

      expect(asked.closes).toBe(0);
    });

    it('emits closed when clicking directly on the backdrop', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      fixture.nativeElement
        .querySelector('.dialog-backdrop')
        .dispatchEvent(new MouseEvent('click', { bubbles: true }));

      expect(asked.closes).toBe(1);
    });

    it('does not emit closed when clicking inside the panel', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      fixture.nativeElement
        .querySelector('.dialog-panel')
        .dispatchEvent(new MouseEvent('click', { bubbles: true }));

      expect(asked.closes).toBe(0);
    });
  });

  describe('language', () => {
    /** A menu now, not a `<select>`: nothing in the window wears the OS's chrome. */
    async function openLanguageMenu(): Promise<HTMLElement[]> {
      fixture.nativeElement.querySelector('[data-testid="choice-language"]').click();
      await fixture.whenStable();
      return [
        ...fixture.nativeElement.querySelectorAll('[data-testid="choice-panel-language"] [data-option-id]'),
      ];
    }

    it('lists every known language and names the note one', async () => {
      fixture.componentRef.setInput('note', createNote({ language: 'sql' }));
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="choice-language"]').textContent).toContain(
        LANGUAGE_LABELS['sql'],
      );
      // Against the label table rather than a copy of it.
      const options = await openLanguageMenu();
      expect(options.map((option) => option.getAttribute('data-option-id'))).toEqual(
        Object.keys(LANGUAGE_LABELS),
      );
    });

    it('offers no format to a Note or a checklist', async () => {
      for (const kind of ['note', 'checklist'] as const) {
        fixture.componentRef.setInput('note', createNote({ kind, language: 'txt' }));
        await fixture.whenStable();

        expect(fixture.nativeElement.querySelector('[data-testid="choice-language"]'), kind).toBeNull();
      }
    });

    it('emits the picked language', async () => {
      fixture.componentRef.setInput('note', createNote({ language: 'txt' }));
      await fixture.whenStable();
      const emitted = patched('language');

      const options = await openLanguageMenu();
      options.find((option) => option.getAttribute('data-option-id') === 'json')!.click();
      await fixture.whenStable();

      expect(emitted).toEqual(['json']);
    });
  });

  describe('placement', () => {
    /** Opens one of the two placement menus and picks an option once the store has it. */
    async function pick(kind: 'space' | 'folder', id: string): Promise<void> {
      fixture.nativeElement.querySelector(`[data-testid="choice-${kind}"]`).click();
      await fixture.whenStable();
      const option = await vi.waitFor(() => {
        const found = fixture.nativeElement.querySelector(
          `[data-testid="choice-panel-${kind}"] [data-option-id="${id}"]`,
        );
        if (!found) throw new Error(`no option ${id} yet`);
        return found as HTMLElement;
      });
      option.click();
      await fixture.whenStable();
    }

    it('moves the note to the space picked', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();
      const emitted = patched('spaceId');

      await pick('space', 'work');

      expect(emitted).toEqual(['work']);
    });

    /** Not a patch either: `set_priority` leaves the modification date alone. */
    it('gives the note the priority picked, and takes it off from "No priority"', async () => {
      fixture.componentRef.setInput('note', createNote({ priority: 'low' }));
      await fixture.whenStable();
      expect(fixture.nativeElement.querySelector('[data-testid="choice-priority"]').textContent).toContain(
        'Basse',
      );

      fixture.nativeElement.querySelector('[data-testid="choice-priority"]').click();
      await fixture.whenStable();
      fixture.nativeElement
        .querySelector('[data-testid="choice-panel-priority"] [data-option-id="high"]')
        .click();
      await fixture.whenStable();
      fixture.nativeElement.querySelector('[data-testid="choice-priority"]').click();
      await fixture.whenStable();
      fixture.nativeElement
        .querySelector('[data-testid="choice-panel-priority"] [data-testid="choice-none"]')
        .click();
      await fixture.whenStable();

      expect(asked.priorities).toEqual(['high', 'none']);
    });

    /** Not a patch: filing answers what it changed, which the undo needs. */
    it('files the note into the folder picked, among those of its space', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      await pick('folder', 'perf');

      expect(asked.filings).toEqual(['perf']);
    });
  });

  describe('tags', () => {
    it('sends the list without the tag that was removed', async () => {
      fixture.componentRef.setInput('note', createNote({ tags: ['keep', 'drop'] }));
      await fixture.whenStable();
      const emitted = patched('tags');

      const removeButtons = fixture.nativeElement.querySelectorAll('.tag-remove');
      (removeButtons[1] as HTMLButtonElement).click();

      expect(emitted).toEqual([['keep']]);
    });

    it('sends the list with the typed tag appended, and clears the field', async () => {
      fixture.componentRef.setInput('note', createNote({ tags: ['keep'] }));
      await fixture.whenStable();
      const emitted = patched('tags');

      const input = fixture.nativeElement.querySelector('.tag-add-input') as HTMLInputElement;
      await type(input, 'urgent');
      input.form?.dispatchEvent(new Event('submit', { cancelable: true }));
      await fixture.whenStable();

      expect(emitted).toEqual([['keep', 'urgent']]);
      expect(input.value).toBe('');
    });

    it('ignores a blank tag submission', async () => {
      fixture.componentRef.setInput('note', createNote({ tags: [] }));
      await fixture.whenStable();
      const emitted = patched('tags');

      const input = fixture.nativeElement.querySelector('.tag-add-input') as HTMLInputElement;
      await type(input, '   ');
      input.form?.dispatchEvent(new Event('submit', { cancelable: true }));
      await fixture.whenStable();

      expect(emitted).toEqual([]);
    });
  });

  describe('pinning and deletion', () => {
    it('asks for the opposite pin state, and reflects the current one', async () => {
      fixture.componentRef.setInput('note', createNote({ pinned: false }));
      await fixture.whenStable();
      const emitted = patched('pinned');

      const pinButton = fixture.debugElement.query(By.css('[data-testid="editor-pin"]'));
      expect(text('[data-testid="editor-pin"]')).toBe('📌 Épingler');
      expect(pinButton.classes['pinned']).toBeFalsy();

      pinButton.triggerEventHandler('click');

      expect(emitted).toEqual([true]);
    });

    it('shows the pinned label and class when the note is pinned', async () => {
      fixture.componentRef.setInput('note', createNote({ pinned: true }));
      await fixture.whenStable();

      expect(text('[data-testid="editor-pin"]')).toBe('📌 Épinglée');
      expect(fixture.debugElement.query(By.css('[data-testid="editor-pin"]')).classes['pinned']).toBe(true);
    });

    it('asks for confirmation before emitting a deletion', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      toolbarButton('.delete-btn').click();
      await fixture.whenStable();

      expect(asked.deletions).toBe(0);
      expect(text('.delete-btn')).toBe('🗑 Confirmer ?');

      toolbarButton('.delete-btn').click();

      expect(asked.deletions).toBe(1);
    });

    it('drops the pending confirmation when another note is opened', async () => {
      fixture.componentRef.setInput('session', 1);
      fixture.componentRef.setInput('note', createNote({ id: 'a' }));
      await fixture.whenStable();
      toolbarButton('.delete-btn').click();
      await fixture.whenStable();

      fixture.componentRef.setInput('session', 2);
      fixture.componentRef.setInput('note', createNote({ id: 'b' }));
      await fixture.whenStable();

      expect(text('.delete-btn')).toBe('🗑 Supprimer');
    });
  });

  describe('expiry', () => {
    function expiryInput(): HTMLInputElement {
      return fixture.nativeElement.querySelector('.overlay-expiry-input');
    }

    async function pick(value: string): Promise<void> {
      expiryInput().value = value;
      expiryInput().dispatchEvent(new Event('change'));
      await fixture.whenStable();
    }

    it('is empty for a permanent note', async () => {
      fixture.componentRef.setInput('note', createNote({ lifecycle: { kind: 'permanent' } }));
      await fixture.whenStable();

      expect(expiryInput().value).toBe('');
    });

    it('shows an existing deadline in the local timezone', async () => {
      const at = new Date(2026, 7, 1, 23, 59, 59, 999);
      fixture.componentRef.setInput('note', createNote({ lifecycle: { kind: 'expires', at } }));
      await fixture.whenStable();

      expect(expiryInput().value).toBe('2026-08-01');
    });

    it('emits a deadline at the end of the chosen local day', async () => {
      const emitted = patched('lifecycle');
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      await pick('2026-08-01');

      expect(emitted).toEqual([{ kind: 'expires', at: new Date(2026, 7, 1, 23, 59, 59, 999) }]);
    });

    it('makes the note permanent again when the field is cleared', async () => {
      const emitted = patched('lifecycle');
      fixture.componentRef.setInput(
        'note',
        createNote({
          lifecycle: { kind: 'expires', at: new Date(2026, 7, 1) },
        }),
      );
      await fixture.whenStable();

      await pick('');

      expect(emitted).toEqual([{ kind: 'permanent' }]);
    });

    it('is labelled, since a bare date field says nothing about what it sets', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      expect(expiryInput().getAttribute('aria-label')).toBe('Échéance de la note (vide = permanente)');
    });
  });

  /** A band of its own between the header and the attachments, not wedged in the meta row. */
  describe('where the note lives', () => {
    it('is a band after the meta row and before the attachments, both choices inside it', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      const root = fixture.nativeElement as HTMLElement;
      const band = root.querySelector('[data-testid="editor-placement"]')!;
      const metaRow = root.querySelector('.overlay-meta-row')!;
      const strip = root.querySelector('app-attachment-strip')!;

      expect(band.querySelector('[data-testid="choice-space"]')).not.toBeNull();
      expect(band.querySelector('[data-testid="choice-folder"]')).not.toBeNull();
      expect(metaRow.querySelector('[data-testid="choice-space"]')).toBeNull();
      expect(metaRow.compareDocumentPosition(band) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(band.compareDocumentPosition(strip) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });
  });

  describe('fullscreen', () => {
    function panel(): HTMLElement {
      return fixture.nativeElement.querySelector('.dialog-panel');
    }

    async function openNote(): Promise<void> {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();
    }

    beforeEach(openNote);

    it('opens at the panel default size and exposes the button as a toggle', () => {
      expect(panel().classList.contains('fullscreen')).toBe(false);
      expect(fullscreenButton().getAttribute('aria-pressed')).toBe('false');
      expect(fullscreenButton().getAttribute('aria-label')).toBe('Passer en plein écran');
    });

    it('expands the panel and flips the toggle when the button is clicked', async () => {
      fullscreenButton().click();
      await fixture.whenStable();

      expect(panel().classList.contains('fullscreen')).toBe(true);
      expect(fullscreenButton().getAttribute('aria-pressed')).toBe('true');
      expect(fullscreenButton().getAttribute('aria-label')).toBe('Quitter le plein écran');
    });

    it('returns to the default size on a second click', async () => {
      fullscreenButton().click();
      await fixture.whenStable();
      fullscreenButton().click();
      await fixture.whenStable();

      expect(panel().classList.contains('fullscreen')).toBe(false);
    });

    it('keeps the choice while another note is opened', async () => {
      fullscreenButton().click();
      await fixture.whenStable();

      fixture.componentRef.setInput('note', createNote({ id: 'other' }));
      await fixture.whenStable();

      expect(panel().classList.contains('fullscreen')).toBe(true);
    });

    it('persists the choice so it survives a restart', async () => {
      fullscreenButton().click();
      await fixture.whenStable();

      expect(preferences().read('devnotes.editorFullscreen')).toBe('true');
    });

    it('reopens fullscreen when the preference was stored in a previous session', async () => {
      preferences().write('devnotes.editorFullscreen', 'true');
      recreateOverlay();
      await openNote();

      expect(panel().classList.contains('fullscreen')).toBe(true);
    });

    it('ignores an invalid persisted value', async () => {
      preferences().write('devnotes.editorFullscreen', 'oui');
      recreateOverlay();
      await openNote();

      expect(panel().classList.contains('fullscreen')).toBe(false);
    });
  });
  describe('placeholder fields', () => {
    const FIELDS = [
      { name: 'host', defaultValue: '', value: 'db.internal' },
      { name: 'port', defaultValue: '5432', value: '' },
    ];

    async function openTemplated(): Promise<void> {
      fixture.componentRef.setInput(
        'note',
        createNote({ content: 'psql -h {{host}} -p {{port}}', placeholders: FIELDS }),
      );
      await fixture.whenStable();
    }

    function panel(): HTMLElement | null {
      return fixture.nativeElement.querySelector('.panel');
    }

    function panelAction(label: string): HTMLButtonElement | undefined {
      return [...fixture.nativeElement.querySelectorAll('.panel-action')].find((button) =>
        (button as HTMLElement).textContent?.includes(label),
      ) as HTMLButtonElement | undefined;
    }

    function fieldInputs(): HTMLInputElement[] {
      return [...fixture.nativeElement.querySelectorAll('.field-input')];
    }

    it('shows no panel for a note without a single token', async () => {
      fixture.componentRef.setInput('note', createNote());
      await fixture.whenStable();

      expect(panel()).toBeNull();
    });

    it('opens the panel on the values the note already carries', async () => {
      await openTemplated();

      expect(panel()).not.toBeNull();
      expect(fieldInputs().map((input) => input.value)).toEqual(['db.internal', '']);
    });

    it('emits the values to save when a field is left', async () => {
      await openTemplated();

      await type(fieldInputs()[1], '6543');
      fixture.nativeElement
        .querySelector('.panel-body')
        .dispatchEvent(new Event('focusout', { bubbles: true }));
      await fixture.whenStable();

      expect(asked.values.at(-1)).toEqual({ host: 'db.internal', port: '6543' });
    });

    it('confirms the pending values before closing', async () => {
      await openTemplated();

      await type(fieldInputs()[1], '6543');
      toolbarButton('.close-btn').click();
      await fixture.whenStable();

      expect(asked.values.at(-1)).toEqual({ host: 'db.internal', port: '6543' });
    });

    it('folds the panel and remembers it for the next session', async () => {
      await openTemplated();

      fixture.nativeElement.querySelector('.panel-toggle').click();
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('.panel-body')).toBeNull();
      expect(preferences().read('devnotes.editorFieldsPanel')).toBe('false');
    });

    it('copies filled rather than raw, and composes the text at the click', async () => {
      await openTemplated();
      const copyFilled = vi
        .spyOn(TestBed.inject(PlaceholderFillStore), 'copyFilled')
        .mockResolvedValue(undefined);

      await type(fieldInputs()[1], '6543');
      await type(bodyEditor(), 'psql -h {{host}} -p {{port}} -d app');
      const copyButton = [...fixture.nativeElement.querySelectorAll('.toolbar-btn')].find((button) =>
        (button as HTMLElement).textContent?.includes('Copier rempli'),
      ) as HTMLButtonElement;
      copyButton.click();
      await fixture.whenStable();

      expect(copyFilled.mock.calls.map(([request]) => request)).toEqual([
        {
          content: 'psql -h {{host}} -p {{port}} -d app',
          values: { host: 'db.internal', port: '6543' },
        },
      ]);
    });

    it('asks for a preview only once it is turned on, then at every keystroke', async () => {
      await openTemplated();
      const refresh = vi
        .spyOn(TestBed.inject(PlaceholderFillStore), 'refreshPreview')
        .mockResolvedValue(undefined);

      await type(fieldInputs()[1], '5');
      expect(refresh).not.toHaveBeenCalled();

      panelAction('Aperçu')?.click();
      await fixture.whenStable();
      await type(fieldInputs()[1], '54');

      expect(refresh.mock.calls.map(([request]) => request.values['port'])).toEqual(['5', '54']);
    });

    it('shows the filled body in place of the editable one while previewing', async () => {
      await openTemplated();
      // Typed rather than left to the field's default, which is the back end's job.
      await type(fieldInputs()[1], '5432');

      panelAction('Aperçu')?.click();
      await vi.waitFor(() => expect(bodyEditor()).toBeNull());

      expect(codeViewer().content()).toBe('psql -h db.internal -p 5432');
    });

    it('leaves the body editable while the filled text has not come back', async () => {
      await openTemplated();
      // Never resolves: the point is the window between asking and being answered.
      vi.spyOn(TestBed.inject(PlaceholderFillStore), 'refreshPreview').mockReturnValue(
        new Promise(() => undefined),
      );

      panelAction('Aperçu')?.click();
      await fixture.whenStable();

      expect(bodyEditor()).not.toBeNull();
    });

    it('closes the preview when the panel is folded', async () => {
      await openTemplated();
      panelAction('Aperçu')?.click();
      await vi.waitFor(() => expect(bodyEditor()).toBeNull());

      fixture.nativeElement.querySelector('.panel-toggle').click();
      await fixture.whenStable();

      expect(bodyEditor()).not.toBeNull();
    });
  });

  describe('a draft that becomes a note', () => {
    /**
     * Committing the title materialises the draft, which changes the note's id and
     * hands the editor back a note whose content is not written yet. Drafts keyed on
     * that id are replayed over the body; they key on the session instead.
     */
    it('keeps the body typed while the note was still a draft', async () => {
      fixture.componentRef.setInput('session', 1);
      fixture.componentRef.setInput('note', createNote({ id: 'draft', title: '', content: '' }));
      await fixture.whenStable();

      const body = fixture.nativeElement.querySelector('[data-testid="editor-body"]');
      await type(body, 'openssl req -new');

      // The materialisation lands: a real id, the committed title, and no content yet.
      fixture.componentRef.setInput(
        'note',
        createNote({ id: 'note-42', title: 'Rotate the certificate', content: '' }),
      );
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="editor-body"]').value).toBe(
        'openssl req -new',
      );
    });

    it('still starts from the new note when a different one is opened', async () => {
      fixture.componentRef.setInput('session', 1);
      fixture.componentRef.setInput('note', createNote({ id: 'a', content: 'first' }));
      await fixture.whenStable();

      fixture.componentRef.setInput('session', 2);
      fixture.componentRef.setInput('note', createNote({ id: 'b', content: 'second' }));
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="editor-body"]').value).toBe('second');
    });
  });
});
