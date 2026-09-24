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

/** Clamped rather than rejected: a width out of range is a stale file, not a mistake. */
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
 * A write here is final: it reaches `PreferencesService` and the services that push to
 * the native side read these signals. The preferences panel therefore edits
 * `SettingsDraftStore` and writes through this one on Appliquer or OK.
 *
 * This store talks to nobody, which is what keeps it readable outside Tauri.
 */
@Injectable({ providedIn: 'root' })
export class SettingsStore {
  private readonly preferences = inject(PreferencesService);

  /** ⚠️ Both declared before the settings below: class fields initialise in order. */
  private readonly restorers: (() => void)[] = [];
  /**
   * Widened in the map and narrowed back by the two accessors below: eighteen signals
   * of eighteen types share no member type, and `key` is what proves each one.
   */
  private readonly byKey = new Map<keyof AppSettings, SettingSignal<AppSettings[keyof AppSettings]>>();

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
  readonly backupAttachments = this.setting('backupAttachments', asBoolean);
  readonly copyConfirmation = this.setting('copyConfirmation', asBoolean);
  readonly updateNotifications = this.setting('updateNotifications', asBoolean);
  readonly skippedUpdate = this.setting('skippedUpdate', asText);

  /** Followed live: a "system" theme must switch without a restart. */
  private readonly systemPrefersDark = signal(false);

  /**
   * A preview is **not** a write. Nobody picks a theme without seeing it, so the panel
   * shows the one being chosen while nothing has reached the file yet — and Annuler puts
   * the previous appearance back by clearing this, with nothing to roll back.
   */
  private readonly previewed = signal<{
    readonly theme: ThemeChoice | null;
    readonly density: Density | null;
  }>({ theme: null, density: null });

  /** What is on screen, which is the preview where there is one and the setting otherwise. */
  readonly shownTheme = computed<ThemeChoice>(() => this.previewed().theme ?? this.theme());
  readonly shownDensity = computed<Density>(() => this.previewed().density ?? this.density());

  readonly resolvedTheme: Signal<ResolvedTheme> = computed(() => {
    const choice = this.shownTheme();

    return choice === 'system' ? (this.systemPrefersDark() ? 'dark' : 'light') : choice;
  });

  constructor() {
    this.watchSystemTheme();

    // `<html>` carries them: the CSS variables live on `:root`, and a class set any
    // lower would not reach them.
    effect(() => {
      const root = document.documentElement;
      root.dataset['theme'] = this.resolvedTheme();
      root.dataset['density'] = this.shownDensity();
    });
  }

  /**
   * Only these two. Everything else a draft can hold has no preview that means
   * anything, and a half-captured global shortcut being live across the whole machine is
   * one of the arguments for not applying as you type in the first place.
   */
  preview(draft: Partial<AppSettings>): void {
    this.previewed.set({ theme: draft.theme ?? null, density: draft.density ?? null });
  }

  /**
   * Addressed by key, so the draft reads and writes any setting without a switch over
   * eighteen of them — and so adding a setting stays the one line it is below.
   */
  read<K extends keyof AppSettings>(key: K): AppSettings[K] {
    return (this.byKey.get(key) as SettingSignal<AppSettings[K]>)();
  }

  write<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
    (this.byKey.get(key) as SettingSignal<AppSettings[K]>).write(value);
  }

  /** Called after `PreferencesService.hydrate()`: before it, every read yields a default. */
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

  setCloseToTray(enabled: boolean): void {
    this.closeToTray.write(enabled);
  }

  setPaletteShortcut(accelerator: string): void {
    this.paletteShortcut.write(accelerator);
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

    const setting = Object.assign(current.asReadonly(), { write });
    this.byKey.set(key, setting);

    return setting;
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
