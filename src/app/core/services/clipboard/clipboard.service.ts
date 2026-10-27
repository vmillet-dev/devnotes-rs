import { InjectionToken, Injectable, inject } from '@angular/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { SettingsStore } from '@core/services/settings/settings.store';

/** A token rather than a direct call, for the same reason as `PREFERENCES_STORE_LOADER`. */
export interface ClipboardAdapter {
  readText(): Promise<string | null>;
  writeText(value: string): Promise<void>;
}

export const CLIPBOARD_ADAPTER = new InjectionToken<ClipboardAdapter>('CLIPBOARD_ADAPTER', {
  providedIn: 'root',
  factory: () => ({ readText, writeText }),
});

/**
 * ⚠️ The CSP locks the WebView to `'self'` and `navigator.clipboard` is unusable there:
 * everything goes through the plugin, which throws outside Tauri — hence the success
 * boolean. The copy acknowledgement lives here and not in the callers, which say what they
 * copied through `said`: « Accuser chaque copie » off, the card's own message went on (PRF-10).
 */
@Injectable({ providedIn: 'root' })
export class ClipboardService {
  private readonly adapter = inject(CLIPBOARD_ADAPTER);
  private readonly notifier = inject(StatusNotifier);
  private readonly settings = inject(SettingsStore);

  /** `said` `null`: the caller says it elsewhere, behind `acknowledges`. */
  async copy(value: string, said: TranslationRef | null = { key: 'settings.copied' }): Promise<boolean> {
    try {
      await this.adapter.writeText(value);
    } catch {
      return false;
    }

    if (said !== null && this.acknowledges()) {
      this.notifier.notify(said);
    }

    return true;
  }

  acknowledges(): boolean {
    return this.settings.copyConfirmation();
  }

  /** An empty string for an empty clipboard as for an impossible read. */
  async paste(): Promise<string> {
    try {
      return (await this.adapter.readText()) ?? '';
    } catch {
      return '';
    }
  }
}
