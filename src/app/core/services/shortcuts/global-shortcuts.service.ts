import { Injectable, Injector, effect, inject } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindings } from './shortcut.model';

/**
 * A global shortcut is first-come, first-served across the machine and the loser gets
 * no error, so without this message pressing the key does nothing and nothing says why.
 * All three travel together because the native command takes them as a block.
 */
@Injectable({ providedIn: 'root' })
export class GlobalShortcutsService {
  private readonly notifier = inject(ErrorNotifier);
  private readonly settings = inject(SettingsStore);
  private readonly injector = inject(Injector);

  /** ⚠️ Called outside a constructor, so the injector is passed explicitly. */
  start(): void {
    effect(
      () => {
        void this.apply({
          palette: this.settings.paletteShortcut(),
          capture: this.settings.captureShortcut(),
          newNote: this.settings.newNoteShortcut(),
        });
      },
      { injector: this.injector },
    );
  }

  private async apply(bindings: ShortcutBindings): Promise<void> {
    try {
      // No `Result` on the Rust side: it throws when the bridge is absent.
      const taken = await commands.setGlobalShortcuts(bindings);
      if (taken.length > 0) {
        // The count travels beside the list: the sentence agrees three times over —
        // the noun, the adjective and the participle — and a list cannot be counted by
        // the translation.
        this.notifier.notify({
          ref: {
            key: 'shortcuts.unavailable',
            params: { count: taken.length, list: taken.join(', ') },
          },
        });
      }
    } catch {
      // Outside Tauri: there is no global shortcut to take.
    }
  }
}
