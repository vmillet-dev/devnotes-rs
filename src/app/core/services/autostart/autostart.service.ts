import { InjectionToken, Injectable, Injector, effect, inject } from '@angular/core';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { SettingsStore } from '@core/services/settings/settings.store';

/** A token rather than a direct call, for the same reason as `PREFERENCES_STORE_LOADER`. */
export interface AutostartAdapter {
  enable(): Promise<void>;
  disable(): Promise<void>;
  isEnabled(): Promise<boolean>;
}

export const AUTOSTART_ADAPTER = new InjectionToken<AutostartAdapter>('AUTOSTART_ADAPTER', {
  providedIn: 'root',
  factory: () => ({ enable, disable, isEnabled }),
});

/**
 * The real state belongs to the system, not the preferences file: startup reads the
 * system and aligns the preference to it. Without that read-back, disabling autostart
 * from the task manager leaves the box ticked and gets re-enabled on the next change.
 */
@Injectable({ providedIn: 'root' })
export class AutostartService {
  private readonly adapter = inject(AUTOSTART_ADAPTER);
  private readonly settings = inject(SettingsStore);
  private readonly injector = inject(Injector);

  /** Never rejects: an autostart that cannot be set must not stop the app from opening. */
  async start(): Promise<void> {
    try {
      this.settings.setStartWithSystem(await this.adapter.isEnabled());
    } catch {
      // Outside Tauri, or plugin unavailable: the preference keeps its value.
    }

    effect(
      () => {
        void this.push(this.settings.startWithSystem());
      },
      { injector: this.injector },
    );
  }

  private async push(enabled: boolean): Promise<void> {
    try {
      // Read back first: `enable()` would rewrite the system entry on every start.
      if ((await this.adapter.isEnabled()) === enabled) return;

      await (enabled ? this.adapter.enable() : this.adapter.disable());
    } catch {
      // Silent: the box stays what the user set.
    }
  }
}
