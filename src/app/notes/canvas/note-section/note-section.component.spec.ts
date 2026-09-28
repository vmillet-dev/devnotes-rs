import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NotesStore } from '@core/state/notes.store';
import { SpacesStore } from '@core/state/spaces.store';
import { createNote } from '@testing/note.fixture';
import { createSection } from '@testing/section.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { NoteCardComponent } from '../note-card/note-card.component';
import { NoteSectionComponent } from './note-section.component';

describe('NoteSectionComponent', () => {
  let fixture: ComponentFixture<NoteSectionComponent>;

  function text(selector: string): string {
    return fixture.nativeElement.querySelector(selector).textContent.replace(/\s+/g, ' ').trim();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [NoteSectionComponent],
      // A draft needs somewhere to be filed: creating with no space is refused.
      providers: [provideAppTesting({ spaces: [{ id: 'space-1', name: 'Space one', pinned: false }] })],
    });
    fixture = TestBed.createComponent(NoteSectionComponent);
    fixture.componentRef.setInput('section', createSection('today'));
    fixture.autoDetectChanges();
  });

  it('derives the section title from its key, so no label is kept in sync by hand', async () => {
    fixture.componentRef.setInput('section', createSection('today', [createNote(), createNote({ id: '2' })]));
    await fixture.whenStable();

    expect(text('.canvas-section-title')).toContain("Aujourd'hui");
    expect(text('.n')).toBe('· 2');
  });

  it('translates every section key it can be given', async () => {
    for (const [key, expected] of [
      ['pinned', 'Épinglées'],
      ['week', 'Cette semaine'],
      ['older', 'Plus anciennes'],
      ['results', 'Résultats'],
      ['all', 'Toutes les notes'],
    ] as const) {
      fixture.componentRef.setInput('section', createSection(key));
      await fixture.whenStable();

      expect(text('.canvas-section-title')).toContain(expected);
    }
  });

  /** Every format section is `format`: what it gathers names it, and tells it apart. */
  it('names a gathered section by what it gathers', async () => {
    const titles: string[] = [];
    for (const group of [
      { group: 'priority', priority: 'urgent' },
      { group: 'priority', priority: 'none' },
      { group: 'kind', kind: 'checklist' },
      { group: 'language', language: 'sql' },
    ] as const) {
      const key = group.group === 'priority' ? 'priority' : 'format';
      fixture.componentRef.setInput('section', createSection(key, [], { id: `${key}-x`, group }));
      await fixture.whenStable();
      titles.push(text('.canvas-section-title').split(' · ')[0] ?? '');
    }

    expect(titles).toEqual(['Urgente', 'Sans priorité', 'Tâches', 'SQL']);
    expect(fixture.nativeElement.querySelector('h2').id).toBe('section-heading-format-x');
  });

  it('appends the expiring hint to the count when the section holds an expiring note', async () => {
    fixture.componentRef.setInput('section', createSection('week', [], { hasExpiringNotes: true }));
    await fixture.whenStable();

    expect(text('.n')).toBe('· 0 · à trier bientôt');
  });

  it('names its region after its heading for screen-reader navigation', async () => {
    fixture.componentRef.setInput('section', createSection('pinned'));
    await fixture.whenStable();

    const section = fixture.nativeElement.querySelector('section');
    const heading = fixture.nativeElement.querySelector('h2');
    expect(section.getAttribute('aria-labelledby')).toBe(heading.id);
  });

  it('renders one note card per note', async () => {
    const notes = [createNote({ id: 'a' }), createNote({ id: 'b' })];
    fixture.componentRef.setInput('section', createSection('today', notes));
    await fixture.whenStable();

    const cards = fixture.debugElement
      .queryAll(By.directive(NoteCardComponent))
      .map((card) => card.componentInstance as NoteCardComponent);

    expect(cards.map((card) => card.note().id)).toEqual(['a', 'b']);
  });

  it('forwards a card activation, the one thing the canvas has to arbitrate', async () => {
    fixture.componentRef.setInput('section', createSection('today', [createNote({ id: 'a' })]));
    await fixture.whenStable();
    let emitted: string | undefined;
    fixture.componentInstance.noteActivated.subscribe(({ noteId }) => (emitted = noteId));

    const card = fixture.debugElement.query(By.directive(NoteCardComponent))
      .componentInstance as NoteCardComponent;
    card.opened.emit({ noteId: 'a', toggleChecked: false, extendRange: false });

    expect(emitted).toBe('a');
  });

  it('does not show the create-ghost button unless requested by the section', () => {
    expect(fixture.debugElement.query(By.css('.ghost'))).toBeNull();
  });

  it('opens a draft from the create-ghost button', async () => {
    fixture.componentRef.setInput('section', createSection('week', [], { showCreateGhost: true }));
    await fixture.whenStable();
    await vi.waitFor(() => expect(TestBed.inject(SpacesStore).spaces()).toHaveLength(1));

    fixture.debugElement.query(By.css('.ghost')).triggerEventHandler('click');

    await vi.waitFor(() => expect(TestBed.inject(NotesStore).selectedNote()).not.toBeNull());
  });
});
