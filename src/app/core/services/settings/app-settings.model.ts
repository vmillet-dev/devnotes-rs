import type { NoteKind } from '@core/model/checklist.model';
import { AREAS, Area } from '@core/services/areas/area.model';
import { APP_LOCALES } from '@core/services/i18n/locale.model';
import { RecentTool } from '@core/services/tools/tool.model';
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

/** What Tab inserts in the code field; `language` follows the note's. */
export const INDENT_CHOICES = ['language', 'two-spaces', 'four-spaces', 'tab'] as const;
export type IndentChoice = (typeof INDENT_CHOICES)[number];

/** What « Nouvelle note », the « + » card and the global shortcut open; the menu still picks any. */
export const NOTE_KIND_CHOICES = ['note', 'snippet', 'checklist'] as const satisfies readonly NoteKind[];

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
  readonly codeIndent: IndentChoice;
  readonly defaultNoteKind: NoteKind;
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
  /** Which area shows — Notes or Outils — and the one the window reopens on. */
  readonly area: Area;
  /** The tools opened last, newest first: which and when, nothing typed into them. */
  readonly recentTools: readonly RecentTool[];
  readonly showPinnedFirst: boolean;
  /** Read by Rust at launch, before the front end exists: `backup::wanted`. */
  readonly automaticBackups: boolean;
  readonly copyConfirmation: boolean;
  readonly updateNotifications: boolean;
  /**
   * The version the user said "later" to, or `''`. A version and not a boolean:
   * remembering "no" would silence the release after it too.
   */
  readonly skippedUpdate: string;
  /** Until a first Note is made here, the new-note menu marks the kind as new. */
  readonly noteKindTried: boolean;
  /**
   * The zones tool's list, the machine's own aside. ⚠️ The one thing a tool keeps on disk: a
   * list of zones is a preference, not something typed.
   */
  readonly timeZones: readonly string[];
}

/** Zones beyond the machine's, read back from a file nobody checks. */
export const MAX_TIME_ZONES = 24;

/** `closeToTray` is `true`, and the native side carries the same default. */
export const DEFAULT_SETTINGS: AppSettings = {
  locale: 'system',
  theme: 'system',
  density: 'comfortable',
  codeIndent: 'language',
  defaultNoteKind: 'snippet',
  startWithSystem: false,
  minimizeToTray: false,
  closeToTray: true,
  paletteShortcut: DEFAULT_SHORTCUTS.palette,
  captureShortcut: DEFAULT_SHORTCUTS.capture,
  newNoteShortcut: DEFAULT_SHORTCUTS.newNote,
  showLibraryRail: true,
  libraryRailWidth: RAIL_WIDTH.default,
  area: AREAS[0],
  recentTools: [],
  showPinnedFirst: true,
  automaticBackups: true,
  copyConfirmation: true,
  updateNotifications: true,
  skippedUpdate: '',
  noteKindTried: false,
  timeZones: ['UTC', 'America/New_York', 'Asia/Tokyo'],
};

/** Derived rather than hand-written: the key is the field name, prefixed. */
export const SETTINGS_KEYS = Object.fromEntries(
  Object.keys(DEFAULT_SETTINGS).map((field) => [field, `devnotes.${field}`]),
) as Readonly<Record<keyof AppSettings, string>>;
