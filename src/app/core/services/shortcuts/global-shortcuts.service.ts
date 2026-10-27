import { Injectable, Injector, computed, effect, inject, signal, untracked } from '@angular/core';
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

  private readonly bindings = computed<ShortcutBindings>(() => ({
    palette: this.settings.paletteShortcut(),
    capture: this.settings.captureShortcut(),
    newNote: this.settings.newNoteShortcut(),
  }));

  /** While a shortcut field listens: Windows would hand it our own combinations, never the field. */
  private readonly suspended = signal(false);

  /** ⚠️ Called outside a constructor, so the injector is passed explicitly. */
  start(): void {
    effect(
      () => {
        const bindings = this.bindings();
        if (untracked(this.suspended)) return;
        void this.apply(bindings);
      },
      { injector: this.injector },
    );
  }

  suspend(): void {
    if (this.suspended()) return;
    this.suspended.set(true);
    void this.call(() => commands.releaseGlobalShortcuts());
  }

  /** Taken back as they now stand, quietly: what another application holds was said when they were set. */
  resume(): void {
    if (!this.suspended()) return;
    this.suspended.set(false);
    void this.apply(untracked(this.bindings), false);
  }

  private async apply(bindings: ShortcutBindings, announce = true): Promise<void> {
    const taken = await this.call(() => commands.setGlobalShortcuts(bindings));
    if (announce && taken && taken.length > 0) {
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
  }

  /** No `Result` on the Rust side: it throws when the bridge is absent, and outside Tauri there is nothing to take. */
  private async call<T>(command: () => Promise<T>): Promise<T | null> {
    try {
      return await command();
    } catch {
      return null;
    }
  }
}
