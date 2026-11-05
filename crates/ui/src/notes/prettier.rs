//! The library's Prettier settings (`devnotes.notes.prettier`), as the reference stored them, and
//! the options they make: every snippet of a library formats alike.

use serde_json::{Value, json};
use text::format::Options;

pub const KEY: &str = "devnotes.notes.prettier";
pub const WIDTH_RANGE: (u32, u32) = (40, 200);
/// The width of a tab when the level is one: what the editor's Shift+Tab takes back.
const TAB_WIDTH: u32 = 4;

/// `Editor` is the Tab key's own level, whatever the language makes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indentation {
    Editor,
    Two,
    Four,
    Tab,
}

impl Indentation {
    pub const ALL: [Indentation; 4] = [
        Indentation::Editor,
        Indentation::Two,
        Indentation::Four,
        Indentation::Tab,
    ];

    fn id(self) -> &'static str {
        match self {
            Indentation::Editor => "editor",
            Indentation::Two => "two",
            Indentation::Four => "four",
            Indentation::Tab => "tab",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the switches as the reference stored them"
)]
pub struct PrettierSettings {
    pub print_width: u32,
    pub indentation: Indentation,
    pub single_quote: bool,
    pub semicolons: bool,
    pub trailing_commas: bool,
    pub format_on_save: bool,
}

impl Default for PrettierSettings {
    fn default() -> Self {
        Self {
            print_width: 100,
            indentation: Indentation::Editor,
            single_quote: true,
            semicolons: true,
            trailing_commas: true,
            format_on_save: false,
        }
    }
}

impl PrettierSettings {
    /// Trusted no further than it reads: what the file does not name stays default.
    pub fn read(stored: Option<&str>) -> Self {
        let fallback = Self::default();
        let Some(Value::Object(value)) = stored.and_then(|text| serde_json::from_str(text).ok())
        else {
            return fallback;
        };
        let flag =
            |key: &str, default: bool| value.get(key).and_then(Value::as_bool).unwrap_or(default);
        let indentation = value.get("indentation").and_then(Value::as_str);
        Self {
            print_width: value
                .get("printWidth")
                .and_then(Value::as_f64)
                .and_then(clamp_width)
                .unwrap_or(fallback.print_width),
            indentation: Indentation::ALL
                .into_iter()
                .find(|each| Some(each.id()) == indentation)
                .unwrap_or(fallback.indentation),
            single_quote: match value.get("quotes").and_then(Value::as_str) {
                Some("double") => false,
                Some("single") => true,
                _ => fallback.single_quote,
            },
            semicolons: flag("semicolons", fallback.semicolons),
            trailing_commas: flag("trailingCommas", fallback.trailing_commas),
            format_on_save: flag("formatOnSave", fallback.format_on_save),
        }
    }

    pub fn written(&self) -> String {
        json!({
            "printWidth": self.print_width,
            "indentation": self.indentation.id(),
            "quotes": if self.single_quote { "single" } else { "double" },
            "semicolons": self.semicolons,
            "trailingCommas": self.trailing_commas,
            "formatOnSave": self.format_on_save,
        })
        .to_string()
    }

    /// `editor_indent` is one level as the editor's Tab key writes it.
    pub fn options(&self, editor_indent: &str) -> Options {
        let indent = match self.indentation {
            Indentation::Editor => editor_indent,
            Indentation::Two => "  ",
            Indentation::Four => "    ",
            Indentation::Tab => "\t",
        };
        let tabs = indent == "\t";
        Options {
            print_width: self.print_width,
            tab_width: if tabs {
                TAB_WIDTH
            } else {
                u32::try_from(indent.len()).unwrap_or(2)
            },
            use_tabs: tabs,
            single_quote: self.single_quote,
            semi: self.semicolons,
            trailing_comma: self.trailing_commas,
        }
    }
}

/// A width typed out of range comes back to the nearest bound; one that is no number, to none.
pub fn clamp_width(value: f64) -> Option<u32> {
    if !value.is_finite() {
        return None;
    }
    let (min, max) = WIDTH_RANGE;
    let clamped = value.round().clamp(f64::from(min), f64::from(max));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a few hundred"
    )]
    let width = clamped as u32;
    Some(width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_file_is_read_as_far_as_it_is_right() {
        let read = PrettierSettings::read(Some(
            r#"{"printWidth": 999, "indentation": "tab", "quotes": "double", "semicolons": "no"}"#,
        ));
        assert_eq!(read.print_width, 200);
        assert_eq!(read.indentation, Indentation::Tab);
        assert!(!read.single_quote);
        assert!(read.semicolons, "not a boolean: the default");
        assert_eq!(
            PrettierSettings::read(Some("{")),
            PrettierSettings::default()
        );
        assert_eq!(PrettierSettings::read(Some(&read.written())), read);
    }

    #[test]
    fn the_editor_s_level_is_the_language_s_and_a_tab_counts_four() {
        let settings = PrettierSettings::default();
        assert_eq!(settings.options("    ").tab_width, 4);
        let tabs = PrettierSettings {
            indentation: Indentation::Tab,
            ..settings
        }
        .options("  ");
        assert!(tabs.use_tabs);
        assert_eq!(tabs.tab_width, 4);
        assert_eq!(clamp_width(12.4), Some(40));
        assert_eq!(clamp_width(f64::NAN), None);
    }
}
