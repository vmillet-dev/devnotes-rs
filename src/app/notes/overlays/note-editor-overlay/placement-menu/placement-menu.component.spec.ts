import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { PlacementMenuComponent, PlacementOption } from './placement-menu.component';

const FOLDERS: readonly PlacementOption[] = [
  { id: 'perf', name: 'Perf', colour: 'amber' },
  { id: 'migrations', name: 'Migrations', colour: 'blue' },
];

describe('PlacementMenuComponent', () => {
  let fixture: ComponentFixture<PlacementMenuComponent>;

  function trigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.placement-trigger');
  }

  function options(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('[data-testid="editor-placement-option"]')];
  }

  async function open(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [PlacementMenuComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(PlacementMenuComponent);
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

  it('emits the option it was asked for and folds away', async () => {
    const chosen: (string | null)[] = [];
    fixture.componentInstance.chosen.subscribe((id) => chosen.push(id));
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
    fixture.componentInstance.chosen.subscribe((id) => chosen.push(id));
    await open();

    options()[0].click();
    await fixture.whenStable();

    expect(chosen).toEqual([]);
  });

  it('offers the way out only where there is one', async () => {
    fixture.componentRef.setInput('currentId', 'perf');
    await open();
    expect(fixture.nativeElement.querySelector('[data-testid="editor-placement-none"]')).toBeNull();

    fixture.componentRef.setInput('noneLabel', 'Aucun dossier');
    await fixture.whenStable();

    const chosen: (string | null)[] = [];
    fixture.componentInstance.chosen.subscribe((id) => chosen.push(id));
    fixture.nativeElement.querySelector('[data-testid="editor-placement-none"]').click();
    await fixture.whenStable();

    expect(chosen).toEqual([null]);
  });

  /**
   * ⚠️ This menu lives inside a dialog, unlike every other one in the application. The
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
});
