//! The palette: one field over whichever area is on screen. It asks the core for the notes of
//! every space, ranked by how their titles answer, and for the requests, and ignores the canvas's
//! filters: its search would otherwise change what the canvas shows behind it. The tools, the
//! spaces and the commands are read here, which knows their words.

use std::time::Duration;

use chrono::{DateTime, Utc};
use engine::http::search::RequestHit;
use engine::notes::model::TagUsage;
use engine::notes::view::{NoteFilter, NoteOrder, NotesQuery, NotesView};

use super::notes_query::{DEBOUNCE, tz_offset_minutes};
use super::{QueryCell, Revision};
use crate::api;
use crate::backend::{Backend, Pending};

/// Beyond this, notes no longer fit beside the other kinds.
pub const MAX_NOTES: usize = 8;

/// What `@` narrows the search to, in the order the groups are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Notes,
    Requests,
    Tools,
    Spaces,
}

pub const SCOPES: [Scope; 4] = [Scope::Notes, Scope::Requests, Scope::Tools, Scope::Spaces];

/// A chip in the field: a kind, one space's notes, or a tag's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chip {
    Scope(Scope),
    Space { id: String, name: String },
    Tag(String),
}

/// What is typed after the chips asks for: a search, or a list to choose from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Search(String),
    Scopes(String),
    Tags(String),
    Commands(String),
}

/// `@` and `#` are read where the word being typed starts, `>` only at the head of the field.
pub fn mode(text: &str) -> Mode {
    let last = text.rsplit(char::is_whitespace).next().unwrap_or("");
    if let Some(word) = last.strip_prefix('@') {
        return Mode::Scopes(word.to_owned());
    }
    if let Some(word) = last.strip_prefix('#') {
        return Mode::Tags(word.to_owned());
    }
    if let Some(term) = text.trim_start().strip_prefix('>') {
        return Mode::Commands(term.trim().to_owned());
    }
    Mode::Search(text.trim().to_owned())
}

#[derive(Debug, Clone, PartialEq)]
struct Params {
    term: String,
    space: Option<String>,
    tags: Vec<String>,
    pinned_first: bool,
    revision: Revision,
}

#[derive(Default)]
pub struct PaletteState {
    open: bool,
    /// Asked from another application.
    from_elsewhere: bool,
    text: String,
    chips: Vec<Chip>,
    /// The term asked of the core, once the keys have rested.
    term: String,
    due: Option<f64>,
    cell: QueryCell<Params, NotesView>,
    requests: QueryCell<(String, Revision), Vec<RequestHit>>,
    reading_tags: Option<Pending<Vec<TagUsage>>>,
    tags: Vec<TagUsage>,
    /// A copy asked from here: once it is on the clipboard, the palette closes.
    copying: bool,
}

impl PaletteState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn from_elsewhere(&self) -> bool {
        self.from_elsewhere
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn chips(&self) -> &[Chip] {
        &self.chips
    }

    pub fn tags(&self) -> &[TagUsage] {
        &self.tags
    }

    pub fn mode(&self) -> Mode {
        mode(&self.text)
    }

    /// Nothing typed and nothing chosen: the recent notes, a way back to what was in hand.
    pub fn recent(&self) -> bool {
        self.chips.is_empty() && self.mode() == Mode::Search(String::new())
    }

    /// What the search reaches: a space or a tag narrows to notes; no chip at all is every kind.
    pub fn searched(&self, scope: Scope) -> bool {
        let scoped: Vec<Scope> = self
            .chips
            .iter()
            .filter_map(|chip| match chip {
                Chip::Scope(scope) => Some(*scope),
                _ => None,
            })
            .collect();
        let narrowed = self
            .chips
            .iter()
            .any(|chip| !matches!(chip, Chip::Scope(_)));
        if scoped.is_empty() && !narrowed {
            return true;
        }
        scoped.contains(&scope) || (narrowed && scope == Scope::Notes)
    }

    /// The notes found, ranked, at most [`MAX_NOTES`]; none while a list of chips is offered.
    pub fn notes(&self) -> Option<&NotesView> {
        let wanted = matches!(self.mode(), Mode::Search(_)) && self.searched(Scope::Notes);
        self.cell.value().filter(|_| wanted)
    }

    /// The requests found, ranked; none while a list of chips is offered.
    pub fn requests(&self) -> &[RequestHit] {
        let wanted = matches!(self.mode(), Mode::Search(_)) && self.searched(Scope::Requests);
        self.requests
            .value()
            .filter(|_| wanted)
            .map_or(&[], Vec::as_slice)
    }

