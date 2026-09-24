import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { BoardZone } from '@core/model/board.model';
import { Folder } from '@core/model/folder.model';
import { fakeBoardNote, fakeZone } from '@testing/fake-board-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { BoardZoneComponent } from './board-zone.component';

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'amber',
  createdAt: new Date('2026-01-01T10:00:00Z'),
};

const ZONE: BoardZone = fakeZone({
  folder: PERF,
  frame: { x: 40, y: 60, width: 516, height: 200 },
  notes: [
    fakeBoardNote(createNote({ id: 'shown', title: 'Index' })),
    fakeBoardNote(createNote({ id: 'dimmed', title: 'Vacuum' }), { matches: false }),
  ],
});

describe('BoardZoneComponent', () => {
  let fixture: ComponentFixture<BoardZoneComponent>;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  function part(testid: string): HTMLElement | null {
    return root().querySelector(`[data-testid="${testid}"]`);
  }

  function card(id: string): HTMLElement | null {
    return root().querySelector(`.zone-card[data-note-id="${id}"]`);
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [BoardZoneComponent],
      providers: [provideAppTesting()],
    });
    fixture = TestBed.createComponent(BoardZoneComponent);
    fixture.componentRef.setInput('zone', ZONE);
    fixture.componentRef.setInput('frame', ZONE.frame);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('is drawn at its frame, in its colour, with its name and its count', () => {
    const zone = part('board-zone');

    expect(zone?.style.left).toBe('40px');
    expect(zone?.style.top).toBe('60px');
    expect(zone?.className).toContain('is-amber');
    expect(part('board-zone-open')?.textContent).toContain('Perf');
    expect(root().querySelector('.zone-count')?.textContent).toBe('2');
  });

  /** Dimmed, never dropped: reflowing the survivors throws away the board's memory. */
  it('dims the cards the filters do not match, and keeps them', () => {
    expect(card('shown')?.classList.contains('dimmed')).toBe(false);
    expect(card('dimmed')?.classList.contains('dimmed')).toBe(true);
  });

  it('descends into the folder from its title', () => {
    let opened: string | null = null;
    fixture.componentInstance.opened.subscribe((id) => (opened = id));

    part('board-zone-open')?.click();

    expect(opened).toBe('perf');
  });

  it('opens the folder panel from its menu, and closes it after a selection', async () => {
    let selected: string | null = null;
    fixture.componentInstance.selectRequested.subscribe((id) => (selected = id));

    part('board-zone-menu')?.click();
    await fixture.whenStable();
    part('folder-select-notes')?.click();
    await fixture.whenStable();

    expect(selected).toBe('perf');
    expect(part('board-zone-panel')).toBeNull();
  });

  it('says where a card will land while it is dragged over, instead of offering its menu', async () => {
    fixture.componentRef.setInput('isDropTarget', true);
    await fixture.whenStable();

    expect(part('board-zone-drop')).not.toBeNull();
    expect(part('board-zone-menu')).toBeNull();
  });

  it('offers no grip, no menu and no resize when it cannot be edited', async () => {
    fixture.componentRef.setInput('editable', false);
    await fixture.whenStable();

    expect(part('board-zone-grip')).toBeNull();
    expect(part('board-zone-menu')).toBeNull();
    expect(part('board-zone-resize')).toBeNull();
  });

  it('lifts the card being dragged out of it', async () => {
    fixture.componentRef.setInput('draggingNoteId', 'shown');
    await fixture.whenStable();

    expect(card('shown')?.classList.contains('lifted')).toBe(true);
  });
});
