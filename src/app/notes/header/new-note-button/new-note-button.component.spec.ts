import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { NoteKind } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { NewNoteButtonComponent } from './new-note-button.component';

describe('NewNoteButtonComponent', () => {
  let fixture: ComponentFixture<NewNoteButtonComponent>;
  let created: NoteKind[];

  function click(selector: string): void {
    (fixture.nativeElement.querySelector(selector) as HTMLElement).click();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [NewNoteButtonComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(NewNoteButtonComponent);
    // Focus management moves through real elements, which jsdom only tracks for
    // an attached tree.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();

    created = [];
    fixture.componentInstance.created.subscribe((kind) => created.push(kind));
  });

  it('creates an ordinary note without opening anything', () => {
    click('.new-note-btn');

    expect(created).toEqual(['snippet']);
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

    const labels = [...fixture.nativeElement.querySelectorAll('.new-note-option')].map((option) =>
      (option as HTMLElement).textContent?.trim(),
    );
    expect(labels).toEqual(['Snippet de code', 'Note', 'Liste de tâches']);
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
