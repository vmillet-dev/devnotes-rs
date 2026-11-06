//! The application's settings, read from its preferences under their reference keys
//! (`devnotes.<setting>`), and the draft the preferences panel edits. A setting joins this
//! struct with the screen that reads it; the file keeps the others untouched.

use engine::notes::kind::NoteKind;
use engine::notes::language::Language;

use crate::state::Area;
use crate::theme::{Appearance, Density};

use super::preferences::Preferences;
use super::shortcuts::{self, Bindings};

/// The language: the system's, or one chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LocaleChoice {
    #[default]
    System,
    Fr,
    En,
}

impl LocaleChoice {
    pub fn language(self) -> i18n::Language {
        match self {
            Self::System => i18n::Language::of_system(),
            Self::Fr => i18n::Language::Fr,
            Self::En => i18n::Language::En,
        }
    }
}

/// What Tab inserts in the code field: the language's habit, or one chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IndentChoice {
    #[default]
    Language,
    TwoSpaces,
    FourSpaces,
    Tab,
}

impl IndentChoice {
    pub fn unit(self, language: Language) -> &'static str {
        match self {
            Self::Language => crate::notes::language::indent_unit(language),
            Self::TwoSpaces => "  ",
            Self::FourSpaces => "    ",
            Self::Tab => "\t",
        }
    }
}

/// How a sheet writes a date.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CalcDate {
    Numeric,
    #[default]
    Short,
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is its own display choice"
)]
pub struct Settings {
    pub locale: LocaleChoice,
    pub theme: Appearance,
    pub density: Density,
    /// The area on screen at the last exit, shown again at the next launch.
    pub area: Area,
    /// The library rail's, in points, within `RAIL_WIDTH`.
    pub rail_width: u16,
    pub rail_shown: bool,
    /// Display preferences of the editor, not of a note: they outlive moving to the next one.
    pub editor_fullscreen: bool,
    pub editor_header_folded: bool,
    pub editor_fields_open: bool,
    /// The request's share of the height it has with its answer, in tenths of a percent; `0`
    /// leaves it to what the request holds.
    pub http_request_share: u16,
    /// The history's list, in points, within `HISTORY_LIST_WIDTH`.
    pub history_list_width: u16,
    pub code_indent: IndentChoice,
    /// Long lines scroll sideways unless asked to wrap.
    pub code_wrap: bool,
    /// What « Nouvelle note » and the global shortcut open; the menu still picks any.
    pub default_note_kind: NoteKind,
    pub start_with_system: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub show_pinned_first: bool,
    /// Read by the core itself at launch.
    pub automatic_backups: bool,
    pub copy_confirmation: bool,
    pub update_notifications: bool,
    /// The version the person said « later » to, or empty: a version and not a boolean, since
    /// remembering « no » would silence the release after it too.
    pub skipped_update: String,
    /// How a sheet shows its results, never how they are computed: `None` reads the value.
    pub calc_decimals: Option<u8>,
    /// The locale a sheet writes its numbers in; `language` follows the one on screen.
    pub calc_number_format: String,
    pub calc_date_format: CalcDate,
    /// The keys moved from where they shipped.
    pub bindings: Bindings,
}

/// The reference's `RAIL_WIDTH`.
pub const RAIL_WIDTH: (u16, u16) = (160, 520);
/// The reference's `HISTORY_LIST_WIDTH`.
pub const HISTORY_LIST_WIDTH: (u16, u16) = (280, 760);

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: LocaleChoice::default(),
            theme: Appearance::default(),
            density: Density::default(),
            area: Area::default(),
            rail_width: RAIL_WIDTH.0,
            rail_shown: true,
            editor_fullscreen: false,
            editor_header_folded: false,
            editor_fields_open: true,
            http_request_share: 0,
            history_list_width: 380,
            code_indent: IndentChoice::default(),
            code_wrap: false,
            default_note_kind: NoteKind::Snippet,
            start_with_system: false,
            minimize_to_tray: false,
            close_to_tray: true,
            show_pinned_first: true,
            automatic_backups: true,
            copy_confirmation: true,
            update_notifications: true,
            skipped_update: String::new(),
            calc_decimals: None,
            calc_number_format: CALC_LANGUAGE.to_owned(),
            calc_date_format: CalcDate::default(),
            bindings: Bindings::default(),
        }
    }
}

