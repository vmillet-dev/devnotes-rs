import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { PassphrasePromptStore } from '@core/state/passphrase-prompt.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { SpacesStore } from '@core/state/spaces.store';
import { Space } from '@core/model/space.model';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeTransferRepository } from '@testing/fake-transfer-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { FileMenuComponent } from './file-menu.component';

const SPACES: readonly Space[] = [
  { id: 'space-1', name: 'Space one', pinned: false },
  { id: 'work', name: 'Work', pinned: false },
];

describe('FileMenuComponent', () => {
  let fixture: ComponentFixture<FileMenuComponent>;
  let appWindow: FakeAppWindow;
  let transferRepository: FakeTransferRepository;
  let fileDialog: FakeFileDialog;
  let clipboard: FakeClipboard;
  let selection: NoteSelectionStore;
  let spaces: SpacesStore;

  function trigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.file-trigger');
  }

  function options(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.file-option')];
  }

  function optionLabelled(label: string): HTMLButtonElement {
    const found = options().find((option) => option.textContent?.includes(label));
    if (!found) throw new Error(`No menu option labelled "${label}"`);
    return found;
  }

  /** ⚠️ The prompt stands between the click and the file; these scenarios decline it. */
  async function declineProtection(): Promise<void> {
    const prompt = TestBed.inject(PassphrasePromptStore);
    await vi.waitFor(() => expect(prompt.request()).not.toBeNull());
    prompt.answer({ kind: 'none' });
  }

  async function openMenu(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  /** The menu acts on the selection, which is derived from what the canvas shows. */
  async function check(id: string): Promise<void> {
    await vi.waitFor(() => expect(TestBed.inject(NotesQueryStore).visibleNotes().length).toBeGreaterThan(0));
    selection.toggleChecked(id);
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    appWindow = new FakeAppWindow();
    transferRepository = new FakeTransferRepository();
    fileDialog = new FakeFileDialog();
    clipboard = new FakeClipboard();

    TestBed.configureTestingModule({
      imports: [FileMenuComponent],
      providers: [
        provideAppTesting({
          notes: [createNote({ id: 'note-42' })],
          spaces: SPACES,
          transferRepository,
          fileDialog,
          clipboard,
          appWindow,
        }),
      ],
    });
    selection = TestBed.inject(NoteSelectionStore);
    spaces = TestBed.inject(SpacesStore);
    fixture = TestBed.createComponent(FileMenuComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('keeps its panel closed until asked', () => {
    expect(options()).toHaveLength(0);
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
  });

  it('lists what it can do, in the order it declares it', async () => {
    await openMenu();

    expect(options().map((option) => option.textContent?.trim())).toEqual([
      'Importer…',
      'Exporter tout…',
      "Exporter l'espace actif…",
      'Exporter la sélection…',
      'Copier la sélection en Markdown',
      'Bibliothèques…',
      'Préférences…',
      'Quitter DevNotes',
    ]);
  });

  it('opens the preferences panel, closing the menu behind it', async () => {
    await openMenu();

    optionLabelled('Préférences').click();
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('app-settings-dialog')).not.toBeNull();
    expect(options()).toHaveLength(0);
  });

  it('imports a bundle and closes, the report showing elsewhere', async () => {
    fileDialog.openPath = 'C:\\bundles\\devnotes-2026-01-01.json';
    await openMenu();

    optionLabelled('Importer').click();
    await fixture.whenStable();

    await vi.waitFor(() => expect(transferRepository.importedFrom).toBe(fileDialog.openPath));
    expect(options()).toHaveLength(0);
  });

  it('exports the whole corpus', async () => {
    fileDialog.savePath = 'C:\\out\\all.json';
    await openMenu();

    optionLabelled('Exporter tout').click();
    await declineProtection();

    await vi.waitFor(() =>
      expect(transferRepository.exportedTo).toEqual({
        path: 'C:\\out\\all.json',
        spaceId: null,
        passphrase: null,
      }),
    );
  });

  it('keeps "export this space" unavailable until a space is active', async () => {
    fileDialog.savePath = 'C:\\out\\space.json';
    await openMenu();
    expect(optionLabelled("Exporter l'espace").getAttribute('aria-disabled')).toBe('true');

    optionLabelled("Exporter l'espace").click();
    await fixture.whenStable();
    expect(transferRepository.exportedTo).toBeNull();

    spaces.selectSpace('work');
    await fixture.whenStable();
    optionLabelled("Exporter l'espace").click();
    await declineProtection();

    await vi.waitFor(() => expect(transferRepository.exportedTo?.spaceId).toBe('work'));
  });

  it('keeps the selection entries unavailable until notes are checked', async () => {
    await openMenu();

    expect(optionLabelled('Exporter la sélection').getAttribute('aria-disabled')).toBe('true');
    expect(optionLabelled('Copier la sélection').getAttribute('aria-disabled')).toBe('true');

    await check('note-42');

    expect(optionLabelled('Exporter la sélection').getAttribute('aria-disabled')).toBe('false');
    expect(optionLabelled('Copier la sélection').getAttribute('aria-disabled')).toBe('false');
  });

  it('exports exactly the checked notes', async () => {
    fileDialog.savePath = 'C:\\out\\selection.json';
    await check('note-42');
    await openMenu();

    optionLabelled('Exporter la sélection').click();
    await declineProtection();

    await vi.waitFor(() => expect(transferRepository.exportedIds).toEqual(['note-42']));
  });

  it('copies the checked notes as markdown rather than sending them anywhere', async () => {
    await check('note-42');
    await openMenu();

    optionLabelled('Copier la sélection').click();

    await vi.waitFor(() => expect(clipboard.content).toBe(transferRepository.markdown));
    expect(transferRepository.sharedIds).toEqual(['note-42']);
    expect(TestBed.inject(StatusNotifier).status()?.key).toBe('file.copied');
  });

  it('quits only on a second click', async () => {
    await openMenu();

    optionLabelled('Quitter').click();
    await fixture.whenStable();

    expect(appWindow.exitedWith).toBeNull();
    expect(optionLabelled('Confirmer')).toBeTruthy();

    optionLabelled('Confirmer').click();
    await fixture.whenStable();

    expect(appWindow.exitedWith).toBe(0);
  });

  it('forgets a pending quit confirmation when the menu closes', async () => {
    await openMenu();
    optionLabelled('Quitter').click();
    await fixture.whenStable();

    trigger().click();
    await fixture.whenStable();
    await openMenu();

    expect(optionLabelled('Quitter').textContent).not.toContain('Confirmer');
  });
});
