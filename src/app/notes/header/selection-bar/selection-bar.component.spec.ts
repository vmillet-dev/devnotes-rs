import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SelectionBarComponent } from './selection-bar.component';

const SPACES = [
  { id: 'space-1', name: 'Perso' },
  { id: 'space-2', name: 'Boulot' },
];

const FOLDERS = [
  { id: 'perf', spaceId: 'space-1', name: 'Perf', colour: 'amber' as const, createdAt: new Date(0) },
  { id: 'migr', spaceId: 'space-1', name: 'Migrations', colour: 'blue' as const, createdAt: new Date(0) },
];

describe('SelectionBarComponent', () => {
  let fixture: ComponentFixture<SelectionBarComponent>;

  function actions(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.selection-action')];
  }

  function deleteButton(): HTMLButtonElement {
    return actions()[1];
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SelectionBarComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(SelectionBarComponent);
    fixture.componentRef.setInput('count', 3);
    fixture.componentRef.setInput('spaces', SPACES);
    fixture.autoDetectChanges();
  });

  it('announces how many notes are selected', () => {
    expect(fixture.nativeElement.querySelector('.selection-count').textContent).toContain('3');
  });

  /**
   * ⚠️ Menus, not fields. Both are **commands** — they reset their own value after every
   * change — so a `<select>` announced a combobox whose current value was "Ranger dans".
   */
  function entries(kind: string): string[] {
    return [...fixture.nativeElement.querySelectorAll(`[data-testid="choice-panel-${kind}"] button`)].map(
      (option) => (option as HTMLElement).textContent?.trim() ?? '',
    );
  }

  async function openMenu(kind: string): Promise<void> {
    fixture.nativeElement.querySelector(`[data-testid="choice-${kind}"]`).click();
    await fixture.whenStable();
  }

  async function pick(kind: string, optionId: string): Promise<void> {
    await openMenu(kind);
    fixture.nativeElement
      .querySelector(`[data-testid="choice-panel-${kind}"] [data-option-id="${optionId}"]`)
      .click();
    await fixture.whenStable();
  }

  it('offers every space as a move target', async () => {
    await openMenu('selection-move');

    expect(entries('selection-move')).toEqual(['Perso', 'Boulot']);
  });

  /** It names what it does, never where the selection is: it has no current value at all. */
  it('names the command on the trigger', () => {
    const trigger = fixture.nativeElement.querySelector('[data-testid="choice-selection-move"]');

    expect(trigger.textContent).toContain('Déplacer vers');
  });

  it('emits the chosen space', async () => {
    let emitted: string | undefined;
    fixture.componentInstance.moveRequested.subscribe((id) => (emitted = id));

    await pick('selection-move', 'space-2');

    expect(emitted).toBe('space-2');
  });

  it('hides the move picker when there is nowhere to move to', async () => {
    fixture.componentRef.setInput('spaces', []);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('[data-testid="choice-selection-move"]')).toBeNull();
  });

  describe('filing into a folder', () => {
    beforeEach(async () => {
      fixture.componentRef.setInput('folders', FOLDERS);
      await fixture.whenStable();
    });

    /** ⚠️ Both directions are one control: unfiling is as much a filing as any other. */
    it('offers every folder and a way back out of one', async () => {
      await openMenu('selection-file');

      expect(entries('selection-file')).toEqual(['Perf', 'Migrations', 'Sortir du dossier']);
    });

    it('emits the chosen folder', async () => {
      let emitted: string | null | undefined;
      fixture.componentInstance.fileRequested.subscribe((id) => (emitted = id));

      await pick('selection-file', 'migr');

      expect(emitted).toBe('migr');
    });

    it('emits null to take the selection out of its folder', async () => {
      let emitted: string | null | undefined = 'untouched';
      fixture.componentInstance.fileRequested.subscribe((id) => (emitted = id));

      await pick('selection-file', '__unfile__');

      expect(emitted).toBeNull();
    });

    it('hides the picker when the space holds no folder', async () => {
      fixture.componentRef.setInput('folders', []);
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="choice-selection-file"]')).toBeNull();
    });
  });

  it('emits the typed tag and clears the field', async () => {
    let emitted: string | undefined;
    fixture.componentInstance.tagRequested.subscribe((tag) => (emitted = tag));
    const input: HTMLInputElement = fixture.nativeElement.querySelector('.selection-tag-input');
    input.value = '  urgent ';
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    fixture.debugElement
      .query(By.css('.selection-tag-form'))
      .triggerEventHandler('submit', new Event('submit'));
    await fixture.whenStable();

    expect(emitted).toBe('urgent');
    expect(input.value).toBe('');
  });

  it('does not emit a blank tag', async () => {
    let emitted = 0;
    fixture.componentInstance.tagRequested.subscribe(() => (emitted += 1));

    fixture.debugElement
      .query(By.css('.selection-tag-form'))
      .triggerEventHandler('submit', new Event('submit'));
    await fixture.whenStable();

    expect(emitted).toBe(0);
  });

  it('asks for a confirmation before trashing a whole selection', async () => {
    let emitted = 0;
    fixture.componentInstance.deleteRequested.subscribe(() => (emitted += 1));

    deleteButton().click();
    await fixture.whenStable();

    expect(emitted).toBe(0);
    expect(deleteButton().textContent).toContain('Confirmer');

    deleteButton().click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });
});
