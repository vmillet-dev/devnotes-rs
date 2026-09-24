import { APP_LOCALES } from '@core/services/i18n/locale.model';
import { DEFAULT_SHORTCUTS } from '@core/services/shortcuts/shortcut.model';

/**
 * A preference is stored per key rather than as one serialised object, so a setting
 * added later cannot make a file written by the previous version unreadable.
 */

export const LOCALE_CHOICES = ['system', ...APP_LOCALES] as const;
export type LocaleChoice = (typeof LOCALE_CHOICES)[number];

export const THEME_CHOICES = ['system', 'dark', 'light'] as const;
export type ThemeChoice = (typeof THEME_CHOICES)[number];

export type ResolvedTheme = Exclude<ThemeChoice, 'system'>;

/** Spacing only: a density that shrank the typography would be a zoom. */
export const DENSITIES = ['comfortable', 'compact'] as const;
export type Density = (typeof DENSITIES)[number];

/**
 * The floor is the tree's own, not the panels': `--editor-min-width` is what lets
 * `space-editor` and `folder-editor` follow the rail rather than hold it open at their
 * 240px. It opens at that floor — the width is remembered, so the default is a first
 * launch and nothing else.
 */
export const RAIL_WIDTH = { min: 160, max: 520, default: 160 } as const;

export interface AppSettings {
  readonly locale: LocaleChoice;
  readonly theme: ThemeChoice;
  readonly density: Density;
  readonly startWithSystem: boolean;
  readonly minimizeToTray: boolean;
  readonly closeToTray: boolean;
  /**
   * The three **global** accelerators, stored here because the native side takes them
   * as a block. The canvas keys are one preference each and belong to
   * `ShortcutBindingsStore`, which is what reads both paths as one table.
   */
  readonly paletteShortcut: string;
  readonly captureShortcut: string;
  readonly newNoteShortcut: string;
  /** The library rail, remembered like the window's own geometry rather than reset on launch. */
  readonly showLibraryRail: boolean;
  /** Its width, dragged from its edge and clamped on the way in and out. */
  readonly libraryRailWidth: number;
  readonly showPinnedFirst: boolean;
  /** Read by Rust at launch, before the front end exists: `backup::wanted`. */
  readonly automaticBackups: boolean;
  /** Whether that copy carries `attachments/`, read by Rust the same way. */
  readonly backupAttachments: boolean;
  readonly copyConfirmation: boolean;
  readonly updateNotifications: boolean;
  /**
   * The version the user said "later" to, or `''`. A version and not a boolean:
   * remembering "no" would silence the release after it too.
   */
  readonly skippedUpdate: string;
}

/** `closeToTray` is `true`, and the native side carries the same default. */
export const DEFAULT_SETTINGS: AppSettings = {
  locale: 'system',
  theme: 'system',
  density: 'comfortable',
  startWithSystem: false,
  minimizeToTray: false,
  closeToTray: true,
  paletteShortcut: DEFAULT_SHORTCUTS.palette,
  captureShortcut: DEFAULT_SHORTCUTS.capture,
  newNoteShortcut: DEFAULT_SHORTCUTS.newNote,
  showLibraryRail: true,
  libraryRailWidth: RAIL_WIDTH.default,
  showPinnedFirst: true,
  automaticBackups: true,
  backupAttachments: true,
  copyConfirmation: true,
  updateNotifications: true,
  skippedUpdate: '',
};

/** Derived rather than hand-written: the key is the field name, prefixed. */
export const SETTINGS_KEYS = Object.fromEntries(
  Object.keys(DEFAULT_SETTINGS).map((field) => [field, `devnotes.${field}`]),
) as Readonly<Record<keyof AppSettings, string>>;
