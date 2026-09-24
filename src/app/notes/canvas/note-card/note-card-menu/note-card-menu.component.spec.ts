import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { NoteCardMenuComponent } from './note-card-menu.component';

const SPACES: readonly Space[] = [
  { id: 'work', name: 'Work', pinned: false },
  { id: 'personal', name: 'Personal', pinned: false },
  { id: 'archive', name: 'Archive', pinned: false },
];

const FOLDERS: readonly Folder[] = [
  { id: 'perf', spaceId: 'work', name: 'Perf', colour: 'amber', createdAt: new Date('2026-01-01') },
  {
    id: 'migrations',
    spaceId: 'work',
    name: 'Migrations',
    colour: 'blue',
    createdAt: new Date('2026-01-02'),
  },
];

describe('NoteCardMenuComponent', () => {
  let fixture: ComponentFixture<NoteCardMenuComponent>;

  function trigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.card-menu-trigger');
  }

  function items(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.card-menu-item')];
  }

  function moveItems(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('[data-testid="note-card-move"]')];
  }

  function fileItems(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('[data-testid="note-card-file"]')];
  }

  function deleteItem(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.card-menu-delete');
  }

  async function open(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  async function pressKey(key: string): Promise<void> {
    (document.activeElement ?? document.body).dispatchEvent(
      new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }),
    );
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [NoteCardMenuComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(NoteCardMenuComponent);
    fixture.componentRef.setInput('noteTitle', 'Ma note');
    fixture.componentRef.setInput('spaces', SPACES);
    fixture.componentRef.setInput('currentSpaceId', 'work');
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('stays closed until the trigger is used', () => {
    expect(items()).toHaveLength(0);
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
  });

  it('names the note in the trigger label, since ⋯ says nothing on its own', () => {
    expect(trigger().getAttribute('aria-label')).toBe('Options de la note Ma note');
  });

  it('offers every space except the one the note is already in', async () => {
    await open();

    expect(moveItems().map((item) => item.textContent?.trim())).toEqual(['Personal', 'Archive']);
  });

  it('emits the destination space and closes', async () => {
    const moves: string[] = [];
    fixture.componentInstance.moveRequested.subscribe((id) => moves.push(id));
    await open();

    moveItems()[1].click();
    await fixture.whenStable();

    expect(moves).toEqual(['archive']);
    expect(items()).toHaveLength(0);
  });

  it('hides the move group entirely when there is nowhere to move to', async () => {
    fixture.componentRef.setInput('spaces', [SPACES[0]]);
    await fixture.whenStable();

    await open();

    expect(moveItems()).toHaveLength(0);
    expect(fixture.nativeElement.querySelector('.card-menu-title')).toBeNull();
    expect(deleteItem()).not.toBeNull();
  });

  /** The complete menu: every property of a note, from its card. */
  describe('the entries that used to be somewhere else', () => {
    it('opens, pins and copies from here too', async () => {
      const fired: string[] = [];
      fixture.componentInstance.opened.subscribe(() => fired.push('open'));
      fixture.componentInstance.pinToggled.subscribe(() => fired.push('pin'));
      fixture.componentInstance.copyRequested.subscribe(() => fired.push('copy'));
      await open();

      for (const id of ['note-card-open', 'note-card-pin', 'note-card-copy']) {
        fixture.nativeElement.querySelector(`[data-testid="${id}"]`).click();
        await fixture.whenStable();
        await open();
      }

      expect(fired).toEqual(['open', 'pin', 'copy']);
    });

    it('names the pin by what pressing it would do', async () => {
      fixture.componentRef.setInput('pinned', true);
      await open();

      expect(fixture.nativeElement.querySelector('[data-testid="note-card-pin"]').textContent).toContain(
        'Épinglée',
      );
    });

    it('offers every folder of the space except the one it is in', async () => {
      fixture.componentRef.setInput('folders', FOLDERS);
      fixture.componentRef.setInput('currentFolderId', 'perf');
      await open();

      expect(fileItems().map((item) => item.textContent?.trim())).toEqual(['Migrations']);
    });

    /** Filing is a batch command of its own, so `null` is a real answer, not an absence. */
    it('offers a way out of the folder only when there is one to leave', async () => {
      fixture.componentRef.setInput('folders', FOLDERS);
      await open();
      expect(fixture.nativeElement.querySelector('[data-testid="note-card-unfile"]')).toBeNull();

      await pressKey('Escape');
      fixture.componentRef.setInput('currentFolderId', 'perf');
      await open();

      const filings: (string | null)[] = [];
      fixture.componentInstance.fileRequested.subscribe((id) => filings.push(id));
      fixture.nativeElement.querySelector('[data-testid="note-card-unfile"]').click();
      await fixture.whenStable();

      expect(filings).toEqual([null]);
    });

    it('emits the folder it was asked to file into, and closes', async () => {
      fixture.componentRef.setInput('folders', FOLDERS);
      const filings: (string | null)[] = [];
      fixture.componentInstance.fileRequested.subscribe((id) => filings.push(id));
      await open();

      fileItems()[0].click();
      await fixture.whenStable();

      expect(filings).toEqual(['perf']);
      expect(items()).toHaveLength(0);
    });

    it('draws no filing group at all for a space with no folder', async () => {
      await open();

      expect(fileItems()).toHaveLength(0);
    });
  });

  it('asks for confirmation before emitting a deletion', async () => {
    const deletions: unknown[] = [];
    fixture.componentInstance.deleteRequested.subscribe(() => deletions.push(true));
    await open();

    deleteItem().click();
    await fixture.whenStable();

    expect(deletions).toEqual([]);
    expect(deleteItem().textContent).toContain('Confirmer ?');

    deleteItem().click();
    await fixture.whenStable();

    expect(deletions).toHaveLength(1);
  });

  it('forgets a pending confirmation when the menu is reopened', async () => {
    await open();
    deleteItem().click();
    await fixture.whenStable();

    trigger().click();
    await fixture.whenStable();
    await open();

    expect(deleteItem().textContent).toContain('Supprimer');
  });

  it('does not open the note when the trigger is clicked', async () => {
    let bubbled = false;
    fixture.nativeElement.parentElement?.addEventListener('click', () => (bubbled = true), {
      once: true,
    });

    await open();

    expect(bubbled).toBe(false);
  });

  it('moves focus into the menu when it opens', async () => {
    await open();

    expect(document.activeElement).toBe(items()[0]);
  });

  it('cycles focus with the arrow keys', async () => {
    await open();

    await pressKey('ArrowDown');
    expect(document.activeElement).toBe(items()[1]);

    await pressKey('End');
    expect(document.activeElement).toBe(items()[items().length - 1]);

    await pressKey('ArrowDown');
    expect(document.activeElement).toBe(items()[0]);
  });

  it('closes on Escape and hands focus back to the trigger', async () => {
    await open();

    await pressKey('Escape');

    expect(items()).toHaveLength(0);
    expect(document.activeElement).toBe(trigger());
  });

  it('closes when a click lands outside', async () => {
    await open();

    document.body.click();
    await fixture.whenStable();

    expect(items()).toHaveLength(0);
  });
});
