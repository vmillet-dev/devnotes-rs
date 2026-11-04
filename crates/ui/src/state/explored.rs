//! The document the editor explores, computed off the frame; the last answer stays while the
//! next one loads, so nothing jumps while typing.

use engine::json::model::{JsonQuery, JsonView};

use super::QueryCell;
use crate::api;
use crate::backend::Backend;

#[derive(Default)]
pub struct ExploredState {
    cell: QueryCell<(u64, JsonQuery), JsonView>,
}

impl ExploredState {
    pub fn want(&mut self, session: u64, query: JsonQuery) {
        self.cell.want((session, query));
    }

    pub fn update(&mut self, backend: &Backend) {
        self.cell
            .update(|(_, query)| api::json::explore(backend, query));
    }

    /// The opening `session`'s view as last answered, and the text it was made of: its spans
    /// point into that text, not into the draft typed since.
    pub fn of(&self, session: u64) -> Option<(&JsonView, &str)> {
        let (asked, query) = self.cell.answered()?;
        let view = self.cell.value()?;
        (*asked == session).then_some((view, query.text.as_str()))
    }
}
