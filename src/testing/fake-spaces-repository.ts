import { guard } from './fail-next';
import { SpacesRepository } from '@core/data/spaces.repository';
import { Space, SpaceContents, SpaceDraft } from '@core/model/space.model';

export class FakeSpacesRepository implements Pick<SpacesRepository, keyof SpacesRepository> {
  private spaces: readonly Space[];
  private nextId = 0;

  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  /** What a spec says each space holds: this double owns no notes to count. */
  readonly contents = new Map<string, SpaceContents>();

  constructor(spaces: readonly Space[] = []) {
    this.spaces = spaces;
  }

  loadAll(): Promise<readonly Space[]> {
    return guard(this, () => this.spaces);
  }

  create(draft: SpaceDraft): Promise<Space> {
    return guard(this, () => {
      const space: Space = { id: `fake-space-${++this.nextId}`, name: draft.name, pinned: false };
      this.spaces = [...this.spaces, space];
      return space;
    });
  }

  rename(id: string, draft: SpaceDraft): Promise<Space> {
    return guard(this, () => {
      const existing = this.spaces.find((space) => space.id === id);
      const renamed: Space = { id, name: draft.name, pinned: existing?.pinned ?? false };
      this.spaces = this.spaces.map((space) => (space.id === id ? renamed : space));
      return renamed;
    });
  }

  /** Hoists like the real one: a double that only flipped the flag would let a broken sort pass. */
  setPinned(id: string, pinned: boolean): Promise<Space> {
    return guard(this, () => {
      const updated = this.spaces.map((space) => (space.id === id ? { ...space, pinned } : space));
      this.spaces = [...updated].sort(
        (a, b) => Number(b.pinned) - Number(a.pinned) || a.name.localeCompare(b.name),
      );

      const found = this.spaces.find((space) => space.id === id);
      if (!found) {
        throw new Error(`Unknown space: ${id}`);
      }
      return found;
    });
  }

  countContents(id: string): Promise<SpaceContents> {
    return guard(this, () => this.contents.get(id) ?? { notes: 0, folders: 0 });
  }

  /** The target is accepted without checking it holds the notes: this double owns none. */
  delete(id: string, _targetSpaceId: string): Promise<void> {
    return guard(this, () => {
      this.spaces = this.spaces.filter((space) => space.id !== id);
    });
  }
}
