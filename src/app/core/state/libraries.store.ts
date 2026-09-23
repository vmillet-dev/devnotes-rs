import { Injectable, computed, inject, signal } from '@angular/core';
import { LibrariesRepository, LibraryEntry } from '@core/data/libraries.repository';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { AppWindowService } from '@core/services/window/app-window.service';

/** What a deletion would take, named before it runs. */
export interface PendingLibraryDeletion {
  readonly entry: LibraryEntry;
}

/**
 * The libraries, and which one is open.
 *
 * ⚠️ Switching is a full teardown: the command empties the connection and the front end
 * reloads onto the gate. The other library has its own passphrase, and asking for it is
 * the only proof the right one is open.
 */
@Injectable({ providedIn: 'root' })
export class LibrariesStore {
  private readonly repository = inject(LibrariesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly window = inject(AppWindowService);

  private readonly _libraries = signal<readonly LibraryEntry[]>([]);
  private readonly _openId = signal<string | null>(null);
  private readonly _isWorking = signal(false);
  private readonly _pendingDeletion = signal<PendingLibraryDeletion | null>(null);

  readonly libraries = this._libraries.asReadonly();
  readonly isWorking = this._isWorking.asReadonly();

  /**
   * ⚠️ Named before it runs, like emptying the trash and like a corpus-wide retag: this
   * erases a whole library outright, and a second click on the button that fired it is
   * the guard a double click defeats.
   */
  readonly pendingDeletion = this._pendingDeletion.asReadonly();

  readonly open = computed<LibraryEntry | null>(
    () => this._libraries().find((entry) => entry.id === this._openId()) ?? null,
  );

  /** ⚠️ One library is not a choice: the menu shows the entry only past the first. */
  readonly hasSeveral = computed(() => this._libraries().length > 1);

  /**
   * Reads the registry and opens the library's own preference file.
   *
   * ⚠️ Both, always together. The file's path comes from the entry, so a registry read
   * without the hydrate leaves the samples marker and the per-space views pointing at
   * whichever library was open last.
   */
  async load(): Promise<void> {
    await this.notifier.attempt('errors.librariesListFailed', async () => {
      const registry = await this.repository.list();
      this._libraries.set(registry.libraries);
      this._openId.set(registry.open);

      await this.preferences.hydrate(this.open()?.directory ?? '');
    });
  }

  /**
   * ⚠️ Creates **and opens**. One gesture rather than two: you have just named it, so you
   * want to be in it — the surprise would be staying where you were. The gate then asks
   * for a phrase, which is what a first launch does too.
   */
  async create(name: string): Promise<LibraryEntry | null> {
    return this.attempt(async () => {
      const entry = await this.repository.create(name);
      await this.repository.open(entry.id);
      this.window.reload();

      return entry;
    });
  }

  /**
   * ⚠️ A reload, not a `vault.load()`: every store is `providedIn: 'root'` and would carry
   * the other library's spaces, board and undo record past the gate. The vault state lives
   * in Rust, so the page comes back on the gate of the library just opened.
   */
  async openLibrary(id: string): Promise<void> {
    if (id === this._openId()) return;

    await this.attempt(async () => {
      await this.repository.open(id);
      this.window.reload();
    });
  }

  async rename(id: string, name: string): Promise<void> {
    await this.attempt(async () => {
      await this.repository.rename(id, name);
      await this.load();
    });
  }

  /**
   * ⚠️ Proposes only. `confirmDeletion` is what erases.
   *
   * ⚠️ Never the open one — deleting the files under a live connection is how a library
   * that was merely unwanted takes the process down — and never the last, which would
   * leave the gate with nothing to offer. Refused here *and* in Rust: a command is
   * reachable from more than the interface.
   */
  askToDelete(entry: LibraryEntry): void {
    if (entry.id === this._openId() || !this.hasSeveral()) return;

    this._pendingDeletion.set({ entry });
  }

  dismissDeletion(): void {
    this._pendingDeletion.set(null);
  }

  async confirmDeletion(): Promise<void> {
    const pending = this._pendingDeletion();
    if (pending === null) return;

    await this.attempt(async () => {
      await this.repository.delete(pending.entry.id);
      this._pendingDeletion.set(null);
      await this.load();
      this.status.notify({ key: 'libraries.deleted', params: { name: pending.entry.name } });
    });
  }

  private async attempt<T>(action: () => Promise<T>): Promise<T | null> {
    if (this._isWorking()) return null;

    return this.notifier.attemptWhile(this._isWorking, 'errors.libraryActionFailed', action);
  }
}
