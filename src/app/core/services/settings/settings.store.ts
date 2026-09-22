import { Injectable, Signal, computed, effect, inject, signal } from '@angular/core';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import {
  AppSettings,
  DEFAULT_SETTINGS,
  DENSITIES,
  Density,
  LOCALE_CHOICES,
  LocaleChoice,
  RAIL_WIDTH,
  ResolvedTheme,
  SETTINGS_KEYS,
  THEME_CHOICES,
  ThemeChoice,
} from './app-settings.model';

const DARK_QUERY = '(prefers-color-scheme: dark)';

interface SettingCodec<T> {
  /** `null` rejects the stored value, and the setting keeps its default. */
  parse(stored: string): T | null;
  format(value: T): string;
}

const asBoolean: SettingCodec<boolean> = {
  parse: (stored) => (stored === 'true' ? true : stored === 'false' ? false : null),
  format: String,
};

/** Free text, empty included — which is how "nothing is silenced" is written. */
const asText: SettingCodec<string> = {
  parse: (stored) => stored.trim(),
  format: (value) => value.trim(),
};

/** A blank accelerator would leave the palette unreachable without saying so. */
const asAccelerator: SettingCodec<string> = {
  parse: (stored) => stored.trim() || null,
  format: (value) => value,
};

/** ⚠️ Clamped rather than rejected: a width out of range is a stale file, not a mistake. */
function asPixels(bounds: { readonly min: number; readonly max: number }): SettingCodec<number> {
  return {
    parse: (stored) => {
      const value = Number.parseInt(stored, 10);
      return Number.isNaN(value) ? null : Math.min(bounds.max, Math.max(bounds.min, value));
    },
    format: String,
  };
}

function asOneOf<T extends string>(values: readonly T[]): SettingCodec<T> {
  return {
    parse: (stored) => ((values as readonly string[]).includes(stored) ? (stored as T) : null),
    format: (value) => value,
  };
}

export type SettingSignal<T> = Signal<T> & { write(value: T): void };

/**
 * Writes apply immediately — there is no "OK / Cancel" anywhere in the panel. This
 * store talks to nobody: the native services read these signals and push to Rust,
 * which is what keeps it readable outside Tauri.
 */
@Injectable({ providedIn: 'root' })
export class SettingsStore {
  private readonly preferences = inject(PreferencesService);

  /** ⚠️ Declared before the settings below: class fields initialise in order. */
  private readonly restorers: (() => void)[] = [];

  readonly locale = this.setting('locale', asOneOf(LOCALE_CHOICES));
  readonly theme = this.setting('theme', asOneOf(THEME_CHOICES));
  readonly density = this.setting('density', asOneOf(DENSITIES));
  readonly startWithSystem = this.setting('startWithSystem', asBoolean);
  readonly minimizeToTray = this.setting('minimizeToTray', asBoolean);
  readonly closeToTray = this.setting('closeToTray', asBoolean);
  readonly paletteShortcut = this.setting('paletteShortcut', asAccelerator);
  readonly captureShortcut = this.setting('captureShortcut', asAccelerator);
  readonly newNoteShortcut = this.setting('newNoteShortcut', asAccelerator);
  readonly showLibraryRail = this.setting('showLibraryRail', asBoolean);
  readonly libraryRailWidth = this.setting('libraryRailWidth', asPixels(RAIL_WIDTH));
  readonly showPinnedFirst = this.setting('showPinnedFirst', asBoolean);
  readonly automaticBackups = this.setting('automaticBackups', asBoolean);
  readonly copyConfirmation = this.setting('copyConfirmation', asBoolean);
  readonly updateNotifications = this.setting('updateNotifications', asBoolean);
  readonly skippedUpdate = this.setting('skippedUpdate', asText);

  /** Followed live: a "system" theme must switch without a restart. */
  private readonly systemPrefersDark = signal(false);

  readonly resolvedTheme: Signal<ResolvedTheme> = computed(() => {
    const choice = this.theme();

    return choice === 'system' ? (this.systemPrefersDark() ? 'dark' : 'light') : choice;
  });

  constructor() {
    this.watchSystemTheme();

    // `<html>` carries them: the CSS variables live on `:root`, and a class set any
    // lower would not reach them.
    effect(() => {
      const root = document.documentElement;
      root.dataset['theme'] = this.resolvedTheme();
      root.dataset['density'] = this.density();
    });
  }

  /** ⚠️ Called after `PreferencesService.hydrate()`: before it, every read yields a default. */
  restore(): void {
    for (const restore of this.restorers) {
      restore();
    }
  }

  setLocale(locale: LocaleChoice): void {
    this.locale.write(locale);
  }

  setTheme(theme: ThemeChoice): void {
    this.theme.write(theme);
  }

  setDensity(density: Density): void {
    this.density.write(density);
  }

  setStartWithSystem(enabled: boolean): void {
    this.startWithSystem.write(enabled);
  }

  setMinimizeToTray(enabled: boolean): void {
    this.minimizeToTray.write(enabled);
  }

  setCloseToTray(enabled: boolean): void {
    this.closeToTray.write(enabled);
  }

  setPaletteShortcut(accelerator: string): void {
    this.paletteShortcut.write(accelerator);
  }

  setShowPinnedFirst(enabled: boolean): void {
    this.showPinnedFirst.write(enabled);
  }

  setAutomaticBackups(enabled: boolean): void {
    this.automaticBackups.write(enabled);
  }

  setCopyConfirmation(enabled: boolean): void {
    this.copyConfirmation.write(enabled);
  }

  /** Turning the prompt back on also forgets the version that was skipped. */
  setUpdateNotifications(enabled: boolean): void {
    this.updateNotifications.write(enabled);
    if (enabled) {
      this.skippedUpdate.write('');
    }
  }

  /** `''` forgets the skip. */
  setSkippedUpdate(version: string): void {
    this.skippedUpdate.write(version);
  }

  /** One setting: its signal, its restore step and its write-through, in one declaration. */
  private setting<K extends keyof AppSettings>(
    key: K,
    codec: SettingCodec<AppSettings[K]>,
  ): SettingSignal<AppSettings[K]> {
    const current = signal(DEFAULT_SETTINGS[key]);

    this.restorers.push(() => {
      const stored = this.preferences.read(SETTINGS_KEYS[key]);
      // Nothing stored is not "false": it is the setting's default.
      const parsed = stored === null ? null : codec.parse(stored);
      if (parsed !== null) {
        current.set(parsed);
      }
    });

    const write = (value: AppSettings[K]): void => {
      // Through the codec both ways, so a value the restore path would reject is
      // refused on the way in too.
      const accepted = codec.parse(codec.format(value));
      if (accepted === null) return;

      current.set(accepted);
      this.preferences.write(SETTINGS_KEYS[key], codec.format(accepted));
    };

    return Object.assign(current.asReadonly(), { write });
  }

  private watchSystemTheme(): void {
    // `matchMedia` is missing from some test environments; without it the "system"
    // theme falls back to light, which the CSS default assumes.
    const media = window.matchMedia?.(DARK_QUERY);
    if (!media) return;

    this.systemPrefersDark.set(media.matches);
    media.addEventListener('change', (event) => {
      this.systemPrefersDark.set(event.matches);
    });
  }
}
