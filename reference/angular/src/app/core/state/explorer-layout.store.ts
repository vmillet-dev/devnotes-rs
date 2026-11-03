import { Injectable, signal } from '@angular/core';

/** The code's share of the width, side by side, until the line is moved. */
export const DEFAULT_CODE_SHARE = 40;

/**
 * How the editor lays out code and visualiser: side by side or one at a time, the code hidden
 * or not, and where the line between them stands. Kept for the session, written nowhere.
 */
@Injectable({ providedIn: 'root' })
export class ExplorerLayoutStore {
  private readonly _sideBySide = signal(false);
  private readonly _codeHidden = signal(false);
  private readonly _codeShare = signal(DEFAULT_CODE_SHARE);

  readonly sideBySide = this._sideBySide.asReadonly();
  readonly codeHidden = this._codeHidden.asReadonly();
  readonly codeShare = this._codeShare.asReadonly();

  toggleSideBySide(): void {
    this._sideBySide.update((on) => !on);
  }

  toggleCode(): void {
    this._codeHidden.update((hidden) => !hidden);
  }

  /** `0` is the line's own way of asking for its place back. */
  setCodeShare(share: number): void {
    this._codeShare.set(share > 0 ? share : DEFAULT_CODE_SHARE);
  }
}
