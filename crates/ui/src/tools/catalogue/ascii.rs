//! The 128 code points: computed, their words alone being a file.

use i18n::{Language, keys};

use super::reference::{Reference, Table, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, Column, Group, ReferenceRow};

const SPACE: u32 = 32;
const DELETE: u32 = 127;

const GROUPS: [&str; 5] = ["control", "punctuation", "digits", "upper", "lower"];

/// The abbreviation, the standard's own name, the usual escape, and what else it is searched by.
struct Control(
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

const CONTROLS: [Control; 32] = [
    Control("NUL", "Null", "\\0", &[]),
    Control("SOH", "Start of Heading", "", &[]),
    Control("STX", "Start of Text", "", &[]),
    Control("ETX", "End of Text", "", &[]),
    Control("EOT", "End of Transmission", "", &["EOF"]),
    Control("ENQ", "Enquiry", "", &[]),
    Control("ACK", "Acknowledge", "", &[]),
    Control("BEL", "Bell", "\\a", &[]),
    Control("BS", "Backspace", "\\b", &[]),
    Control("HT", "Horizontal Tab", "\\t", &[]),
    Control("LF", "Line Feed", "\\n", &["newline", "NL"]),
    Control("VT", "Vertical Tab", "\\v", &[]),
    Control("FF", "Form Feed", "\\f", &[]),
    Control("CR", "Carriage Return", "\\r", &[]),
    Control("SO", "Shift Out", "", &[]),
    Control("SI", "Shift In", "", &[]),
    Control("DLE", "Data Link Escape", "", &[]),
    Control("DC1", "Device Control 1", "", &["XON"]),
    Control("DC2", "Device Control 2", "", &[]),
    Control("DC3", "Device Control 3", "", &["XOFF"]),
    Control("DC4", "Device Control 4", "", &[]),
    Control("NAK", "Negative Acknowledge", "", &[]),
    Control("SYN", "Synchronous Idle", "", &[]),
    Control("ETB", "End of Transmission Block", "", &[]),
    Control("CAN", "Cancel", "", &[]),
    Control("EM", "End of Medium", "", &[]),
    Control("SUB", "Substitute", "", &[]),
    Control("ESC", "Escape", "\\e", &[]),
    Control("FS", "File Separator", "", &[]),
    Control("GS", "Group Separator", "", &[]),
    Control("RS", "Record Separator", "", &[]),
    Control("US", "Unit Separator", "", &[]),
];

const DEL: Control = Control("DEL", "Delete", "", &[]);

fn control_of(code: u32) -> Option<&'static Control> {
    if code == DELETE {
        return Some(&DEL);
    }
    CONTROLS.get(usize::try_from(code).ok()?)
}

fn group_of(code: u32) -> &'static str {
    match code {
        0..=31 | DELETE => "control",
        48..=57 => "digits",
        65..=90 => "upper",
        97..=122 => "lower",
        _ => "punctuation",
    }
}

/// `^A` for 1: a control is typed with Ctrl and the character 64 above it; DEL is `^?`.
fn ctrl_key(code: u32) -> String {
    match code {
        DELETE => "^?".to_owned(),
        0..=31 => char::from_u32(code + 64).map_or_else(String::new, |it| format!("^{it}")),
        _ => String::new(),
    }
}

/// The code a query names in some base: `65`, `0x41`, `1b`, `0o101`, `0b1000001`.
fn code_in(query: &str) -> Option<u32> {
    let lower = query.to_lowercase();
    if !query.is_empty() && query.len() <= 3 && query.bytes().all(|it| it.is_ascii_digit()) {
        return query.parse().ok();
    }
    for (prefix, radix) in [("0x", 16), ("\\x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = lower.strip_prefix(prefix) {
            return u32::from_str_radix(digits, radix).ok();
        }
    }
    let hexadecimal = lower.len() == 2
        && lower.bytes().all(|it| it.is_ascii_hexdigit())
        && lower.bytes().any(|it| it.is_ascii_alphabetic());
    hexadecimal
        .then(|| u32::from_str_radix(&lower, 16).ok())
        .flatten()
}

