import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EVENT_SUBSCRIBER, EventSubscriber, GlobalAction } from '@core/ipc/app-events.service';
import { RequestHit } from '@core/model/http.model';
import { AreaStore } from '@core/services/areas/area.store';
import { HelpStore } from '@core/services/help/help.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { PreferencesDialogStore } from '@core/services/settings/preferences-dialog.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FoldersStore } from '@core/state/folders.store';
import { NotesStore } from '@core/state/notes.store';
import { PaletteStore } from '@core/state/palette.store';
import { SpacesStore } from '@core/state/spaces.store';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { PaletteComponent } from './palette.component';

const LOGIN: RequestHit = {
  id: 'request-1',
  name: 'Login',
  kind: 'http',
  method: 'POST',
  url: '{{baseUrl}}/auth/login',
  place: ['API Paiements', 'Auth'],
};

const NOTES = [
  createNote({
    id: 'note-1',
    spaceId: 'space-1',
    title: 'Payload JSON',
    content: '{"event":"paid"}',
    language: 'json',
    tags: ['json'],
  }),
  createNote({
    id: 'note-2',
    spaceId: 'space-2',
    title: 'Connexion psql',
    content: 'psql -h {{host}}',
    language: 'sh',
    placeholders: [{ name: 'host', defaultValue: '', value: '' }],
  }),
];

