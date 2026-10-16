import { Injectable, signal } from '@angular/core';

/** Open from the File menu or from the palette's `>`: a store, since neither owns the other. */
@Injectable({ providedIn: 'root' })
export class PreferencesDialogStore {
  private readonly _isOpen = signal(false);

  readonly isOpen = this._isOpen.asReadonly();

  open(): void {
    this._isOpen.set(true);
  }

  close(): void {
    this._isOpen.set(false);
  }
}
