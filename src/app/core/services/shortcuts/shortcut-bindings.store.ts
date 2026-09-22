import { Injectable, inject, signal } from '@angular/core';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { Rebindable, isAccelerator, isCanvasAccelerator } from './shortcut.model';

/** The three the native side takes as a block; everything else is one key per action. */
type GlobalId = 'palette' | 'capture' | 'newNote';

const GLOBAL_IDS: readonly string[] = ['palette', 'capture', 'newNote'];

function isGlobal(id: string): id is GlobalId {
  return GLOBAL_IDS.includes(id);
}

function preferenceKey(id: string): string {
  return `devnotes.shortcut.${id}`;
}

/** Two actions on the same keystroke, named by the accelerator they both claim. */
export interface ShortcutConflict {
  readonly accelerator: string;
  readonly actions: readonly Rebindable[];
}

/**
 * Why a keystroke was not taken. ⚠️ A reason and not a boolean: the panel has to say
 * which of the two it was, and deciding it twice is how the field and the store drift.
 */
export type Refused = { readonly kind: 'illegal' } | { readonly kind: 'taken'; readonly by: Rebindable };

/**
 * Which accelerator each rebindable action answers to.
 *
 * ⚠️ **Two storage paths, deliberately.** The three global ones live in `AppSettings`,
 * because Rust registers them with the operating system and takes them as a block; a
 * canvas key is one preference of its own, written straight through
 * `PreferencesService`. This store is what reads both as one table — which is also what
 * makes a conflict *between* the two paths something the panel can say out loud.
 *
 * It holds no list of actions: a `Rebindable` carries its own fallback, so the canvas
 * table stays the only place a canvas key is declared.
 */
@Injectable({ providedIn: 'root' })
export class ShortcutBindingsStore {
  private readonly preferences = inject(PreferencesService);
  private readonly settings = inject(SettingsStore);

  /** ⚠️ `PreferencesService` is a synchronous cache and not reactive: this is what makes
   *  a rebind redraw the caps that are on screen. */
  private readonly revision = signal(0);

  /** The accelerator an action answers to right now. Reactive, so a template may read it. */
  binding(action: Rebindable): string {
    this.revision();
    if (isGlobal(action.id)) return this.globalBinding(action.id);

    const stored = this.preferences.read(preferenceKey(action.id));

    return stored !== null && isCanvasAccelerator(stored) ? stored : action.fallback;
  }

  isDefault(action: Rebindable): boolean {
    return this.binding(action) === action.fallback;
  }

  /**
   * Moves an action onto a keystroke, or says why it could not.
   *
   * ⚠️ Refused when another action already answers to it: the second one would be
   * unreachable and nothing on screen would say which. `among` is the whole table the
   * panel shows — **both** storage paths, or a key could be taken twice across them.
   */
  rebind(action: Rebindable, accelerator: string, among: readonly Rebindable[]): Refused | null {
    const legal = isGlobal(action.id) ? isAccelerator(accelerator) : isCanvasAccelerator(accelerator);
    if (!legal) return { kind: 'illegal' };

    const by = this.claimedBy(accelerator, action, among);
    if (by) return { kind: 'taken', by };

    if (isGlobal(action.id)) {
      this.setGlobal(action.id, accelerator);
      return null;
    }

    this.preferences.write(preferenceKey(action.id), accelerator);
    this.revision.update((count) => count + 1);
    return null;
  }

  reset(action: Rebindable): void {
    if (isGlobal(action.id)) {
      this.setGlobal(action.id, action.fallback);
      return;
    }

    // Forgotten rather than written back: the default is free to move between versions.
    this.preferences.forget(preferenceKey(action.id));
    this.revision.update((count) => count + 1);
  }

  private claimedBy(
    accelerator: string,
    except: Rebindable,
    among: readonly Rebindable[],
  ): Rebindable | null {
    return among.find((other) => other.id !== except.id && this.binding(other) === accelerator) ?? null;
  }

  /**
   * Every keystroke claimed more than once.
   *
   * ⚠️ `rebind` refuses to make one, so this answers for what it cannot refuse: a
   * preferences file edited by hand, and a shipped default that moves onto a key the
   * user had already taken.
   */
  conflicts(among: readonly Rebindable[]): readonly ShortcutConflict[] {
    const claims = new Map<string, Rebindable[]>();
    for (const action of among) {
      const accelerator = this.binding(action);
      claims.set(accelerator, [...(claims.get(accelerator) ?? []), action]);
    }

    return [...claims]
      .filter(([, actions]) => actions.length > 1)
      .map(([accelerator, actions]) => ({ accelerator, actions }));
  }

  private globalBinding(id: GlobalId): string {
    switch (id) {
      case 'palette':
        return this.settings.paletteShortcut();
      case 'capture':
        return this.settings.captureShortcut();
      case 'newNote':
        return this.settings.newNoteShortcut();
    }
  }

  private setGlobal(id: GlobalId, accelerator: string): void {
    switch (id) {
      case 'palette':
        this.settings.paletteShortcut.write(accelerator);
        return;
      case 'capture':
        this.settings.captureShortcut.write(accelerator);
        return;
      case 'newNote':
        this.settings.newNoteShortcut.write(accelerator);
    }
  }
}
