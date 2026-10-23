import { Injectable, Signal, signal } from '@angular/core';
import { debounced } from '@core/services/time/debounce';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';

/** Long enough to be read, short enough not to sit across the screen. */
export const STATUS_TTL_MS = 6000;

const AFTER_RELOAD_KEY = 'devnotes.status.afterReload';

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

  /** For what reloads the page as it ends, as a restore does: said by the page that follows. */
  notifyAfterReload(ref: TranslationRef): void {
    try {
      sessionStorage.setItem(AFTER_RELOAD_KEY, JSON.stringify(ref));
    } catch {
      this.notify(ref);
    }
  }

  /** Once, at startup. */
  resumeAfterReload(): void {
    try {
      const kept = sessionStorage.getItem(AFTER_RELOAD_KEY);
      sessionStorage.removeItem(AFTER_RELOAD_KEY);
      if (kept) this.notify(JSON.parse(kept) as TranslationRef);
    } catch {
      // Nothing kept, or nothing to keep it in: there is nothing to say.
    }
  }
}
