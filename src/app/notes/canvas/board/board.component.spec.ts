import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { createNote } from '@testing/note.fixture';
import { fakeBoardNote, fakeZone } from '@testing/fake-board-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { BoardComponent } from './board.component';

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'amber',
  createdAt: new Date('2026-01-01T10:00:00Z'),
};

describe('BoardComponent', () => {
  let fixture: ComponentFixture<BoardComponent>;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    // ⚠️ Only `Date`: the zoneless scheduler needs real rAF for `whenStable()`.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-01-10T12:00:00Z'));

    TestBed.configureTestingModule({
      imports: [BoardComponent],
      providers: [provideAppTesting()],
    });
    fixture = TestBed.createComponent(BoardComponent);
    fixture.componentRef.setInput('zones', []);
    fixture.componentRef.setInput('loose', []);
    fixture.componentRef.setInput('width', 1200);
    fixture.componentRef.setInput('height', 800);
    fixture.autoDetectChanges();
  });

  it('sizes the surface from what the back end says is on it', () => {
    const surface = root().querySelector<HTMLElement>('.board-surface');

    expect(surface?.style.width).toBe('1200px');
    expect(surface?.style.height).toBe('800px');
  });

  it('places a zone at its stored frame', async () => {
    fixture.componentRef.setInput('zones', [
      fakeZone({ folder: PERF, frame: { x: 580, y: 16, width: 300, height: 372 } }),
    ]);
    await fixture.whenStable();

    const zone = root().querySelector<HTMLElement>('[data-testid="board-zone"]');
    expect(zone?.style.left).toBe('580px');
    expect(zone?.style.top).toBe('16px');
    expect(zone?.style.width).toBe('300px');
  });

  it('carries the folder colour onto its zone', async () => {
    fixture.componentRef.setInput('zones', [fakeZone({ folder: PERF })]);
    await fixture.whenStable();

    expect(root().querySelector('[data-testid="board-zone"]')?.className).toContain('is-amber');
  });

  /** ⚠️ The inside of a zone is a flow, not a second set of coordinates to maintain. */
  it('lets the cards of a zone flow rather than placing them', async () => {
    fixture.componentRef.setInput('zones', [
      fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] }),
    ]);
    await fixture.whenStable();

    const card = root().querySelector<HTMLElement>('[data-testid="board-zone"] .zone-card');
    expect(card?.style.left).toBe('');
    expect(card?.style.top).toBe('');
  });

  it('places a loose card at its own position', async () => {
    fixture.componentRef.setInput('loose', [
      fakeBoardNote(createNote({ id: 'b' }), { position: { x: 276, y: 426 } }),
    ]);
    await fixture.whenStable();

    const card = root().querySelector<HTMLElement>('[data-testid="board-loose-card"]');
    expect(card?.style.left).toBe('276px');
    expect(card?.style.top).toBe('426px');
  });

  /** ⚠️ Dimmed in place: a reflow throws away the only thing the board has. */
  it('dims what a search did not match instead of dropping it', async () => {
    fixture.componentRef.setInput('zones', [
      fakeZone({
        folder: PERF,
        notes: [
          fakeBoardNote(createNote({ id: 'a' })),
          fakeBoardNote(createNote({ id: 'b' }), { matches: false }),
        ],
      }),
    ]);
    await fixture.whenStable();

    const cards = root().querySelectorAll('[data-testid="board-zone"] .zone-card');
    expect(cards).toHaveLength(2);
    expect(cards[0].className).not.toContain('dimmed');
    expect(cards[1].className).toContain('dimmed');
  });

  it('says so when there is nothing to arrange', () => {
    expect(fixture.debugElement.query(By.css('[data-testid="board-empty"]'))).not.toBeNull();
  });

  describe('the pointer gesture', () => {
    // ⚠️ HTML5 drag & drop does not work in this WebView: the pointer events are the path
    // to cover, not a `dragstart` that would never arrive.
    function pointer(target: Element, type: string, x = 0, y = 0, button = 0): void {
      target.dispatchEvent(
        new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, button, pointerId: 1 }),
      );
    }

    function surface(): HTMLElement {
      return root().querySelector<HTMLElement>('.board-surface')!;
    }

    beforeEach(async () => {
      fixture.componentRef.setInput('zones', [
        fakeZone({ folder: PERF, frame: { x: 0, y: 0, width: 300, height: 300 } }),
      ]);
      fixture.componentRef.setInput('loose', [
        fakeBoardNote(createNote({ id: 'loose-1' }), { position: { x: 400, y: 400 } }),
      ]);
      await fixture.whenStable();

      // jsdom lays nothing out, so the surface has to say where it is.
      surface().getBoundingClientRect = () => ({ left: 0, top: 0 }) as DOMRect;
    });

    /** ⚠️ The card itself: the whole of it is the handle, there is no grip any more. */
    function grip(): HTMLElement {
      return root().querySelector<HTMLElement>('[data-testid="board-loose-card"] .card-title')!;
    }

    /**
     * ⚠️ The whole card is the handle, so a drag ends with a click on the card it moved —
     * and that click must not also open the note.
     */
    it('swallows the click a drag leaves behind, and only that one', async () => {
      const opened: string[] = [];
      fixture.componentInstance.noteActivated.subscribe(({ noteId }) => opened.push(noteId));
      const activate = () =>
        fixture.debugElement
          .query(By.css('[data-testid="board-loose-card"] app-note-card'))
          .triggerEventHandler('opened', { noteId: 'loose-1', toggleChecked: false, extendRange: false });

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 700, 500);
      pointer(surface(), 'pointerup', 700, 500);
      await fixture.whenStable();
      activate();

      expect(opened).toEqual([]);

      // The next press starts clean: the note opens on a press that goes nowhere.
      pointer(grip(), 'pointerdown', 400, 400);
      pointer(grip(), 'pointerup', 400, 400);
      await fixture.whenStable();
      activate();

      expect(opened).toEqual(['loose-1']);
    });

    /** A press aimed at a control is that control's, not the start of a drag. */
    it('starts no gesture from a control of the card', async () => {
      const seen: unknown[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));
      const tick = root().querySelector<HTMLElement>('[data-testid="note-card-check"]')!;

      pointer(tick, 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 150, 150);
      pointer(surface(), 'pointerup', 150, 150);
      await fixture.whenStable();

      expect(seen).toEqual([]);
    });

    /** ⚠️ A click must not persist anything: it is how a note is opened. */
    it('writes nothing when the pointer never travelled', async () => {
      const seen: unknown[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(grip(), 'pointerup', 401, 401);
      await fixture.whenStable();

      expect(seen).toEqual([]);
    });

    it('files a card dropped inside a zone', async () => {
      const seen: { noteId: string; folderId: string | null }[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 150, 150);
      pointer(surface(), 'pointerup', 150, 150);
      await fixture.whenStable();

      // Snapped to the grid the background draws: 150 lands on 160.
      expect(seen).toEqual([{ noteId: 'loose-1', folderId: 'perf', position: { x: 160, y: 160 } }]);
    });

    /** Membership comes from the drop, in both directions. */
    it('unfiles a card dropped on the background, and says where it landed', async () => {
      const seen: { folderId: string | null; position: { x: number; y: number } }[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 700, 500);
      pointer(surface(), 'pointerup', 700, 500);
      await fixture.whenStable();

      expect(seen[0]?.folderId).toBeNull();
      expect(seen[0]?.position).toEqual({ x: 700, y: 500 });
    });

    it('says which zone a drop would land in before the pointer lifts', async () => {
      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 150, 150);
      await fixture.whenStable();

      expect(root().querySelector('[data-testid="board-zone-drop"]')).not.toBeNull();
    });

    /**
     * ⚠️ A filed card flows inside its zone and has no coordinates of its own, so nothing
     * followed the pointer at all — half a gesture, with only the zone lighting up.
     */
    describe('a card dragged out of a zone', () => {
      /** ⚠️ The card itself: the whole of it is the handle, there is no grip any more. */
      function zoneGrip(): HTMLElement {
        return root().querySelector<HTMLElement>('.zone-card .card-title')!;
      }

      function ghost(): HTMLElement | null {
        return root().querySelector<HTMLElement>('[data-testid="board-ghost-card"]');
      }

      beforeEach(async () => {
        fixture.componentRef.setInput('zones', [
          fakeZone({
            folder: PERF,
            frame: { x: 0, y: 0, width: 300, height: 300 },
            notes: [fakeBoardNote(createNote({ id: 'filed-1', title: 'EXPLAIN lent' }))],
          }),
        ]);
        await fixture.whenStable();
      });

      it('is drawn under the pointer for the whole gesture', async () => {
        pointer(zoneGrip(), 'pointerdown', 40, 40);
        expect(ghost()).toBeNull();

        pointer(surface(), 'pointermove', 500, 450);
        await fixture.whenStable();

        expect(ghost()).not.toBeNull();
        expect(ghost()?.getAttribute('data-note-id')).toBe('filed-1');
        // ⚠️ By the grab offset, not under the pointer: the card was grabbed 40px into its
        // seat, so it travels 460 rather than jumping its own corner onto the cursor —
        // then snapped to the grid, which puts 410 on 420.
        expect(ghost()?.style.left).toBe('460px');
        expect(ghost()?.style.top).toBe('420px');
      });

      /** Its seat stays behind, faded: the drop can still be cancelled. */
      it('leaves the card in its zone until the pointer lifts', async () => {
        pointer(zoneGrip(), 'pointerdown', 40, 40);
        pointer(surface(), 'pointermove', 500, 450);
        await fixture.whenStable();

        expect(root().querySelector('.zone-card.lifted')).not.toBeNull();
      });

      it('takes it away again when the gesture is cancelled', async () => {
        pointer(zoneGrip(), 'pointerdown', 40, 40);
        pointer(surface(), 'pointermove', 500, 450);
        await fixture.whenStable();

        pointer(surface(), 'pointercancel', 500, 450);
        await fixture.whenStable();

        expect(ghost()).toBeNull();
      });
    });

    /** ⚠️ A pointer the system took back must not leave a card where nobody put it. */
    it('throws the whole gesture away when the pointer is cancelled', async () => {
      const seen: unknown[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 150, 150);
      pointer(surface(), 'pointercancel', 150, 150);
      pointer(surface(), 'pointerup', 150, 150);
      await fixture.whenStable();

      expect(seen).toEqual([]);
    });

    it('moves a zone by its grip, carrying its size along', async () => {
      const seen: { folderId: string; frame: { x: number; width: number } }[] = [];
      fixture.componentInstance.zoneMoved.subscribe((move) => seen.push(move));
      const zoneGrip = root().querySelector<HTMLElement>('[data-testid="board-zone-grip"]')!;

      pointer(zoneGrip, 'pointerdown', 20, 20);
      pointer(surface(), 'pointermove', 220, 120);
      pointer(surface(), 'pointerup', 220, 120);
      await fixture.whenStable();

      expect(seen[0]?.folderId).toBe('perf');
      expect(seen[0]?.frame).toEqual({ x: 200, y: 100, width: 300, height: 300 });
    });

    /** ⚠️ Resizing captures and releases nothing — the notes are not even consulted. */
    it('resizes a zone from its corner without touching what is in it', async () => {
      const seen: { frame: { x: number; y: number; width: number; height: number } }[] = [];
      fixture.componentInstance.zoneMoved.subscribe((move) => seen.push(move));
      const handle = root().querySelector<HTMLElement>('[data-testid="board-zone-resize"]')!;

      pointer(handle, 'pointerdown', 300, 300);
      pointer(surface(), 'pointermove', 500, 450);
      pointer(surface(), 'pointerup', 500, 450);
      await fixture.whenStable();

      expect(seen[0]?.frame).toEqual({ x: 0, y: 0, width: 500, height: 460 });
    });

    it('draws a band on the background and asks for a zone', async () => {
      const seen: { width: number; height: number }[] = [];
      fixture.componentInstance.zoneDrawn.subscribe((frame) => seen.push(frame));

      pointer(surface(), 'pointerdown', 600, 600);
      pointer(surface(), 'pointermove', 1000, 900);
      await fixture.whenStable();
      expect(root().querySelector('[data-testid="board-draw-band"]')).not.toBeNull();

      pointer(surface(), 'pointerup', 1000, 900);
      await fixture.whenStable();

      expect(seen).toEqual([{ x: 600, y: 600, width: 400, height: 300 }]);
    });

    /** A band barely dragged was a click on the background. */
    it('creates nothing from a band too small to have been meant', async () => {
      const seen: unknown[] = [];
      fixture.componentInstance.zoneDrawn.subscribe((frame) => seen.push(frame));

      pointer(surface(), 'pointerdown', 600, 600);
      pointer(surface(), 'pointermove', 615, 615);
      pointer(surface(), 'pointerup', 615, 615);
      await fixture.whenStable();

      expect(seen).toEqual([]);
    });

    /** Without a space there is nothing to file into, so nothing may be grabbed either. */
    it('moves nothing at all when the board is not editable', async () => {
      const seen: unknown[] = [];
      fixture.componentInstance.cardDropped.subscribe((drop) => seen.push(drop));
      fixture.componentRef.setInput('editable', false);
      await fixture.whenStable();

      pointer(grip(), 'pointerdown', 400, 400);
      pointer(surface(), 'pointermove', 150, 150);
      pointer(surface(), 'pointerup', 150, 150);
      await fixture.whenStable();

      expect(seen).toEqual([]);
      expect(root().querySelector('[data-testid="board-zone-resize"]')).toBeNull();
    });
  });

  it('asks to descend when a zone title is clicked', async () => {
    const seen: string[] = [];
    fixture.componentInstance.folderOpened.subscribe((id) => seen.push(id));
    fixture.componentRef.setInput('zones', [fakeZone({ folder: PERF })]);
    await fixture.whenStable();

    root().querySelector<HTMLElement>('[data-testid="board-zone-open"]')?.click();
    await fixture.whenStable();

    expect(seen).toEqual(['perf']);
  });
});
