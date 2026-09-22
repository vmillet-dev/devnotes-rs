import { Injectable, inject } from '@angular/core';
import { LIBRARY_PREFERENCES_FILE } from '@core/ipc/bindings';
import { KeyValueStore } from './key-value-store';
import { PreferencesService } from './preferences.service';

/**
 * What belongs to *these* notes rather than to the application: the samples marker, and
 * which view each space was left on.
 *
 * ⚠️ Re-opened on every library switch, where `PreferencesService` is opened once. That
 * is the whole point of the split — a space id means nothing in another library, and a
 * samples marker carried across would leave a fresh library empty with no way to create
 * a note, since a note needs a space.
 *
 * ⚠️ The file is named by Rust (`LIBRARY_PREFERENCES_FILE`) and not spelled here. Rust
 * decides where a library lives, so it decides this too — and ⚠️ it is **not**
 * `preferences.json`: the first library's directory *is* the profile root, where the
 * application's own file already sits.
 */
@Injectable({ providedIn: 'root' })
export class LibraryPreferencesService extends KeyValueStore {
  private readonly application = inject(PreferencesService);

  /**
   * @param directory the open library's path relative to the profile — empty for the
   * first one, which lives at the root.
   */
  async hydrate(directory: string): Promise<void> {
    await this.open(directory ? `${directory}/${LIBRARY_PREFERENCES_FILE}` : LIBRARY_PREFERENCES_FILE);
    this.adoptFromApplication();
  }

  /**
   * ⚠️ Moves the library-scoped keys out of the application's file, once. Every install
   * before the registry kept both scopes in one — without this, the first launch after
   * the upgrade reads no samples marker and re-seeds a library that is full.
   *
   * ⚠️ Only into a library that has none of its own: a second library must not inherit
   * the first one's markers, and the application file is emptied of them by the first
   * adoption anyway.
   */
  private adoptFromApplication(): void {
    for (const [key, value] of this.application.libraryScoped()) {
      if (this.read(key) === null) {
        this.write(key, value);
      }
      this.application.forget(key);
    }
  }
}
