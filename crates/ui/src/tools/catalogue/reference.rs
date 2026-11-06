//! A reference: rows built once a language from the facts of its file and the words of its own,
//! drawn by the one table. Nothing is computed, and nothing is kept as a note.

use egui::Ui;
use i18n::{Key, Language};

use crate::tools::ui::{Column, Group, ReferenceRow, ReferenceTable};
use crate::tools::{Drawn, Tool};

/// What a reference's file gives the table.
pub(super) struct Table {
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
    built: Option<(Language, Table)>,
}

impl Reference {
    pub(super) fn of(build: fn(Language) -> Table) -> Box<dyn Tool> {
        Box::new(Self {
            build,
            query: String::new(),
            built: None,
        })
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
            lit: None,
            matches: table.matches,
        };
        shown.show(ui, &mut self.query, language);
    }

    fn clear(&mut self) {
        self.query.clear();
    }
}

/// The text of `path` among the references' words; nothing when it has none.
pub(super) fn word(language: Language, path: &str) -> String {
    Key::find(path).map_or_else(String::new, |key| i18n::translate(language, key, &[]))
}