const LOCALE: &str = "devnotes.locale";
const THEME: &str = "devnotes.theme";
const DENSITY: &str = "devnotes.density";
const AREA: &str = "devnotes.area";
const RAIL: &str = "devnotes.libraryRailWidth";
const RAIL_SHOWN: &str = "devnotes.showLibraryRail";
const EDITOR_FULLSCREEN: &str = "devnotes.editorFullscreen";
const EDITOR_HEADER_FOLDED: &str = "devnotes.editorHeaderFolded";
const EDITOR_FIELDS_OPEN: &str = "devnotes.editorFieldsPanel";
const HTTP_REQUEST_SHARE: &str = "devnotes.httpRequestShare";
const HISTORY_LIST_WIDTH_KEY: &str = "devnotes.httpHistoryListWidth";
const CODE_INDENT: &str = "devnotes.codeIndent";
const CODE_WRAP: &str = "devnotes.codeWrap";
const DEFAULT_NOTE_KIND: &str = "devnotes.defaultNoteKind";
const START_WITH_SYSTEM: &str = "devnotes.startWithSystem";
const MINIMIZE_TO_TRAY: &str = "devnotes.minimizeToTray";
const CLOSE_TO_TRAY: &str = "devnotes.closeToTray";
const SHOW_PINNED_FIRST: &str = "devnotes.showPinnedFirst";
const AUTOMATIC_BACKUPS: &str = "devnotes.automaticBackups";
const COPY_CONFIRMATION: &str = "devnotes.copyConfirmation";
const UPDATE_NOTIFICATIONS: &str = "devnotes.updateNotifications";
const SKIPPED_UPDATE: &str = "devnotes.skippedUpdate";
const CALC_DECIMALS: &str = "devnotes.calcDecimals";
const CALC_NUMBER_FORMAT: &str = "devnotes.calcNumberFormat";
const CALC_DATE_FORMAT: &str = "devnotes.calcDateFormat";

/// The number format that follows the language on screen.
pub const CALC_LANGUAGE: &str = "language";

/// The share's bounds, in tenths of a percent: neither pane goes out of reach.
pub const HTTP_SHARE: (u16, u16) = (50, 950);

impl Settings {
    /// A value this version does not know is the default, never an error.
    pub fn read(preferences: &Preferences) -> Self {
        Self {
            locale: chosen(preferences, LOCALE, &LOCALES).unwrap_or_default(),
            theme: chosen(preferences, THEME, &THEMES).unwrap_or_default(),
            density: chosen(preferences, DENSITY, &DENSITIES).unwrap_or_default(),
            area: chosen(preferences, AREA, &AREAS).unwrap_or_default(),
            rail_width: preferences
                .get(RAIL)
                .and_then(|width| width.parse::<u16>().ok())
                .map_or(RAIL_WIDTH.0, |width| {
                    width.clamp(RAIL_WIDTH.0, RAIL_WIDTH.1)
                }),
            rail_shown: preferences.get(RAIL_SHOWN) != Some("false"),
            editor_fullscreen: preferences.get(EDITOR_FULLSCREEN) == Some("true"),
            editor_header_folded: preferences.get(EDITOR_HEADER_FOLDED) == Some("true"),
            editor_fields_open: preferences.get(EDITOR_FIELDS_OPEN) != Some("false"),
            http_request_share: read_share(preferences.get(HTTP_REQUEST_SHARE)),
            history_list_width: preferences
                .get(HISTORY_LIST_WIDTH_KEY)
                .and_then(|width| width.parse().ok())
                .unwrap_or(380),
            code_indent: chosen(preferences, CODE_INDENT, &INDENTS).unwrap_or_default(),
            code_wrap: preferences.get(CODE_WRAP) == Some("true"),
            default_note_kind: chosen(preferences, DEFAULT_NOTE_KIND, &NOTE_KINDS)
                .unwrap_or_default(),
            start_with_system: preferences.get(START_WITH_SYSTEM) == Some("true"),
            minimize_to_tray: preferences.get(MINIMIZE_TO_TRAY) == Some("true"),
            close_to_tray: preferences.get(CLOSE_TO_TRAY) != Some("false"),
            show_pinned_first: preferences.get(SHOW_PINNED_FIRST) != Some("false"),
            automatic_backups: preferences.get(AUTOMATIC_BACKUPS) != Some("false"),
            copy_confirmation: preferences.get(COPY_CONFIRMATION) != Some("false"),
            update_notifications: preferences.get(UPDATE_NOTIFICATIONS) != Some("false"),
            skipped_update: preferences
                .get(SKIPPED_UPDATE)
                .unwrap_or_default()
                .to_owned(),
            calc_decimals: preferences
                .get(CALC_DECIMALS)
                .and_then(|places| places.parse().ok()),
            calc_number_format: preferences
                .get(CALC_NUMBER_FORMAT)
                .unwrap_or(CALC_LANGUAGE)
                .to_owned(),
            calc_date_format: chosen(preferences, CALC_DATE_FORMAT, &CALC_DATES)
                .unwrap_or_default(),
            bindings: read_bindings(preferences),
        }
    }

