//! A reference: rows built once a language from the facts of its file and the words of its own,
//! drawn by the one table. Nothing is computed, and nothing is kept as a note.

use egui::Ui;
use i18n::{Key, Language};

use crate::tools::ui::{Column, Group, ReferenceRow, ReferenceTable};
use crate::tools::{Drawn, Opening, Tool};

/// What a reference's file gives the table.
pub(crate) struct Table {
    pub rows: Vec<ReferenceRow>,
    pub groups: Vec<Group>,
    pub columns: Vec<Column>,
    pub caption: Key,
    pub placeholder: Key,
    /// A card is no narrower than this.
    pub card_width: f32,
    /// For a reference that reads a query its own way: a code in any base.
    pub matches: Option<fn(&ReferenceRow, &str) -> bool>,
}

pub(super) struct Reference {
    build: fn(Language) -> Table,
    query: String,
    /// The row the home's search opened it on, until something is searched here.
    lit: Option<String>,
    built: Option<(Language, Table)>,
}

impl Reference {
    pub(super) fn of(build: fn(Language) -> Table) -> Box<dyn Tool> {
        Box::new(Self {
            build,
            query: String::new(),
            lit: None,
            built: None,
        })
    }
}

impl Table {
    /// The rows `query` finds, by the reference's own reading of it.
    pub(crate) fn found(&self, query: &str) -> Vec<&ReferenceRow> {
        let folded = crate::api::tools::fold(query.trim());
        let finds = |row: &&ReferenceRow| match self.matches {
            Some(matches) => matches(row, query),
            None => row.matches(&folded),
        };
        self.rows.iter().filter(finds).collect()
    }
}

impl Tool for Reference {
    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        if self
            .built
            .as_ref()
            .is_none_or(|(built, _)| *built != language)
        {
            self.built = Some((language, (self.build)(language)));
        }
        let Some((_, table)) = &self.built else {
            return;
        };
        let caption = i18n::translate(language, table.caption, &[]);
        let placeholder = i18n::translate(language, table.placeholder, &[]);
        let shown = ReferenceTable {
            rows: &table.rows,
            groups: &table.groups,
            columns: &table.columns,
            caption: &caption,
            placeholder: &placeholder,
            card_width: table.card_width,
            lit: self.lit.as_deref(),
            matches: table.matches,
        };
        let asked = self.query.clone();
        shown.show(ui, &mut self.query, language);
        if self.query != asked {
            self.lit = None;
        }
    }

    fn opened(&mut self, opening: Opening) {
        match opening {
            Opening::OnRow(key) => {
                self.query.clear();
                self.lit = Some(key);
            }
            Opening::Searched(query) => {
                self.query = query;
                self.lit = None;
            }
        }
    }

    fn clear(&mut self) {
        self.query.clear();
        self.lit = None;
    }
}

/// The groups of a reference, their labels under `path`.
pub(super) fn groups_of(language: Language, path: &str, ids: &[&str]) -> Vec<Group> {
    ids.iter()
        .map(|id| Group {
            id: (*id).to_owned(),
            label: word(language, &format!("{path}.{id}")),
        })
        .collect()
}

/// The columns of a reference, their labels under `path`.
pub(super) fn columns_of(
    language: Language,
    path: &str,
    columns: &[(&str, Option<f32>)],
) -> Vec<Column> {
    columns
        .iter()
        .map(|(id, share)| Column {
            label: word(language, &format!("{path}.{id}")),
            share: *share,
        })
        .collect()
}

/// The text of `path` among the references' words; nothing when it has none.
pub(super) fn word(language: Language, path: &str) -> String {
    Key::find(path).map_or_else(String::new, |key| i18n::as_written(language, key))
}
