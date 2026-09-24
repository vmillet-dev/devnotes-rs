import { Injectable, Signal, computed, effect, inject, resource, signal } from '@angular/core';
import { SpacesRepository } from '@core/data/spaces.repository';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { Space } from '@core/model/space.model';
import { NotesRevision } from './notes-revision';

/**
 * `null` is not a waiting state but a choice — "all spaces". No "All" entry exists on
 * the data side: it would be a phantom space notes could be filed into.
 */
@Injectable({ providedIn: 'root' })
export class SpacesStore {
  private readonly repository = inject(SpacesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);

  private readonly spacesResource = resource({
    loader: () => this.repository.loadAll(),
    defaultValue: [] as readonly Space[],
  });

  readonly spaces = computed<readonly Space[]>(() =>
    this.spacesResource.hasValue() ? this.spacesResource.value() : [],
  );

  readonly isLoading = this.spacesResource.isLoading;
  readonly loadError: Signal<Error | undefined> = this.spacesResource.error;

  private readonly _activeSpaceId = signal<string | null>(null);

  /** An unknown id falls back to "all spaces" rather than hiding every note. */
  readonly activeSpaceId = computed<string | null>(() => this.activeSpace()?.id ?? null);

  readonly activeSpace = computed<Space | null>(() => {
    const activeId = this._activeSpaceId();
    return activeId === null ? null : (this.spaces().find((space) => space.id === activeId) ?? null);
  });

  constructor() {
    // A failure here empties no screen, so without a banner it would go unnoticed.
    effect(() => {
      const error = this.loadError();
      if (error) {
        this.notifier.notify({ ref: { key: 'errors.spacesLoadFailed' }, detail: error.message });
      }
    });
  }

  reload(): void {
    this.spacesResource.reload();
  }

  selectSpace(id: string | null): void {
    this._activeSpaceId.set(id);
  }

  /** Uniqueness is not checked here: only storage sees the real state of the database. */
  async createSpace(name: string): Promise<Space | null> {
    const trimmed = name.trim();
    if (!trimmed) return null;

    // The typed name is the interpolation fallback when the back end supplies none.
    const created = await this.notifier.attempt(
      'errors.spaceCreateFailed',
      () => this.repository.create({ name: trimmed }),
      { name: trimmed },
    );
    if (!created) return null;

    this.spacesResource.set([...this.spaces(), created]);
    this.selectSpace(created.id);
    return created;
  }

  /** Not optimistic: the list adopts only what persistence returned. */
  async renameSpace(id: string, name: string): Promise<boolean> {
    const trimmed = name.trim();
    const current = this.spaces().find((space) => space.id === id);
    if (!trimmed || !current || current.name === trimmed) return false;

    const renamed = await this.notifier.attempt(
      'errors.spaceRenameFailed',
      () => this.repository.rename(id, { name: trimmed }),
      { name: trimmed },
    );
    if (!renamed) return false;

    this.spacesResource.set(this.spaces().map((space) => (space.id === id ? renamed : space)));
    return true;
  }

  /**
   * Reloads rather than patching the one row: pinning changes the order of the list,
   * and the order is the back end's.
   */
  async togglePinned(id: string): Promise<boolean> {
    const current = this.spaces().find((space) => space.id === id);
    if (!current) return false;

    const updated = await this.notifier.attempt('errors.spacePinFailed', () =>
      this.repository.setPinned(id, !current.pinned),
    );
    if (!updated) return false;

    this.spacesResource.reload();
    return true;
  }

  /** `targetSpaceId` becomes active: the notes have just landed there. */
  async deleteSpace(id: string, targetSpaceId: string): Promise<boolean> {
    // The back end refuses this too; the guard only saves a round trip.
    if (id === targetSpaceId || !this.spaces().some((space) => space.id === targetSpaceId)) {
      return false;
    }

    const deleted = await this.notifier.attempt('errors.spaceDeleteFailed', () =>
      this.repository.delete(id, targetSpaceId),
    );
    if (deleted === null) return false;

    this.spacesResource.set(this.spaces().filter((space) => space.id !== id));
    this.selectSpace(targetSpaceId);
    // The absorbed notes changed `spaceId`, which a query on another space would miss.
    this.revision.bump();
    return true;
  }
}
