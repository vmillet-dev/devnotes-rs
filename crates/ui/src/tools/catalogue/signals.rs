//! The Unix signals: the facts here, their use in the reference's words.

use i18n::{Language, keys};
use serde_json::Value;

use super::reference::{Reference, Table, columns_of, facts, groups_of, text, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 6] = [
    "terminal",
    "stopping",
    "faults",
    "timers",
    "events",
    "application",
];
const WIDE_CARD: f32 = 760.0;
const NONE: &str = "—";

/// Linux's number on x86 and ARM, macOS's, `null` where a system has none; a range for the
/// real-time ones. SIGKILL and SIGSTOP alone cannot be caught, blocked or ignored.
const SIGNALS: &str = include_str!("facts/signals.json");

/// `34–64`, the real-time signals, holds every number between.
fn holds(numbers: &str, wanted: u32) -> bool {
    let mut bounds = numbers.split('–').filter_map(|it| it.parse::<u32>().ok());
    let Some(from) = bounds.next() else {
        return false;
    };
    (from..=bounds.next().unwrap_or(from)).contains(&wanted)
}

/// A number is a signal's number exactly: « 9 » is SIGKILL, not 19 nor 29.
fn matches_signal(row: &ReferenceRow, query: &str) -> bool {
    let wanted = query.trim();
    match wanted.parse() {
        Ok(number) => row.cells.first().is_some_and(|it| holds(&it.text, number)),
        Err(_) => row.matches(&crate::api::tools::fold(wanted)),
    }
}

/// The action is searched too: « core » finds every signal that leaves a core dump.
fn row(signal: &Value, language: Language) -> ReferenceRow {
    use keys::tools::signals as words;
    let (name, group) = (text(signal, "name"), text(signal, "group"));
    let linux = signal["linux"].as_str();
    let usage = word(language, &format!("tools.signals.words.signals.{name}"));
    let heading = word(language, &format!("tools.signals.groups.{group}"));
    let action = word(
        language,
        &format!("tools.signals.actions.{}", text(signal, "action")),
    );
    let catchable = if signal["uncatchable"] == true {
        words::NO
    } else {
        words::YES
    };
    let cells = vec![
        Cell::mono(linux.unwrap_or(NONE)),
        Cell::mono(name),
        Cell::text(action.clone()),
        Cell::text(i18n::translate(language, catchable, &[])),
        Cell::text(usage.clone()),
        Cell::mono(signal["macos"].as_str().unwrap_or(NONE)),
    ];
    let title = linux.map_or_else(|| name.to_owned(), |number| format!("{name} ({number})"));
    ReferenceRow::new(name, group, cells)
        .searched_by([name, &usage, &heading, &action])
        .found_as(title, usage)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: facts(SIGNALS).iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.signals.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.signals.columns",
            &[
                ("number", Some(0.07)),
                ("name", Some(0.12)),
                ("action", Some(0.16)),
                ("catchable", Some(0.14)),
                ("use", None),
                ("macos", Some(0.08)),
            ],
        ),
        caption: keys::tools::signals::NAME,
        placeholder: keys::tools::signals::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: Some(matches_signal),
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_a_signal_s_number_exactly() {
        let table = table(Language::En);
        let found = |query: &str| {
            let rows = table.rows.iter();
            rows.filter(|row| matches_signal(row, query))
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>()
        };

        assert_eq!(found("9"), ["SIGKILL"]);
        assert!(found("40").iter().any(|it| it.contains("RT")));
    }
}
