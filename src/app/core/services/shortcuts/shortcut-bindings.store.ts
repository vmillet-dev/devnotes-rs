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

/** How an action's current keystroke is read — from what is stored, or from what is staged. */
export type BindingReader = (action: Rebindable) => string;

/**
 * Whether a keystroke can be taken, and why not.
 *
 * ⚠️ One rule, two readers: this store answers from what is stored and
 * `SettingsDraftStore` from what is staged, and a check living in only one of them would
 * let the panel offer a key the store is about to refuse.
 */
export function refuseBinding(
  action: Rebindable,
  accelerator: string,
  among: readonly Rebindable[],
  bindingOf: BindingReader,
): Refused | null {
  const legal = isGlobal(action.id) ? isAccelerator(accelerator) : isCanvasAccelerator(accelerator);
  if (!legal) return { kind: 'illegal' };

  const by = among.find((other) => other.id !== action.id && bindingOf(other) === accelerator);

  return by ? { kind: 'taken', by } : null;
}

/**
 * Every keystroke claimed more than once.
 *
 * ⚠️ `refuseBinding` stops one being made, so this answers for what nothing can refuse: a
 * preferences file edited by hand, and a shipped default that lands on a taken key.
 */
export function conflictsAmong(
  among: readonly Rebindable[],
  bindingOf: BindingReader,
): readonly ShortcutConflict[] {
  const claims = new Map<string, Rebindable[]>();
  for (const action of among) {
    const accelerator = bindingOf(action);
    claims.set(accelerator, [...(claims.get(accelerator) ?? []), action]);
  }

  return [...claims]
    .filter(([, actions]) => actions.length > 1)
    .map(([accelerator, actions]) => ({ accelerator, actions }));
}

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
   * ⚠️ `among` is the whole table the panel shows — **both** storage paths, or a key
   * could be taken twice across them.
   */
  rebind(action: Rebindable, accelerator: string, among: readonly Rebindable[]): Refused | null {
    const refused = refuseBinding(action, accelerator, among, (other) => this.binding(other));
    if (refused) return refused;

    this.store(action, accelerator);
    return null;
  }

  /** Writes what `rebind` accepted, or what the draft staged and Appliquer let through. */
  store(action: Rebindable, accelerator: string): void {
    if (isGlobal(action.id)) {
      this.setGlobal(action.id, accelerator);
      return;
    }

    this.preferences.write(preferenceKey(action.id), accelerator);
    this.revision.update((count) => count + 1);
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

  conflicts(among: readonly Rebindable[]): readonly ShortcutConflict[] {
    return conflictsAmong(among, (action) => this.binding(action));
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
