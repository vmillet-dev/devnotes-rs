//! What the canvas holds apart from the notes: the ticked cards, and the card a first `Delete`
//! armed. The focus itself is egui's, on the card's button.

use std::collections::HashSet;

#[derive(Default)]
pub struct SelectionState {
    checked: HashSet<String>,
    armed: Option<String>,
}

impl SelectionState {
    pub fn armed(&self) -> Option<&str> {
        self.armed.as_deref()
    }

    /// `true` when `id` was armed already: the second press, which deletes.
    pub fn arm(&mut self, id: &str) -> bool {
        if self.armed.as_deref() == Some(id) {
            self.armed = None;
            return true;
        }
        self.armed = Some(id.to_owned());
        false
    }

    /// `true` when something was armed: Escape's first rung.
    pub fn disarm(&mut self) -> bool {
        self.armed.take().is_some()
    }

    pub fn is_checked(&self, id: &str) -> bool {
        self.checked.contains(id)
    }

    pub fn toggle(&mut self, id: &str) {
        if !self.checked.remove(id) {
            self.checked.insert(id.to_owned());
        }
    }

    /// From `anchor` to `id` in `order`, both ticked; `id` alone without an anchor on screen.
    pub fn check_range(&mut self, order: &[&str], anchor: Option<&str>, id: &str) {
        let Some(to) = order.iter().position(|each| *each == id) else {
            return;
        };
        let from = anchor
            .and_then(|anchor| order.iter().position(|each| *each == anchor))
            .unwrap_or(to);
        let (low, high) = (from.min(to), from.max(to));
        self.checked
            .extend(order[low..=high].iter().map(|each| (*each).to_owned()));
    }

    /// Every one of `ids` ticked, none untoggled: a sweep or a whole zone.
    pub fn check_all(&mut self, ids: impl IntoIterator<Item = String>) {
        self.checked.extend(ids);
    }

    /// `true` when something was ticked: Escape's next rung.
    pub fn clear(&mut self) -> bool {
        let had = !self.checked.is_empty();
        self.checked.clear();
        had
    }

    /// The ticked notes still on screen, in `order`: an id ticked then gone reaches no batch.
    pub fn checked_in(&self, order: &[&str]) -> Vec<String> {
        order
            .iter()
            .filter(|id| self.checked.contains(**id))
            .map(|id| (*id).to_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: [&str; 5] = ["a", "b", "c", "d", "e"];

    #[test]
    fn a_range_runs_from_the_focused_card_either_way() {
        let mut selection = SelectionState::default();
        selection.check_range(&ORDER, Some("d"), "b");
        assert_eq!(selection.checked_in(&ORDER), ["b", "c", "d"]);

        selection.clear();
        selection.check_range(&ORDER, None, "c");
        assert_eq!(selection.checked_in(&ORDER), ["c"]);
    }

    #[test]
    fn a_note_gone_from_the_view_is_in_no_batch() {
        let mut selection = SelectionState::default();
        selection.toggle("a");
        selection.toggle("z");

        assert_eq!(selection.checked_in(&ORDER), ["a"]);
        selection.toggle("a");
        assert!(selection.checked_in(&ORDER).is_empty());
    }
}
