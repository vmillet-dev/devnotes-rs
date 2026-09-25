import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ChoiceMenuComponent, ChoiceOption } from './choice-menu.component';

const FOLDERS: readonly ChoiceOption[] = [
  { id: 'perf', name: 'Perf', colour: 'amber' },
  { id: 'migrations', name: 'Migrations', colour: 'blue' },
];

describe('ChoiceMenuComponent', () => {
  let fixture: ComponentFixture<ChoiceMenuComponent>;

  function trigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.choice-trigger');
  }

  function options(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('[data-testid="choice-option"]')];
  }

  async function open(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [ChoiceMenuComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(ChoiceMenuComponent);
    fixture.componentRef.setInput('kind', 'folder');
    fixture.componentRef.setInput('label', 'Ranger dans');
    fixture.componentRef.setInput('options', FOLDERS);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('names where the note is, and falls back to the way out when it is nowhere', async () => {
    fixture.componentRef.setInput('noneLabel', 'Aucun dossier');
    await fixture.whenStable();
    expect(trigger().textContent).toContain('Aucun dossier');

    fixture.componentRef.setInput('currentId', 'perf');
    await fixture.whenStable();

    expect(trigger().textContent).toContain('Perf');
  });

  it('translates an option named by a key, and follows a change of language', async () => {
    fixture.componentRef.setInput('options', [
      ...FOLDERS,
      { id: 'out', name: 'selection.unfile', nameIsKey: true },
    ]);
    fixture.componentRef.setInput('currentId', 'out');
    await open();
    expect(trigger().querySelector('.choice-name')?.textContent?.trim()).toBe('Sortir du dossier');
    expect(options()[2].textContent?.trim()).toBe('Sortir du dossier');

    TestBed.inject(TranslocoService).setActiveLang('en');
    await fixture.whenStable();

    expect(trigger().querySelector('.choice-name')?.textContent?.trim()).toBe('Take out of folder');
    expect(options()[2].textContent?.trim()).toBe('Take out of folder');
    expect(options()[0].textContent?.trim()).toBe('Perf');
  });

  it('emits the option it was asked for and folds away', async () => {
    const chosen: (string | null)[] = [];
    fixture.componentInstance.chosen.subscribe((id: string | null) => chosen.push(id));
    await open();

    options()[1].click();
    await fixture.whenStable();

    expect(chosen).toEqual(['migrations']);
    expect(options()).toHaveLength(0);
  });

  /** A menu is a command: asking for where the note already is is asking for nothing. */
  it('says nothing when the note is already there', async () => {
    fixture.componentRef.setInput('currentId', 'perf');
    const chosen: (string | null)[] = [];
    fixture.componentInstance.chosen.subscribe((id: string | null) => chosen.push(id));
    await open();

    options()[0].click();
    await fixture.whenStable();

    expect(chosen).toEqual([]);
  });

  it('offers the way out only where there is one', async () => {
    fixture.componentRef.setInput('currentId', 'perf');
    await open();
    expect(fixture.nativeElement.querySelector('[data-testid="choice-none"]')).toBeNull();

    fixture.componentRef.setInput('noneLabel', 'Aucun dossier');
    await fixture.whenStable();

    const chosen: (string | null)[] = [];
    fixture.componentInstance.chosen.subscribe((id: string | null) => chosen.push(id));
    fixture.nativeElement.querySelector('[data-testid="choice-none"]').click();
    await fixture.whenStable();

    expect(chosen).toEqual([null]);
  });

  /**
   * This menu lives inside a dialog, unlike every other one in the application. The
   * trigger directive lets Escape bubble on purpose, and the next listener up is the
   * editor's own — so one Escape closed the note along with the menu.
   */
  it('keeps Escape to itself while it is open', async () => {
    // What the editor's own listener would see, one level up.
    const reachedTheDialog: string[] = [];
    const parent = fixture.nativeElement.parentElement as HTMLElement;
    const listen = (event: Event) => reachedTheDialog.push((event as KeyboardEvent).key);
    parent.addEventListener('keydown', listen);

    await open();
    fixture.nativeElement.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
    );
    await fixture.whenStable();

    expect(reachedTheDialog).toEqual([]);
    expect(options()).toHaveLength(0);

    // Closed, so the editor gets the next one: Escape is how a note is put away.
    fixture.nativeElement.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
    );
    parent.removeEventListener('keydown', listen);

    expect(reachedTheDialog).toEqual(['Escape']);
  });

  /**
   * `naming: 'label'` is what tells a **command** from a field. The selection bar's two
   * controls reset their own value after every `change`, so a screen reader announced a
   * combobox whose current value was "Ranger dans".
   */
  describe('as a command rather than a field', () => {
    beforeEach(async () => {
      fixture.componentRef.setInput('naming', 'label');
      fixture.componentRef.setInput('currentId', 'perf');
      await fixture.whenStable();
    });

    it('names what it does, never what is chosen', () => {
      expect(trigger().textContent).toContain('Ranger dans');
      expect(trigger().textContent).not.toContain('Perf');
    });

    it('asks again for the option it is already on, which a field would not', async () => {
      const chosen: (string | null)[] = [];
      fixture.componentInstance.chosen.subscribe((id: string | null) => chosen.push(id));
      await open();

      options()[0].click();
      await fixture.whenStable();

      expect(chosen).toEqual(['perf']);
    });

    it('marks nothing as current, having none', async () => {
      await open();

      expect(options().some((option) => option.classList.contains('on'))).toBe(false);
    });

    /** Nothing to leave: a command's own entries are the only answers it takes. */
    it('offers no way out even when one is named', async () => {
      fixture.componentRef.setInput('noneLabel', 'Aucun dossier');
      await open();

      expect(fixture.nativeElement.querySelector('[data-testid="choice-none"]')).toBeNull();
    });
  });
});
