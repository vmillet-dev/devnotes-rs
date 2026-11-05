//! The HTTP area's state: the collections as the rail shows them. Every write is persisted, then
//! the tree read again: the core renumbers what a move or a copy touched, and the interface never
//! guesses the result.

pub mod tree;

use std::collections::BTreeSet;

use engine::http::model::HttpTree;
use i18n::keys;

use super::{QueryCell, Revision};
use crate::api;
use crate::backend::Backend;
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
use tree::RailRow;

/// In the library's preferences: which folds were left closed belongs to its collections.
pub const COLLAPSED_KEY: &str = "devnotes.notes.http.collapsed";

#[derive(Default)]
pub struct HttpState {
    /// Bumped by the area's own writes: the notes are not asked again for a request renamed.
    revision: Revision,
    tree: QueryCell<Revision, HttpTree>,
    collapsed: BTreeSet<String>,
    /// Laid out again when the tree or a fold changes, not every frame: 2,000 rows.
    rows: Vec<RailRow>,
}

impl HttpState {
    pub fn restore(&mut self, preferences: &Preferences) {
        let stored = preferences.get(COLLAPSED_KEY).unwrap_or("[]");
        self.collapsed = serde_json::from_str::<Vec<serde_json::Value>>(stored)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        self.lay_out();
    }

    /// Once a frame, with the library open: the tree is read the first time the area asks.
    pub fn update(&mut self, backend: &Backend, errors: &mut ErrorNotifier) {
        self.tree.want(self.revision);
        if self.tree.update(|_| api::http::tree(backend)) {
            if let Some(error) = self.tree.error() {
                errors.report(keys::http::FAILED, error);
            }
            self.lay_out();
        }
    }

    pub fn rows(&self) -> &[RailRow] {
        &self.rows
    }

    /// Read at least once: until then an empty tree says nothing about what the library holds.
    pub fn loaded(&self) -> bool {
        self.tree.value().is_some()
    }

    /// Folded or unfolded, and remembered in the library's preferences.
    pub fn toggle(&mut self, id: &str, preferences: &mut Preferences) {
        if !self.collapsed.remove(id) {
            self.collapsed.insert(id.to_owned());
        }
        let ids: Vec<&String> = self.collapsed.iter().collect();
        preferences.set(
            COLLAPSED_KEY,
            &serde_json::to_string(&ids).unwrap_or_default(),
        );
        self.lay_out();
    }

    fn lay_out(&mut self) {
        let empty = HttpTree::default();
        let tree = self.tree.value().unwrap_or(&empty);
        self.rows = tree::rail_rows(tree, &self.collapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folds_are_read_back_from_the_preferences() {
        let mut preferences = Preferences::default();
        let mut state = HttpState::default();

        state.toggle("a", &mut preferences);
        state.toggle("b", &mut preferences);
        state.toggle("a", &mut preferences);
        let mut read = HttpState::default();
        read.restore(&preferences);

        assert_eq!(preferences.get(COLLAPSED_KEY), Some(r#"["b"]"#));
        assert_eq!(read.collapsed, BTreeSet::from(["b".to_owned()]));
    }

    #[test]
    fn a_damaged_list_of_folds_is_none() {
        let mut preferences = Preferences::default();
        preferences.set(COLLAPSED_KEY, "{not json");
        let mut state = HttpState::default();

        state.restore(&preferences);

        assert!(state.collapsed.is_empty());
    }
}
