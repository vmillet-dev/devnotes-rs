import { guard } from './fail-next';
import { BoardRepository } from '@core/data/board.repository';
import {
  BoardArrangement,
  BoardLayout,
  BoardNote,
  BoardQuery,
  BoardView,
  BoardScope,
  BoardZone,
  CardPlacement,
  ZonePlacement,
} from '@core/model/board.model';

const EMPTY: BoardView = {
  zones: [],
  loose: [],
  availableTags: [],
  availableLanguages: [],
  isFiltering: false,
  matched: 0,
  width: 960,
  height: 540,
};

export class FakeBoardRepository implements Pick<BoardRepository, keyof BoardRepository> {
  private view: BoardView = EMPTY;

  /** What the store last asked for, so a spec can assert the query it composed. */
  lastQuery: BoardQuery | null = null;
  queryCount = 0;

  /** Every batch it was handed, so a spec can assert one gesture wrote once. */
  readonly saved: { zones: readonly ZonePlacement[]; cards: readonly CardPlacement[] }[] = [];

  /** Each tidy-up it was asked for, and what it answers with. */
  readonly arranged: { spaceId: string; scope: BoardScope }[] = [];
  readonly restored: BoardLayout[] = [];
  arrangement: BoardArrangement = { moved: 0, previous: { zones: [], cards: [] } };

  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  constructor(view: Partial<BoardView> = {}) {
    this.view = { ...EMPTY, ...view };
  }

  setView(view: Partial<BoardView>): void {
    this.view = { ...this.view, ...view };
  }

  query(query: BoardQuery): Promise<BoardView> {
    return guard(this, () => {
      this.lastQuery = query;
      this.queryCount += 1;
      return this.view;
    });
  }

  saveLayout(zones: readonly ZonePlacement[], cards: readonly CardPlacement[]): Promise<void> {
    return guard(this, () => {
      this.saved.push({ zones: [...zones], cards: [...cards] });
    });
  }

  arrange(spaceId: string, scope: BoardScope): Promise<BoardArrangement> {
    return guard(this, () => {
      this.arranged.push({ spaceId, scope });
      return this.arrangement;
    });
  }

  restoreLayout(layout: BoardLayout): Promise<void> {
    return guard(this, () => {
      this.restored.push(layout);
    });
  }
}

/** A zone is mostly its frame and its notes; a spec cares about one or two of them. */
export function fakeZone(overrides: Partial<BoardZone> & Pick<BoardZone, 'folder'>): BoardZone {
  return {
    frame: { x: 16, y: 16, width: 516, height: 200 },
    notes: [],
    ...overrides,
  };
}

export function fakeBoardNote(note: BoardNote['note'], overrides: Partial<BoardNote> = {}): BoardNote {
  return { note, matches: true, position: null, ...overrides };
}