    /// Into `preferences`, every other key left as it was.
    pub fn write(&self, preferences: &mut Preferences) {
        preferences.set(LOCALE, name_of(&LOCALES, self.locale));
        preferences.set(THEME, name_of(&THEMES, self.theme));
        preferences.set(DENSITY, name_of(&DENSITIES, self.density));
        preferences.set(AREA, name_of(&AREAS, self.area));
        preferences.set(RAIL, &self.rail_width.to_string());
        preferences.set(RAIL_SHOWN, flag(self.rail_shown));
        preferences.set(EDITOR_FULLSCREEN, flag(self.editor_fullscreen));
        preferences.set(EDITOR_HEADER_FOLDED, flag(self.editor_header_folded));
        preferences.set(EDITOR_FIELDS_OPEN, flag(self.editor_fields_open));
        preferences.set(HISTORY_LIST_WIDTH_KEY, &self.history_list_width.to_string());
        let share = f64::from(self.http_request_share) / 10.0;
        preferences.set(HTTP_REQUEST_SHARE, &share.to_string());
        preferences.set(CODE_INDENT, name_of(&INDENTS, self.code_indent));
        preferences.set(CODE_WRAP, flag(self.code_wrap));
        preferences.set(
            DEFAULT_NOTE_KIND,
            name_of(&NOTE_KINDS, self.default_note_kind),
        );
        preferences.set(START_WITH_SYSTEM, flag(self.start_with_system));
        preferences.set(MINIMIZE_TO_TRAY, flag(self.minimize_to_tray));
        preferences.set(CLOSE_TO_TRAY, flag(self.close_to_tray));
        preferences.set(SHOW_PINNED_FIRST, flag(self.show_pinned_first));
        preferences.set(AUTOMATIC_BACKUPS, flag(self.automatic_backups));
        preferences.set(COPY_CONFIRMATION, flag(self.copy_confirmation));
        preferences.set(UPDATE_NOTIFICATIONS, flag(self.update_notifications));
        preferences.set(SKIPPED_UPDATE, &self.skipped_update);
        let places = self
            .calc_decimals
            .map_or_else(|| "auto".to_owned(), |places| places.to_string());
        preferences.set(CALC_DECIMALS, &places);
        preferences.set(CALC_NUMBER_FORMAT, &self.calc_number_format);
        preferences.set(
            CALC_DATE_FORMAT,
            name_of(&CALC_DATES, self.calc_date_format),
        );
        for action in shortcuts::every() {
            let key = shortcuts::preference_key(action);
            if self.bindings.is_default(action) {
                preferences.remove(&key);
            } else {
                preferences.set(&key, self.bindings.of(action));
            }
        }
    }
}

fn read_bindings(preferences: &Preferences) -> Bindings {
    let mut bindings = Bindings::default();
    for action in shortcuts::every() {
        if let Some(stored) = preferences.get(&shortcuts::preference_key(action)) {
            bindings.set(action, stored.to_owned());
        }
    }
    bindings
}

/// A percent written by the reference (`42.3`), in tenths; out of bounds, or not a number, is
/// automatic.
fn read_share(stored: Option<&str>) -> u16 {
    let Some(percent) = stored.and_then(|stored| stored.parse::<f64>().ok()) else {
        return 0;
    };
    let tenths = (percent * 10.0).round();
    if tenths < f64::from(HTTP_SHARE.0) || tenths > f64::from(HTTP_SHARE.1) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "within HTTP_SHARE"
    )]
    let tenths = tenths as u16;
    tenths
}

