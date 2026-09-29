import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ShortcutsDialogComponent } from './shortcuts-dialog.component';

describe('ShortcutsDialogComponent', () => {
  let fixture: ComponentFixture<ShortcutsDialogComponent>;
  let settings: SettingsStore;

  const groupTitles = (): string[] =>
    Array.from(fixture.nativeElement.querySelectorAll('.shortcut-group-title')).map((title) =>
      (title as HTMLElement).textContent!.trim(),
    );

  /** The caps of the row whose label contains `label`, in order. */
  function keysFor(label: string): string[] {
    const labels = Array.from(fixture.nativeElement.querySelectorAll('.shortcut-label'));
    const index = labels.findIndex((entry) => (entry as HTMLElement).textContent!.includes(label));
    if (index < 0) throw new Error(`No shortcut labelled "${label}"`);

    const row = fixture.nativeElement.querySelectorAll('.shortcut-keys')[index] as HTMLElement;
    return Array.from(row.querySelectorAll('kbd')).map((key) => (key as HTMLElement).textContent!);
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ShortcutsDialogComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    fixture = TestBed.createComponent(ShortcutsDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('opens on the global shortcuts, which work with the window closed', async () => {
    expect(groupTitles()[0]).toBe('Globaux (même fenêtre fermée)');
    expect(keysFor('palette de collage rapide')).toEqual(['Ctrl', 'Alt', 'P']);
    expect(keysFor('Capturer le presse-papier')).toEqual(['Ctrl', 'Alt', 'V']);
  });

  it('shows the quick-paste key that is really bound, not the one that shipped', async () => {
    settings.setPaletteShortcut('Ctrl+Shift+K');
    await fixture.whenStable();

    expect(keysFor('palette de collage rapide')).toEqual(['Ctrl', 'Shift', 'K']);
  });

  it('lists the areas, then the groups of the notes, after the global one', async () => {
    expect(groupTitles()).toEqual([
      'Globaux (même fenêtre fermée)',
      'Zones',
      'Canevas',
      'Éditeur',
      'Collage rapide',
    ]);
    expect(keysFor('Aller aux outils')).toEqual(['Ctrl', '2']);
    expect(keysFor('champ de recherche')).toEqual(['Ctrl', 'K']);
  });

  it('draws the separator between two caps rather than writing it', async () => {
    const row = fixture.nativeElement.querySelector('.shortcut-keys') as HTMLElement;

    expect(row.querySelectorAll('kbd')).toHaveLength(3);
    expect(row.querySelectorAll('.shortcut-plus')).toHaveLength(2);
    expect(row.querySelector('.shortcut-plus')!.getAttribute('aria-hidden')).toBe('true');
  });

  it('names itself to the dialog shell by its own title', () => {
    const panel = fixture.nativeElement.querySelector('.dialog-panel') as HTMLElement;

    expect(panel.getAttribute('aria-labelledby')).toBe('shortcuts-dialog-title');
  });

  it('closes from its button', async () => {
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));

    (fixture.nativeElement.querySelector('.sheet-close') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(closed).toBe(1);
  });
});
