import { Injectable, inject, signal } from '@angular/core';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { PrettierSettings, readPrettierSettings } from './format.model';

const SETTINGS_KEY = 'devnotes.notes.prettier';

/** Applied as it is chosen, like the canvas's arrangement: the panel has no button to confirm. */
@Injectable({ providedIn: 'root' })
export class PrettierSettingsStore {
  private readonly preferences = inject(LibraryPreferencesService);

  private readonly _settings = signal<PrettierSettings>(
    readPrettierSettings(this.preferences.read(SETTINGS_KEY)),
  );
  readonly settings = this._settings.asReadonly();

  update(change: Partial<PrettierSettings>): void {
    const next = { ...this._settings(), ...change };
    this._settings.set(next);
    this.preferences.write(SETTINGS_KEY, JSON.stringify(next));
  }
}