fn flag(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

fn chosen<T: Copy>(preferences: &Preferences, key: &str, choices: &[(&str, T)]) -> Option<T> {
    let stored = preferences.get(key)?;
    choices
        .iter()
        .find(|(name, _)| *name == stored)
        .map(|(_, value)| *value)
}

fn name_of<T: PartialEq + Copy>(choices: &[(&'static str, T)], value: T) -> &'static str {
    choices
        .iter()
        .find(|(_, choice)| *choice == value)
        .map_or("", |(name, _)| *name)
}

const LOCALES: [(&str, LocaleChoice); 3] = [
    ("system", LocaleChoice::System),
    ("fr", LocaleChoice::Fr),
    ("en", LocaleChoice::En),
];
const THEMES: [(&str, Appearance); 3] = [
    ("system", Appearance::System),
    ("dark", Appearance::Dark),
    ("light", Appearance::Light),
];
const DENSITIES: [(&str, Density); 2] = [
    ("comfortable", Density::Comfortable),
    ("compact", Density::Compact),
];
const AREAS: [(&str, Area); 3] = [
    ("notes", Area::Notes),
    ("tools", Area::Tools),
    ("http", Area::Http),
];
const INDENTS: [(&str, IndentChoice); 4] = [
    ("language", IndentChoice::Language),
    ("two-spaces", IndentChoice::TwoSpaces),
    ("four-spaces", IndentChoice::FourSpaces),
    ("tab", IndentChoice::Tab),
];
const NOTE_KINDS: [(&str, NoteKind); 4] = [
    ("note", NoteKind::Note),
    ("snippet", NoteKind::Snippet),
    ("checklist", NoteKind::Checklist),
    ("calc", NoteKind::Calc),
];
const CALC_DATES: [(&str, CalcDate); 3] = [
    ("numeric", CalcDate::Numeric),
    ("short", CalcDate::Short),
    ("long", CalcDate::Long),
];

/// What the preferences panel edits before « Apply ». Only the theme and the density are seen
/// at once, as a preview; the rest waits for the button, and the native side reads the
/// applied settings, never the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDraft {
    applied: Settings,
    draft: Settings,
}

impl SettingsDraft {
    pub fn open(applied: Settings) -> Self {
        Self {
            draft: applied.clone(),
            applied,
        }
    }

    pub fn edit(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.draft);
    }

    pub fn draft(&self) -> &Settings {
        &self.draft
    }

    /// How many settings wait for « Apply », one key each in the file.
    pub fn pending(&self) -> u32 {
        let (mut applied, mut draft) = (Preferences::default(), Preferences::default());
        self.applied.write(&mut applied);
        self.draft.write(&mut draft);
        applied.differences(&draft)
    }

    /// What the window shows meanwhile: the draft's look over the applied rest.
    pub fn previewed(&self) -> Settings {
        Settings {
            theme: self.draft.theme,
            density: self.draft.density,
            ..self.applied.clone()
        }
    }

    pub fn is_changed(&self) -> bool {
        self.draft != self.applied
    }

    /// The settings to adopt; closing without applying is dropping the draft.
    pub fn apply(&mut self) -> Settings {
        self.applied = self.draft.clone();
        self.draft.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved_key() -> Bindings {
        let mut bindings = Bindings::default();
        bindings.set(shortcuts::ACTIONS[9], "J".to_owned());
        bindings
    }

    #[test]
    fn settings_round_trip_under_the_reference_keys() {
        let mut preferences = Preferences::default();
        let chosen = Settings {
            locale: LocaleChoice::Fr,
            theme: Appearance::Light,
            density: Density::Compact,
            area: Area::Http,
            rail_width: 240,
            rail_shown: false,
            editor_fullscreen: true,
            editor_header_folded: true,
            editor_fields_open: false,
            http_request_share: 423,
            history_list_width: 500,
            code_indent: IndentChoice::Tab,
            code_wrap: true,
            default_note_kind: NoteKind::Calc,
            start_with_system: true,
            minimize_to_tray: true,
            close_to_tray: false,
            show_pinned_first: false,
            automatic_backups: false,
            copy_confirmation: false,
            update_notifications: false,
            skipped_update: "0.25.1".to_owned(),
            calc_decimals: Some(3),
            calc_number_format: "de-DE".to_owned(),
            calc_date_format: CalcDate::Long,
            bindings: moved_key(),
        };

        chosen.write(&mut preferences);

        assert_eq!(preferences.get("devnotes.theme"), Some("light"));
        assert_eq!(preferences.get("devnotes.httpRequestShare"), Some("42.3"));
        assert_eq!(preferences.get("devnotes.area"), Some("http"));
        assert_eq!(preferences.get("devnotes.closeToTray"), Some("false"));
        assert_eq!(preferences.get("devnotes.calcDecimals"), Some("3"));
        assert_eq!(preferences.get("devnotes.shortcut.canvas.pin"), Some("J"));
        assert_eq!(preferences.get("devnotes.shortcut.canvas.copy"), None);
        assert_eq!(Settings::read(&preferences), chosen);
    }

    #[test]
    fn an_unknown_value_is_the_default() {
        let mut preferences = Preferences::default();
        preferences.set("devnotes.theme", "sepia");

        assert_eq!(Settings::read(&preferences).theme, Appearance::System);
    }

    #[test]
    fn the_draft_previews_the_look_and_keeps_the_rest_for_apply() {
        let mut draft = SettingsDraft::open(Settings::default());

        draft.edit(|settings| {
            settings.theme = Appearance::Light;
            settings.locale = LocaleChoice::En;
        });

        assert_eq!(draft.previewed().theme, Appearance::Light);
        assert_eq!(draft.previewed().locale, LocaleChoice::System);
        assert!(draft.is_changed());
        assert_eq!(draft.pending(), 2);
        assert_eq!(draft.apply().locale, LocaleChoice::En);
        assert!(!draft.is_changed());
    }

    #[test]
    fn the_defaults_of_the_reference_read_from_an_empty_file() {
        let read = Settings::read(&Preferences::default());

        assert_eq!(read, Settings::default());
        assert!(read.close_to_tray && read.automatic_backups && read.copy_confirmation);
    }
}
