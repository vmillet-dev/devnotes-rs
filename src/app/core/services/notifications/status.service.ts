import { Injectable, Signal, signal } from '@angular/core';
import { debounced } from '../time/debounce';
import { TranslationRef } from '../i18n/translation-ref.model';

/** Long enough to be read, short enough not to sit across the screen. */
export const STATUS_TTL_MS = 6000;

/** Separate from `ErrorNotifier`: in one banner, a success would read as a problem. */
@Injectable({ providedIn: 'root' })
export class StatusNotifier {
  private readonly _status = signal<TranslationRef | null>(null);

  readonly status: Signal<TranslationRef | null> = this._status.asReadonly();

  private readonly expire = debounced<void>(() => this._status.set(null), STATUS_TTL_MS);

  notify(ref: TranslationRef): void {
    this._status.set(ref);
    this.expire();
  }

  dismiss(): void {
    this.expire.cancel();
    this._status.set(null);
  }
}
