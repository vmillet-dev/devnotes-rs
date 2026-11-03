import { Injectable, inject } from '@angular/core';
import { PREFERENCES_FILE } from '@core/ipc/bindings';
import { KeyValueStore } from './key-value-store';
import { PreferencesService } from './preferences.service';

/**
 * What belongs to these notes rather than to the application: the samples marker, and the view
 * each space was left on. Re-opened on every library switch: a space id means nothing in another
 * library, and a marker carried across would leave a fresh one with no space. The file is named
 * by Rust (`PREFERENCES_FILE`) and told apart from the application's by its directory.
 */
@Injectable({ providedIn: 'root' })
export class LibraryPreferencesService extends KeyValueStore {
  private readonly application = inject(PreferencesService);

  /** @param directory the open library's path, relative to the profile. */
  async hydrate(directory: string): Promise<void> {
    await this.open(directory ? `${directory}/${PREFERENCES_FILE}` : PREFERENCES_FILE);
    this.adoptFromApplication();
  }

  /**
   * ⚠️ Moves the library-scoped keys out of the application's file, once: installs from before
   * the registry kept both scopes in one, and would re-seed a full library. Only into a library
   * with none of its own, so a second library inherits nothing.
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
