//! The Unix signals: the facts here, their use in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, groups_of, word};
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

/// macOS's number beside Linux's.
#[derive(Clone, Copy)]
enum Macos {
    Same,
    Other(&'static str),
    Absent,
}

use Macos::{Absent, Other, Same};

struct Signal {
    name: &'static str,
    /// On x86 and ARM; `None` where Linux has none. A range for the real-time ones.
    linux: Option<&'static str>,
    macos: Macos,
    group: &'static str,
    action: &'static str,
    /// SIGKILL and SIGSTOP alone cannot be caught, blocked or ignored.
    uncatchable: bool,
}

const SIGNALS: [Signal; 33] = [
    Signal {
        name: "SIGHUP",
        linux: Some("1"),
        macos: Same,
        group: "terminal",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGINT",
        linux: Some("2"),
        macos: Same,
        group: "terminal",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGQUIT",
        linux: Some("3"),
        macos: Same,
        group: "terminal",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGTSTP",
        linux: Some("20"),
        macos: Other("18"),
        group: "terminal",
        action: "stop",
        uncatchable: false,
    },
    Signal {
        name: "SIGTTIN",
        linux: Some("21"),
        macos: Same,
        group: "terminal",
        action: "stop",
        uncatchable: false,
    },
    Signal {
        name: "SIGTTOU",
        linux: Some("22"),
        macos: Same,
        group: "terminal",
        action: "stop",
        uncatchable: false,
    },
    Signal {
        name: "SIGWINCH",
        linux: Some("28"),
        macos: Same,
        group: "terminal",
        action: "ignore",
        uncatchable: false,
    },
    Signal {
        name: "SIGINFO",
        linux: None,
        macos: Other("29"),
        group: "terminal",
        action: "ignore",
        uncatchable: false,
    },
    Signal {
        name: "SIGTERM",
        linux: Some("15"),
        macos: Same,
        group: "stopping",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGKILL",
        linux: Some("9"),
        macos: Same,
        group: "stopping",
        action: "terminate",
        uncatchable: true,
    },
    Signal {
        name: "SIGSTOP",
        linux: Some("19"),
        macos: Other("17"),
        group: "stopping",
        action: "stop",
        uncatchable: true,
    },
    Signal {
        name: "SIGCONT",
        linux: Some("18"),
        macos: Other("19"),
        group: "stopping",
        action: "continue",
        uncatchable: false,
    },
    Signal {
        name: "SIGABRT",
        linux: Some("6"),
        macos: Same,
        group: "stopping",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGSEGV",
        linux: Some("11"),
        macos: Same,
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGBUS",
        linux: Some("7"),
        macos: Other("10"),
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGFPE",
        linux: Some("8"),
        macos: Same,
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGILL",
        linux: Some("4"),
        macos: Same,
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGTRAP",
        linux: Some("5"),
        macos: Same,
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGSYS",
        linux: Some("31"),
        macos: Other("12"),
        group: "faults",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGSTKFLT",
        linux: Some("16"),
        macos: Absent,
        group: "faults",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGALRM",
        linux: Some("14"),
        macos: Same,
        group: "timers",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGVTALRM",
        linux: Some("26"),
        macos: Same,
        group: "timers",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGPROF",
        linux: Some("27"),
        macos: Same,
        group: "timers",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGXCPU",
        linux: Some("24"),
        macos: Same,
        group: "timers",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGXFSZ",
        linux: Some("25"),
        macos: Same,
        group: "timers",
        action: "core",
        uncatchable: false,
    },
    Signal {
        name: "SIGCHLD",
        linux: Some("17"),
        macos: Other("20"),
        group: "events",
        action: "ignore",
        uncatchable: false,
    },
    Signal {
        name: "SIGPIPE",
        linux: Some("13"),
        macos: Same,
        group: "events",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGURG",
        linux: Some("23"),
        macos: Other("16"),
        group: "events",
        action: "ignore",
        uncatchable: false,
    },
    Signal {
        name: "SIGIO",
        linux: Some("29"),
        macos: Other("23"),
        group: "events",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGPWR",
        linux: Some("30"),
        macos: Absent,
        group: "events",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGUSR1",
        linux: Some("10"),
        macos: Other("30"),
        group: "application",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGUSR2",
        linux: Some("12"),
        macos: Other("31"),
        group: "application",
        action: "terminate",
        uncatchable: false,
    },
    Signal {
        name: "SIGRTMIN",
        linux: Some("34–64"),
        macos: Absent,
        group: "application",
        action: "terminate",
        uncatchable: false,
    },
];

/// `34–64`, the real-time signals, holds every number between.
fn holds(numbers: &str, wanted: u32) -> bool {
    let mut bounds = numbers.split('–').filter_map(|it| it.parse::<u32>().ok());
    let Some(from) = bounds.next() else {
        return false;
    };
    (from..=bounds.next().unwrap_or(from)).contains(&wanted)
}

fn linux_of(name: &str) -> Option<&'static str> {
    let signal = SIGNALS.iter().find(|it| it.name == name)?;
    signal.linux
}

/// A number is a signal's number exactly: « 9 » is SIGKILL, not 19 nor 29.
fn matches_signal(row: &ReferenceRow, query: &str) -> bool {
    let wanted = query.trim();
    match wanted.parse() {
        Ok(number) => linux_of(&row.key).is_some_and(|it| holds(it, number)),
        Err(_) => row.matches(&crate::api::tools::fold(wanted)),
    }
}

/// The action is searched too: « core » finds every signal that leaves a core dump.
fn row(signal: &Signal, language: Language) -> ReferenceRow {
    use keys::tools::signals as words;
    let name = signal.name;
    let usage = word(language, &format!("tools.signals.words.signals.{name}"));
    let heading = word(language, &format!("tools.signals.groups.{}", signal.group));
    let action = word(
        language,
        &format!("tools.signals.actions.{}", signal.action),
    );
    let catchable = if signal.uncatchable {
        words::NO
    } else {
        words::YES
    };
    let macos = match signal.macos {
        Same => signal.linux.unwrap_or(NONE),
        Other(number) => number,
        Absent => NONE,
    };
    let cells = vec![
        Cell::mono(signal.linux.unwrap_or(NONE)),
        Cell::mono(name),
        Cell::text(action.clone()),
        Cell::text(i18n::translate(language, catchable, &[])),
        Cell::text(usage.clone()),
        Cell::mono(macos),
    ];
    let title = signal
        .linux
        .map_or_else(|| name.to_owned(), |number| format!("{name} ({number})"));
    ReferenceRow::new(name, signal.group, cells)
        .searched_by([name, &usage, &heading, &action])
        .found_as(title, usage)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: SIGNALS.iter().map(|it| row(it, language)).collect(),
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