describe('PaletteComponent', () => {
  let fixture: ComponentFixture<PaletteComponent>;
  let fire: (action: GlobalAction) => void;
  let notes: FakeNotesRepository;
  let http: FakeHttpRepository;
  let clipboard: FakeClipboard;
  let appWindow: FakeAppWindow;
  let store: PaletteStore;

  const input = (): HTMLInputElement => fixture.nativeElement.querySelector('[data-testid="palette-input"]');
  const rows = (): HTMLElement[] => [
    ...fixture.nativeElement.querySelectorAll('[data-testid="palette-option"]'),
  ];
  const kinds = (): string[] => rows().map((row) => row.dataset['kind'] ?? '');
  const headings = (): string[] =>
    [...fixture.nativeElement.querySelectorAll('[data-testid="palette-group"]')].map(
      (group: HTMLElement) => group.textContent?.replace(/\s+/g, ' ').trim() ?? '',
    );
  const hint = (): string =>
    fixture.nativeElement.querySelector('[data-testid="palette-hint"]').textContent.replace(/\s+/g, ' ');

  async function press(key: string, options: KeyboardEventInit = {}): Promise<void> {
    input().dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...options }));
    await fixture.whenStable();
  }

  /** Typed as a user would, then past the debounce for Rust to answer. */
  async function type(text: string): Promise<void> {
    input().value = text;
    input().dispatchEvent(new Event('input'));
    await vi.waitFor(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
    });
    await fixture.whenStable();
  }

  async function open(action: GlobalAction = 'palette'): Promise<void> {
    fire(action);
    await vi.waitFor(() => expect(input()).not.toBeNull());
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    notes = new FakeNotesRepository(NOTES);
    http = new FakeHttpRepository();
    http.hits = [LOGIN];
    clipboard = new FakeClipboard();
    appWindow = new FakeAppWindow();
    let handler: ((action: GlobalAction) => void) | null = null;
    const subscribe: EventSubscriber = async (next) => {
      handler = next;
      return () => (handler = null);
    };
    fire = (action) => handler?.(action);

    TestBed.configureTestingModule({
      imports: [PaletteComponent],
      providers: [
        provideAppTesting({
          notesRepository: notes,
          httpRepository: http,
          clipboard,
          appWindow,
          spaces: [
            { id: 'space-1', name: 'Travail', pinned: false },
            { id: 'space-2', name: 'Recette', pinned: false },
          ],
        }),
        { provide: EVENT_SUBSCRIBER, useValue: subscribe },
      ],
    });
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    TestBed.inject(SettingsStore).recentTools.write([{ id: 'convert', at: '2026-10-16T10:00:00Z' }]);
    store = TestBed.inject(PaletteStore);
    fixture = TestBed.createComponent(PaletteComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('opens on the native action, and knows whether it came from another application', async () => {
    await open('palette');
    expect(store.fromElsewhere()).toBe(true);
    expect(hint()).toContain('copier et revenir');

    store.close();
    await open('palette-here');
    expect(store.fromElsewhere()).toBe(false);
    expect(hint()).not.toContain('revenir');
  });

  it('opens on the recent things of every kind, and names the prefixes', async () => {
    await open();

    expect(headings()).toEqual(['Récents']);
    expect(kinds()).toEqual(['note', 'note', 'request', 'tool']);
    expect(fixture.nativeElement.querySelector('[data-testid="palette-prefixes"]').textContent).toContain(
      'choisir où chercher',
    );
  });

  it('groups what answers by kind, each with its mark and its count', async () => {
    await open();
    await type('json');

    expect(headings()).toEqual(['Notes 2', 'Requêtes 1', 'Outils 3']);
    const [note] = rows();
    expect(note.querySelector('app-kind-badge')).not.toBeNull();
    const request = rows().find((row) => row.dataset['kind'] === 'request')!;
    expect(request.querySelector('.method')?.getAttribute('data-method')).toBe('POST');
    expect(request.querySelector('.variable')?.textContent).toBe('{{baseUrl}}');
    const tool = rows().find((row) => row.dataset['kind'] === 'tool')!;
    expect(tool.querySelector('.tile')?.getAttribute('data-category')).toBeTruthy();
  });

  it('turns @ into a chip with Tab, and searches that kind alone', async () => {
    await open();
    await type('json @out');
    expect(kinds()).toEqual(['chip']);

    await press('Tab');
    await type('json');

    expect(fixture.nativeElement.querySelector('[data-testid="palette-chip"]').textContent).toContain(
      '@outils',
    );
    expect(new Set(kinds())).toEqual(new Set(['tool']));
    expect(headings()).toEqual([]);
  });

  it('narrows to one space’s notes from @, and drops the chip on ⌫ in an empty field', async () => {
    await open();
    await type('@rec');
    expect(rows()[0]?.textContent).toContain('Recette');

    await press('Enter');
    expect(store.chips()).toEqual([{ kind: 'space', id: 'space-2', name: 'Recette' }]);
    await vi.waitFor(() => expect(notes.lastQuery?.spaceId).toBe('space-2'));

    await press('Backspace');
    expect(store.chips()).toEqual([]);
  });

  it('offers the tags after #, with how many notes carry each', async () => {
    vi.spyOn(notes, 'loadTags').mockResolvedValue([
      { tag: 'json', noteCount: 12 },
      { tag: 'jwt', noteCount: 1 },
    ]);
    await open();
    await type('#j');

    // The highlighted row says what Enter does in place of its count.
    expect(rows()[0]?.textContent).toContain('Tab choisir');
    expect(rows()[1]?.textContent).toContain('1 note');
  });

  it('lists the commands after >, with their keys, and runs one on Enter', async () => {
    await open();
    await type('>');
    expect(headings()[0]).toMatch(/^Commandes \d+$/);
    expect(rows()[0]?.textContent).toContain('Nouveau snippet de code');
    expect([...rows()[1]!.querySelectorAll('kbd')].map((key) => key.textContent)).toEqual([
      'Ctrl',
      'Shift',
      'N',
    ]);

    await type('> aller aux outils');
    await press('Enter');

    expect(TestBed.inject(AreaStore).current()).toBe('tools');
    expect(store.isOpen()).toBe(false);
  });

  describe('the commands', () => {
    async function run(id: string): Promise<void> {
      await open();
      await type('>');
      const row = rows().find((candidate) => candidate.dataset['command'] === id);
      row!.querySelector<HTMLButtonElement>('[data-testid="palette-open"]')!.click();
      await fixture.whenStable();
    }

    it('creates a note of the kind asked, in the notes', async () => {
      const created = vi.spyOn(TestBed.inject(NotesStore), 'createNote').mockReturnValue();
      await run('create-checklist');

      expect(created).toHaveBeenCalledWith('checklist');
      expect(TestBed.inject(AreaStore).current()).toBe('notes');
    });

    it('opens a new request in HTTP', async () => {
      const drafted = vi.spyOn(TestBed.inject(HttpTabsStore), 'newRequest');
      await run('request');

      await vi.waitFor(() => expect(drafted).toHaveBeenCalled());
      expect(TestBed.inject(AreaStore).current()).toBe('http');
    });

    it('asks a new folder’s name in the space open, and is not offered across every space', async () => {
      await open();
      await type('>');
      expect(rows().some((row) => row.dataset['command'] === 'folder')).toBe(false);
      store.close();

      TestBed.inject(SpacesStore).selectSpace('space-1');
      await run('folder');

      expect(TestBed.inject(FoldersStore).naming()).toBe(true);
    });

    it('opens the preferences', async () => {
      await run('preferences');

      expect(TestBed.inject(PreferencesDialogStore).isOpen()).toBe(true);
    });
  });

  it('opens the shortcut sheet from >', async () => {
    await open();
    await type('> feuille');
    await press('Enter');

    expect(TestBed.inject(HelpStore).panel()).toBe('shortcuts');
  });

  describe('Enter on each kind', () => {
    it('opens a note in the notes', async () => {
      const openNote = vi.spyOn(TestBed.inject(NotesStore), 'openNote').mockResolvedValue();
      TestBed.inject(AreaStore).show('http');
      await open();

      await press('Enter');

      expect(openNote).toHaveBeenCalledWith('note-1');
      expect(TestBed.inject(AreaStore).current()).toBe('notes');
    });

    it('opens a request in its tab', async () => {
      const opened = vi.spyOn(TestBed.inject(HttpTabsStore), 'open').mockResolvedValue();
      await open();
      await type('@req');
      await press('Tab');
      await type('login');

      await press('Enter');

      await vi.waitFor(() => expect(opened).toHaveBeenCalledWith('request-1'));
      expect(TestBed.inject(AreaStore).current()).toBe('http');
    });

    it('opens a tool', async () => {
      await open();
      await type('@outils');
      await press('Tab');
      await type('cron');

      await press('Enter');

      expect(TestBed.inject(ToolsStore).openId()).toBe('cron');
      expect(TestBed.inject(AreaStore).current()).toBe('tools');
    });

    it('switches to a space', async () => {
      await open();
      await type('@espaces');
      await press('Tab');
      await type('recette');

      await press('Enter');

      expect(TestBed.inject(SpacesStore).activeSpaceId()).toBe('space-2');
    });

    it('creates a note named after what was typed from the row after the results', async () => {
      const created = vi.spyOn(TestBed.inject(NotesStore), 'createTitled').mockResolvedValue();
      await open();
      await type('rotation des certificats');
      expect(fixture.nativeElement.querySelector('[data-testid="palette-create"]')).not.toBeNull();

      await press('ArrowUp');
      for (let step = 0; step < 20; step += 1) await press('ArrowDown');
      await press('Enter');

      expect(created).toHaveBeenCalledWith('rotation des certificats');
    });
  });

  describe('Ctrl+C', () => {
    it('copies the highlighted note, and puts the window away over another application', async () => {
      await open('palette');

      await press('c', { ctrlKey: true });

      await vi.waitFor(() => expect(clipboard.content).toBe('{"event":"paid"}'));
      expect(appWindow.hidden).toBe(1);
    });

    it('copies and stays with DevNotes already in front', async () => {
      await open('palette-here');

      await press('c', { ctrlKey: true });

      await vi.waitFor(() => expect(clipboard.content).toBe('{"event":"paid"}'));
      expect(appWindow.hidden).toBe(0);
    });

    it('leaves Ctrl+C to the field while text is selected in it, and copies nothing but a note', async () => {
      await open();
      await type('@req');
      await press('Tab');
      await type('login');
      input().setSelectionRange(0, 3);
      await press('c', { ctrlKey: true });
      input().setSelectionRange(5, 5);
      await press('c', { ctrlKey: true });

      expect(clipboard.content).toBe('');
    });

    it('asks for the fields of a templated snippet before copying it', async () => {
      await open();
      await press('ArrowDown');
      await press('c', { ctrlKey: true });

      await vi.waitFor(() =>
        expect(fixture.nativeElement.querySelector('[data-testid="placeholder-form"]')).not.toBeNull(),
      );
      expect(store.pendingFill()?.id).toBe('note-2');
    });
  });

  it('stops at both ends of the list, and walks it with Tab outside a chip', async () => {
    await open();
    const current = (): string | null => input().getAttribute('aria-activedescendant');

    await press('ArrowUp');
    expect(current()).toBe('palette-option-0');
    await press('Tab');
    expect(current()).toBe('palette-option-1');
    await press('Tab', { shiftKey: true });
    expect(current()).toBe('palette-option-0');
    expect(document.activeElement).toBe(input());
  });

  it('says what was looked for when nothing answers it', async () => {
    await open();
    await type('#zzz');

    expect(fixture.nativeElement.querySelector('[data-testid="palette-empty"]').textContent).toContain('zzz');
  });
});
