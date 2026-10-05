import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { NoteKind } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { NewNoteButtonComponent } from './new-note-button.component';

describe('NewNoteButtonComponent', () => {
  let fixture: ComponentFixture<NewNoteButtonComponent>;
  let created: (NoteKind | null)[];

  function click(selector: string): void {
    (fixture.nativeElement.querySelector(selector) as HTMLElement).click();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [NewNoteButtonComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(NewNoteButtonComponent);
    fixture.componentRef.setInput('shortcuts', {
      snippet: ['Ctrl', 'N'],
      note: ['Ctrl', 'Shift', 'N'],
      checklist: ['Alt', 'L'],
    });
    // Focus management moves through real elements, which jsdom only tracks for
    // an attached tree.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();

    created = [];
    fixture.componentInstance.created.subscribe((kind) => created.push(kind));
  });

  /** No kind: the store opens the one the preferences name. */
  it('creates a note of the default kind without opening anything', () => {
    click('.new-note-btn');

    expect(created).toEqual([null]);
    expect(fixture.nativeElement.querySelector('.new-note-menu')).toBeNull();
  });

  it('opens the menu from the caret and announces it as expanded', async () => {
    click('.new-note-caret');
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.new-note-menu')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('.new-note-caret').getAttribute('aria-expanded')).toBe('true');
  });

  it('emits the kind picked in the menu, then closes it', async () => {
    for (const [index, kind] of [
      [1, 'note'],
      [2, 'checklist'],
    ] as const) {
      click('.new-note-caret');
      await fixture.whenStable();

      fixture.nativeElement.querySelectorAll('.new-note-option')[index].click();
      await fixture.whenStable();

      expect(created.at(-1)).toBe(kind);
      expect(fixture.nativeElement.querySelector('.new-note-menu')).toBeNull();
    }
  });

  it('offers exactly the three kinds, the snippet first', async () => {
    click('.new-note-caret');
    await fixture.whenStable();

    const names = [...fixture.nativeElement.querySelectorAll('.kind-name')].map((name) =>
      (name as HTMLElement).textContent?.trim(),
    );
    expect(names).toEqual(['Snippet de code', 'Note', 'Liste de tâches']);
  });

  /** A rebound key is the one named: the caps come in from the bindings, not from here. */
  it('names the keys each kind is bound to', async () => {
    click('.new-note-caret');
    await fixture.whenStable();

    const options: HTMLElement[] = [...fixture.nativeElement.querySelectorAll('.new-note-option')];
    const caps = options.map((option) => [...option.querySelectorAll('kbd')].map((kbd) => kbd.textContent));

    expect(caps).toEqual([
      ['Ctrl', 'N'],
      ['Ctrl', 'Shift', 'N'],
      ['Alt', 'L'],
    ]);
    expect(options[1]?.getAttribute('aria-keyshortcuts')).toBe('Control+Shift+N');
  });

  it('marks the Note new only while it is', async () => {
    fixture.componentRef.setInput('noteIsNew', true);
    click('.new-note-caret');
    await fixture.whenStable();

    const badge = () => fixture.nativeElement.querySelector('[data-testid="new-note-badge"]');
    expect(badge()?.closest('[data-testid="new-note-note"]')).not.toBeNull();

    fixture.componentRef.setInput('noteIsNew', false);
    await fixture.whenStable();
    expect(badge()).toBeNull();
  });

  it('closes on Escape without creating anything', async () => {
    click('.new-note-caret');
    await fixture.whenStable();

    fixture.nativeElement
      .querySelector('.new-note-menu')
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.new-note-menu')).toBeNull();
    expect(created).toEqual([]);
  });
});
