//! Each tab's console: what Rust says as a send goes — its traffic, the scripts' logs, the tests
//! that failed, what stopped —, kept for the session and never written. Folded at first.

use std::collections::{HashMap, VecDeque};

use engine::http::console::{ConsoleEvent, ConsoleLine};

/// What a tab keeps, the oldest dropped first.
pub const LINES_KEPT: usize = 500;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Traffic,
    Logs,
    Errors,
}

impl Filter {
    pub const ALL: [Self; 4] = [Self::All, Self::Traffic, Self::Logs, Self::Errors];

    /// Which of the three a line falls under.
    pub fn of(line: &ConsoleLine) -> Self {
        match line {
            ConsoleLine::Out { .. } | ConsoleLine::In { .. } => Self::Traffic,
            ConsoleLine::Log { .. } => Self::Logs,
            _ => Self::Errors,
        }
    }

    pub fn holds(self, line: &ConsoleLine) -> bool {
        self == Self::All || self == Self::of(line)
    }
}

#[derive(Default)]
pub struct Console {
    lines: HashMap<String, VecDeque<ConsoleEvent>>,
    /// One for the area: a console unfolded stays so from tab to tab.
    open: bool,
}

impl Console {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn lines_of(&self, key: &str) -> impl Iterator<Item = &ConsoleEvent> {
        self.lines.get(key).into_iter().flatten()
    }

    pub fn take(&mut self, key: &str, event: ConsoleEvent) {
        let lines = self.lines.entry(key.to_owned()).or_default();
        lines.push_back(event);
        if lines.len() > LINES_KEPT {
            lines.pop_front();
        }
    }

    pub fn clear(&mut self, key: &str) {
        self.lines.remove(key);
    }

    pub fn rekey(&mut self, from: &str, to: &str) {
        if let Some(lines) = self.lines.remove(from) {
            self.lines.insert(to.to_owned(), lines);
        }
    }
}
