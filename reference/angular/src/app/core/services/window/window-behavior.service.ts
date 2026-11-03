import { Injectable, Injector, effect, inject } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { SettingsStore } from '@core/services/settings/settings.store';

/**
 * Pushed to the native side like the tray labels; Rust still carries a default, since
 * the window can be closed before the front has started. Both settings only hold when
 * there is a tray: without one Rust refuses to hide the window, which would leave a
 * process nothing can call back.
 */
@Injectable({ providedIn: 'root' })
export class WindowBehaviorService {
  private readonly settings = inject(SettingsStore);
  private readonly injector = inject(Injector);

  start(): void {
    effect(
      () => {
        void this.push({
          closeToTray: this.settings.closeToTray(),
          minimizeToTray: this.settings.minimizeToTray(),
        });
      },
      { injector: this.injector },
    );
  }

  private async push(behavior: { closeToTray: boolean; minimizeToTray: boolean }): Promise<void> {
    try {
      // No `Result` on the Rust side: it throws if the bridge is absent.
      await commands.setWindowBehavior(behavior);
    } catch {
      // Outside Tauri: a browser window files itself nowhere.
    }
  }
}
