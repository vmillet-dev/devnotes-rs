//! The notes area: the canvas of cards, the header over it and the rail beside it, as they are
//! ported (plan § 7 « Phase 3 »).

pub mod calc;
pub mod canvas;
pub mod language;
pub mod ui;

use egui::{Role, Sense, Ui, UiBuilder, WidgetInfo};
use engine::notes::kind::NoteKind;
use engine::notes::priority::Priority;
use i18n::{Key, keys};

use engine::notes::checklist::ChecklistItem;

use crate::action::Actions;
use crate::state::AppState;
use crate::state::drawing_pictures::Stamp;
use crate::widgets::address;

/// What the area's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The view, asked again after it failed to load.
    Reload,
    /// A todo list's items, ticked from its card: the whole list.
    SetItems {
        id: String,
        items: Vec<ChecklistItem>,
    },
    /// A drawing's picture, for the card that just showed it.
    Thumbnail(Stamp),
}

pub fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let (region, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Main, true, "area-notes"));
    address(ui, &response, "area-notes");
    let mut ui = ui.new_child(UiBuilder::new().max_rect(region));
    canvas::show(&mut ui, state, actions);
}

/// « Snippets », the heading of a kind's section.
fn kind_plural(kind: NoteKind) -> Key {
    match kind {
        NoteKind::Snippet => keys::notes::kind_plural::SNIPPET,
        NoteKind::Note => keys::notes::kind_plural::NOTE,
        NoteKind::Checklist => keys::notes::kind_plural::CHECKLIST,
        NoteKind::Calc => keys::notes::kind_plural::CALC,
        NoteKind::Drawing => keys::notes::kind_plural::DRAWING,
    }
}

/// The heading of a priority's section: « No priority » for the notes without one.
fn priority_section(priority: Priority) -> Key {
    match priority {
        Priority::None => keys::sections::NO_PRIORITY,
        Priority::Low => keys::notes::priority::LOW,
        Priority::Medium => keys::notes::priority::MEDIUM,
        Priority::High => keys::notes::priority::HIGH,
        Priority::Urgent => keys::notes::priority::URGENT,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_kind_and_priority_heads_a_section_of_its_own() {
        for locale in [i18n::Language::En, i18n::Language::Fr] {
            let mut headings = HashSet::new();
            let keys = NoteKind::ALL
                .map(kind_plural)
                .into_iter()
                .chain(Priority::ALL.map(priority_section));
            for key in keys {
                let heading = i18n::translate(locale, key, &[]);
                assert!(headings.insert(heading.clone()), "{heading}");
            }
        }
    }
}