    /// Opens on the recent notes: reopening without typing has a meaning.
    pub fn open(&mut self, backend: &Backend, from_elsewhere: bool) {
        self.open = true;
        self.from_elsewhere = from_elsewhere;
        self.text.clear();
        self.term.clear();
        self.chips.clear();
        self.due = None;
        self.copying = false;
        // Edits of the HTTP area bump no revision: a palette opened anew reads them.
        self.requests = QueryCell::default();
        self.reading_tags = Some(api::notes::list_tags(backend));
    }

    pub fn close(&mut self) {
        self.open = false;
        self.due = None;
        self.copying = false;
    }

    pub fn type_text(&mut self, text: String, now: f64) {
        self.text = text;
        self.due = Some(now + DEBOUNCE);
    }

    /// The word that asked for it leaves the field. A second space takes the place of the first.
    pub fn add_chip(&mut self, chip: Chip) {
        self.chips.retain(|other| {
            other != &chip
                && !(matches!(chip, Chip::Space { .. }) && matches!(other, Chip::Space { .. }))
        });
        self.chips.push(chip);
        let kept = match self.text.rfind(char::is_whitespace) {
            Some(at) => self.text[..at].trim_end().to_owned(),
            None => String::new(),
        };
        self.text = kept;
        self.term = search_term(&self.text);
        self.due = None;
    }

    pub fn remove_chip(&mut self, at: usize) {
        if at < self.chips.len() {
            self.chips.remove(at);
        }
    }

    pub fn copying(&mut self) {
        self.copying = true;
    }

    /// `true` once: the copy asked from here has reached the clipboard.
    pub fn copied(&mut self) -> bool {
        let was = self.copying;
        self.copying = false;
        was
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        (now, pinned_first, revision): (DateTime<Utc>, bool, Revision),
    ) {
        if let Some(answer) = self.reading_tags.as_mut().and_then(Pending::poll) {
            self.reading_tags = None;
            self.tags = answer.unwrap_or_default();
        }
        if !self.open {
            return;
        }
        if let Some(due) = self.due {
            let time = ctx.input(|input| input.time);
            if time >= due {
                self.term = search_term(&self.text);
                self.due = None;
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(due - time));
            }
        }
        let space = self.chips.iter().find_map(|chip| match chip {
            Chip::Space { id, .. } => Some(id.clone()),
            _ => None,
        });
        let tags = self
            .chips
            .iter()
            .filter_map(|chip| match chip {
                Chip::Tag(tag) => Some(tag.clone()),
                _ => None,
            })
            .collect();
        self.cell.want(Params {
            term: self.term.clone(),
            space,
            tags,
            pinned_first,
            revision,
        });
        self.cell
            .update(|params| api::notes::query(backend, query(&params, now)));
        if matches!(self.mode(), Mode::Search(_)) && self.searched(Scope::Requests) {
            self.requests.want((self.term.clone(), revision));
        }
        self.requests
            .update(|(term, _)| api::http::search_requests(backend, term));
    }
}

fn search_term(text: &str) -> String {
    match mode(text) {
        Mode::Search(term) => term,
        _ => String::new(),
    }
}

fn query(params: &Params, now: DateTime<Utc>) -> NotesQuery {
    NotesQuery {
        space_id: params.space.clone(),
        folder_id: None,
        search: params.term.clone(),
        filter: NoteFilter::All,
        tags: params.tags.clone(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: engine::notes::view::Grouping::default(),
        ranked: true,
        now,
        tz_offset_minutes: tz_offset_minutes(now),
        pinned_first: params.pinned_first,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prefixes_are_read_where_the_word_starts() {
        assert_eq!(mode("reset"), Mode::Search("reset".into()));
        assert_eq!(mode("reset @no"), Mode::Scopes("no".into()));
        assert_eq!(mode("#ops"), Mode::Tags("ops".into()));
        assert_eq!(mode("> new"), Mode::Commands("new".into()));
        assert_eq!(mode("a > b"), Mode::Search("a > b".into()));
    }

    #[test]
    fn a_chip_takes_the_word_that_asked_for_it_and_narrows_to_notes() {
        let mut palette = PaletteState {
            text: "reset #op".into(),
            ..PaletteState::default()
        };
        palette.add_chip(Chip::Tag("ops".into()));

        assert_eq!(palette.text(), "reset");
        assert!(palette.searched(Scope::Notes));
        assert!(!palette.searched(Scope::Spaces));
    }

    #[test]
    fn a_second_space_takes_the_first_one_s_place() {
        let mut palette = PaletteState::default();
        let space = |id: &str| Chip::Space {
            id: id.into(),
            name: id.into(),
        };
        palette.add_chip(space("a"));
        palette.add_chip(space("b"));

        assert_eq!(palette.chips(), [space("b")]);
    }
}
