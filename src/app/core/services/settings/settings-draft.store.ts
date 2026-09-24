import { Injectable, computed, inject, signal } from '@angular/core';
import {
  Refused,
  ShortcutBindingsStore,
  ShortcutConflict,
  conflictsAmong,
  refuseBinding,
} from '@core/services/shortcuts/shortcut-bindings.store';
import { Rebindable } from '@core/services/shortcuts/shortcut.model';
import { AppSettings } from './app-settings.model';
import { SettingsStore } from './settings.store';

/** `null` stages a return to the shipped default, which is not the same as storing it. */
interface StagedBinding {
  readonly action: Rebindable;
  readonly accelerator: string | null;
}

/**
 * What the preferences panel edits, between its controls and the two stores that write.
 *
 * A control writes here, Appliquer and OK push it through, Annuler drops it. The
 * three services that carry a setting to the native side read `SettingsStore`, never
 * this: a half-captured global shortcut must not be live across the whole machine for as
 * long as it takes to finish typing it — which is the argument this layer exists for.
 *
 * It stages **both** paths a preference can take, because the panel edits both: an
 * `AppSettings` key, and a shortcut binding. A draft covering only the first would let
 * OK mean two different things on two pages.
 */
@Injectable({ providedIn: 'root' })
export class SettingsDraftStore {
  private readonly settings = inject(SettingsStore);
  private readonly bindings = inject(ShortcutBindingsStore);

  private readonly staged = signal<Partial<AppSettings>>({});
  private readonly stagedKeys = signal<ReadonlyMap<string, StagedBinding>>(new Map());

  readonly isDirty = computed(() => Object.keys(this.staged()).length > 0 || this.stagedKeys().size > 0);

  /** How many changes are waiting, so the panel can say it rather than merely hint. */
  readonly pendingCount = computed(() => Object.keys(this.staged()).length + this.stagedKeys().size);

  value<K extends keyof AppSettings>(key: K): AppSettings[K] {
    const staged = this.staged()[key];

    return staged === undefined ? this.settings.read(key) : staged;
  }

  /**
   * A value put back to what is stored leaves the draft **clean**: toggling a switch
   * twice must not leave the panel claiming there is something to apply.
   */
  set<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
    const stored = this.settings.read(key);
    this.staged.update((draft) => {
      const next = { ...draft };
      if (value === stored) delete next[key];
      else next[key] = value;
      return next;
    });

    this.settings.preview(this.staged());
  }

  binding(action: Rebindable): string {
    const staged = this.stagedKeys().get(action.id);
    if (staged === undefined) return this.bindings.binding(action);

    return staged.accelerator ?? action.fallback;
  }

  isDefaultBinding(action: Rebindable): boolean {
    return this.binding(action) === action.fallback;
  }

  /** The same rule the store applies, read through what is staged rather than stored. */
  rebind(action: Rebindable, accelerator: string, among: readonly Rebindable[]): Refused | null {
    const refused = refuseBinding(action, accelerator, among, (other) => this.binding(other));
    if (refused) return refused;

    this.stage(action, accelerator);
    return null;
  }

  resetBinding(action: Rebindable): void {
    this.stage(action, null);
  }

  conflicts(among: readonly Rebindable[]): readonly ShortcutConflict[] {
    return conflictsAmong(among, (action) => this.binding(action));
  }

  /** The preview is cleared last, so the written value is already what it falls back to. */
  apply(): void {
    for (const [key, value] of Object.entries(this.staged())) {
      this.settings.write(key as keyof AppSettings, value);
    }

    for (const { action, accelerator } of this.stagedKeys().values()) {
      if (accelerator === null) this.bindings.reset(action);
      else this.bindings.store(action, accelerator);
    }

    this.clear();
  }

  /** Nothing to roll back: a preview never reached the file, and neither did the rest. */
  cancel(): void {
    this.clear();
  }

  private stage(action: Rebindable, accelerator: string | null): void {
    const stored = this.bindings.binding(action);
    this.stagedKeys.update((keys) => {
      const next = new Map(keys);
      if ((accelerator ?? action.fallback) === stored) next.delete(action.id);
      else next.set(action.id, { action, accelerator });
      return next;
    });
  }

  private clear(): void {
    this.staged.set({});
    this.stagedKeys.set(new Map());
    this.settings.preview({});
  }
}
