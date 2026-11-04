//! The application's settings, read from its preferences under their reference keys
//! (`devnotes.<setting>`), and the draft the preferences panel edits. A setting joins this
//! struct with the screen that reads it; the file keeps the others untouched.

use crate::state::Area;
use crate::theme::{Appearance, Density};

use super::preferences::Preferences;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// The reference's `RAIL_WIDTH`.
pub const RAIL_WIDTH: (u16, u16) = (160, 520);

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
    }
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

/// What the preferences panel edits before « Apply ». Only the theme and the density are seen
/// at once, as a preview; the rest waits for the button, and the native side reads the
/// applied settings, never the draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsDraft {
    applied: Settings,
    draft: Settings,
}

impl SettingsDraft {
    pub fn open(applied: Settings) -> Self {
        Self {
            applied,
            draft: applied,
        }
    }

    pub fn edit(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.draft);
    }

    /// What the window shows meanwhile: the draft's look over the applied rest.
    pub fn previewed(&self) -> Settings {
        Settings {
            theme: self.draft.theme,
            density: self.draft.density,
            ..self.applied
        }
    }

    pub fn is_changed(&self) -> bool {
        self.draft != self.applied
    }

    /// The settings to adopt; closing without applying is dropping the draft.
    pub fn apply(self) -> Settings {
        self.draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };

        chosen.write(&mut preferences);

        assert_eq!(preferences.get("devnotes.theme"), Some("light"));
        assert_eq!(preferences.get("devnotes.area"), Some("http"));
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
        assert_eq!(draft.apply().locale, LocaleChoice::En);
    }
}