/// One character is that character, case and all, or a one-digit code: « A » is 65 alone, « 6 »
/// both the digit and ACK. Anything longer is a code in some base, or words.
fn matches_ascii(row: &ReferenceRow, query: &str) -> bool {
    let code: u32 = row.key.parse().unwrap_or(u32::MAX);
    let character = char::from_u32(code).map(String::from).unwrap_or_default();
    if query.chars().count() == 1 {
        return character == query || row.key == query;
    }
    code_in(query.trim()) == Some(code) || row.matches(&crate::api::tools::fold(query.trim()))
}

fn row(code: u32, language: Language) -> ReferenceRow {
    let group = group_of(code);
    let control = control_of(code);
    let character = char::from_u32(code).map(String::from).unwrap_or_default();
    let said = word(language, &format!("tools.ascii.words.codes.{code}"));
    let ctrl = ctrl_key(code);
    let shown = match control {
        Some(control) => control.0.to_owned(),
        None if code == SPACE => "SP".to_owned(),
        None => character.clone(),
    };
    let name = control.map_or_else(|| said.clone(), |it| it.1.to_owned());
    let name = Cell::text(name);
    let name = if control.is_some() {
        name.below(said.clone())
    } else {
        name
    };
    let cells = vec![
        Cell::mono(code.to_string()),
        Cell::mono(format!("{code:02X}")),
        Cell::mono(format!("{code:03o}")),
        Cell::mono(format!("{code:08b}")),
        if control.is_some() {
            Cell::chips(vec![shown.clone()])
        } else {
            Cell::mono(shown.clone())
        },
        name,
        Cell::mono(control.map_or("", |it| it.2)),
        Cell::mono(ctrl.clone()),
    ];
    let heading = word(language, &format!("tools.ascii.groups.{group}"));
    let mut searched: Vec<&str> = vec![&said, &ctrl, &heading];
    if let Some(control) = control {
        searched.extend([control.0, control.1, control.2]);
        searched.extend(control.3);
    }
    let what = if code == SPACE {
        said.clone()
    } else {
        shown.clone()
    };
    let title = format!("{shown} ({code})");
    let said_as = if said.is_empty() {
        control.map_or("", |it| it.1)
    } else {
        said.as_str()
    };
    let hexadecimal = format!("0x{code:02X}");
    let line = if said_as.is_empty() {
        hexadecimal
    } else {
        format!("{said_as} · {hexadecimal}")
    };
    let mut row = ReferenceRow::new(code.to_string(), group, cells)
        .searched_by(searched)
        .found_as(title, line);
    row.copy = Some((character, what));
    row
}

pub(crate) fn table(language: Language) -> Table {
    let tr = |id: &str| word(language, &format!("tools.ascii.columns.{id}"));
    let column = |id: &str, share| Column {
        label: tr(id),
        share,
    };
    Table {
        rows: (0..128).map(|code| row(code, language)).collect(),
        groups: GROUPS
            .iter()
            .map(|id| Group {
                id: (*id).to_owned(),
                label: word(language, &format!("tools.ascii.groups.{id}")),
            })
            .collect(),
        columns: vec![
            column("decimal", Some(0.07)),
            column("hex", Some(0.07)),
            column("octal", Some(0.07)),
            column("binary", Some(0.12)),
            column("character", Some(0.08)),
            column("name", None),
            column("escape", Some(0.1)),
            column("ctrl", Some(0.08)),
        ],
        caption: keys::tools::ascii::NAME,
        placeholder: keys::tools::ascii::PLACEHOLDER,
        card_width: 860.0,
        matches: Some(matches_ascii),
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_names_a_code_in_any_base() {
        assert_eq!(code_in("65"), Some(65));
        assert_eq!(code_in("0x41"), Some(65));
        assert_eq!(code_in("1b"), Some(27));
        assert_eq!(code_in("0o101"), Some(65));
        assert_eq!(code_in("0b1000001"), Some(65));
        assert_eq!(code_in("newline"), None);
    }

    #[test]
    fn one_character_is_itself_case_and_all() {
        let upper = row(65, Language::En);
        let lower = row(97, Language::En);

        assert!(matches_ascii(&upper, "A"));
        assert!(!matches_ascii(&lower, "A"));
    }

    #[test]
    fn a_control_says_its_abbreviation_and_its_key() {
        let escape = row(27, Language::En);

        assert_eq!(escape.cells[4].chips, ["ESC"]);
        assert_eq!(escape.cells[7].text, "^[");
        assert!(matches_ascii(&escape, "escape"));
    }
}
