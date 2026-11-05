//! Over the canvas while cards are ticked, never in its flow — inserted, it pushed every card
//! down on each tick. It says how many notes the next click acts on; its menus are commands,
//! naming what they do rather than a value chosen.

use egui::{
    CornerRadius, FontFamily, Margin, Rect, Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2,
    vec2,
};
use engine::notes::priority::Priority;
use i18n::keys;

use super::CanvasView;
use crate::action::{Action, Actions};
use crate::notes::Batch;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, IconButton, Kind, Naming, TextField};

const LEVELS: [Priority; 5] = [
    Priority::None,
    Priority::Low,
    Priority::Medium,
    Priority::High,
    Priority::Urgent,
];

/// No entry chosen: the menus name what they do.
const NONE: usize = usize::MAX;

/// A card on screen, as the bar reads it: the list's or the board's.
pub(super) struct Shown<'a> {
    pub(super) id: &'a str,
    pub(super) pinned: bool,
    pub(super) tags: &'a [String],
}

fn batch(actions: &mut Actions, batch: Batch) {
    actions.push(Action::Notes(crate::notes::Action::Batch(batch)));
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    shown: &[Shown<'_>],
    canvas_view: &mut CanvasView,
    actions: &mut Actions,
) {
    let order: Vec<&str> = shown.iter().map(|card| card.id).collect();
    let checked = state.selection.checked_in(&order);
    if checked.is_empty() {
        canvas_view.confirm_delete = false;
        return;
    }
    let p = palette(ui.ctx());
    let region = ui.max_rect();
    let band = Rect::from_min_size(
        pos2(region.left() + 16.0, region.top() + 10.0),
        vec2(region.width() - 32.0, 120.0),
    );
    let mut bar = ui.new_child(UiBuilder::new().max_rect(band));
    let frame = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .shadow(p.shadow_dropdown)
        .inner_margin(Margin::symmetric(10, 6));
    frame.show(&mut bar, |ui| {
        // Two lines rather than a control cut off when one does not hold.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            controls(ui, state, shown, &checked, canvas_view, actions);
        });
    });
}

fn controls(
    ui: &mut Ui,
    state: &AppState,
    shown: &[Shown<'_>],
    checked: &[String],
    canvas_view: &mut CanvasView,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    count(ui, checked.len(), language);
    placing(ui, state, actions);
    let levels: Vec<String> = LEVELS
        .iter()
        .map(|level| i18n::translate(language, level_key(*level), &[]))
        .collect();
    let options: Vec<(usize, &str)> = levels
        .iter()
        .enumerate()
        .map(|(at, name)| (at, name.as_str()))
        .collect();
    let mut picked = NONE;
    let label = tr(keys::selection::PRIORITY);
    if ChoiceMenu::new(&label, "selection-priority", &options)
        .naming(Naming::Label)
        .show(ui, &mut picked)
        .changed()
        && let Some(level) = LEVELS.get(picked)
    {
        batch(actions, Batch::Prioritise(*level));
    }
    tagging(ui, shown, checked, canvas_view, language, actions);
    let notes: Vec<_> = shown
        .iter()
        .filter(|card| checked.iter().any(|id| id == card.id))
        .collect();
    let all_pinned = notes.iter().all(|card| card.pinned);
    let pin = tr(if all_pinned {
        keys::selection::UNPIN
    } else {
        keys::selection::PIN
    });
    if Button::new(&pin, "selection-pin").show(ui).clicked() {
        batch(actions, Batch::Pin(!all_pinned));
    }
    let delete = tr(if canvas_view.confirm_delete {
        keys::selection::DELETE_CONFIRM
    } else {
        keys::selection::DELETE
    });
    let deleting = Button::new(&delete, "selection-delete")
        .kind(Kind::Destructive)
        .armed(canvas_view.confirm_delete)
        .show(ui);
    if deleting.clicked() {
        if canvas_view.confirm_delete {
            batch(actions, Batch::Delete);
        }
        canvas_view.confirm_delete = !canvas_view.confirm_delete;
    }
    let clear = tr(keys::selection::CLEAR);
    if IconButton::new(Icon::X, &clear, "selection-clear")
        .show(ui)
        .clicked()
    {
        actions.push(Action::Notes(crate::notes::Action::ClearSelection));
    }
}

/// Move to another space, file into a folder of the space shown or out of every one.
fn placing(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let spaces: Vec<(usize, &str)> = state
        .spaces
        .spaces()
        .iter()
        .enumerate()
        .map(|(at, space)| (at, space.name.as_str()))
        .collect();
    if !spaces.is_empty() {
        let mut picked = NONE;
        let label = tr(keys::selection::MOVE_TO);
        if ChoiceMenu::new(&label, "selection-move", &spaces)
            .naming(Naming::Label)
            .show(ui, &mut picked)
            .changed()
            && let Some(space) = state.spaces.spaces().get(picked)
        {
            batch(actions, Batch::MoveTo(space.id.clone()));
        }
    }
    let shown_space = state.spaces.active_space();
    let folders: Vec<_> = state
        .folders
        .list()
        .iter()
        .filter(|folder| shown_space.is_none_or(|space| folder.space_id == space))
        .collect();
    if !folders.is_empty() {
        let unfile = tr(keys::selection::UNFILE);
        let mut options: Vec<(usize, &str)> = folders
            .iter()
            .enumerate()
            .map(|(at, folder)| (at, folder.name.as_str()))
            .collect();
        options.push((folders.len(), unfile.as_str()));
        let mut picked = NONE;
        let label = tr(keys::selection::FILE_INTO);
        if ChoiceMenu::new(&label, "selection-file", &options)
            .naming(Naming::Label)
            .show(ui, &mut picked)
            .changed()
        {
            let folder = folders.get(picked).map(|folder| folder.id.clone());
            batch(actions, Batch::File(folder));
        }
    }
}

/// « 2 selected »: the one thing saying how many notes the next click acts on.
fn count(ui: &mut Ui, count: usize, language: i18n::Language) {
    let p = palette(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a selection on screen")]
    let words = i18n::translate(
        language,
        keys::selection::COUNT,
        &[("count", (count as u32).into())],
    );
    let galley = widgets::text(
        ui,
        &words,
        12.5,
        600.0,
        FontFamily::Proportional,
        p.amber_text,
    );
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(4.0, 0.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "selection-count");
    ui.painter().galley(rect.min, galley, p.amber_text);
}

/// A tag typed then Enter, onto every ticked note; and the menu of those they carry.
fn tagging(
    ui: &mut Ui,
    shown: &[Shown<'_>],
    checked: &[String],
    canvas_view: &mut CanvasView,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let tr = |key| i18n::translate(language, key, &[]);
    let (label, hint) = (
        tr(keys::selection::ADD_TAG_LABEL),
        tr(keys::selection::ADD_TAG_PLACEHOLDER),
    );
    let field = ui
        .allocate_ui(vec2(120.0, 32.0), |ui| {
            TextField::new(&mut canvas_view.tag_draft, &label, "selection-tag")
                .hint(&hint)
                .compact()
                .show(ui)
        })
        .inner;
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let tag = canvas_view.tag_draft.trim().to_owned();
    if entered && !tag.is_empty() {
        batch(actions, Batch::Tag(tag));
        canvas_view.tag_draft.clear();
    }
    let mut carried: Vec<&str> = shown
        .iter()
        .filter(|card| checked.iter().any(|id| id == card.id))
        .flat_map(|card| card.tags.iter().map(String::as_str))
        .collect();
    carried.sort_unstable();
    carried.dedup();
    if carried.is_empty() {
        return;
    }
    let options: Vec<(usize, &str)> = carried.iter().copied().enumerate().collect();
    let mut picked = NONE;
    let label = tr(keys::selection::REMOVE_TAG);
    if ChoiceMenu::new(&label, "selection-untag", &options)
        .naming(Naming::Label)
        .show(ui, &mut picked)
        .changed()
        && let Some(tag) = carried.get(picked)
    {
        batch(actions, Batch::Untag((*tag).to_owned()));
    }
}

fn level_key(level: Priority) -> i18n::Key {
    match level {
        Priority::None => keys::notes::priority::NONE,
        Priority::Low => keys::notes::priority::LOW,
        Priority::Medium => keys::notes::priority::MEDIUM,
        Priority::High => keys::notes::priority::HIGH,
        Priority::Urgent => keys::notes::priority::URGENT,
    }
}

#[cfg(test)]
mod tests {
    use egui::Modifiers;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::model::DisplayNote;

    use crate::notes::bench::{Bench, area, cards, click, click_id, settle, space};
    use crate::testing;

    fn stored(harness: &Harness<'_, Bench>, id: &str) -> DisplayNote {
        let id = id.to_owned();
        engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::notes::get_note(engine, id)
        })
        .unwrap()
    }

    fn card(harness: &Harness<'_, Bench>, title: &str) -> String {
        let view = harness.state().state.notes_query.view().unwrap();
        view.cards()
            .find(|card| card.title == title)
            .map(|card| card.id.clone())
            .unwrap()
    }

    fn ctrl_click(harness: &mut Harness<'_, Bench>, title: &str) {
        harness
            .get_by(|node| {
                node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
            })
            .click_modifiers(Modifiers::COMMAND);
        settle(harness);
    }

    fn three() -> (Harness<'static, Bench>, [String; 3]) {
        let (backend, profile) = testing::unlocked();
        for title in ["one", "two", "three"] {
            testing::note(&backend, title);
        }
        space(&backend, "Archive");
        let harness = area(backend, profile);
        let ids = ["one", "two", "three"].map(|title| card(&harness, title));
        (harness, ids)
    }

    fn said(harness: &Harness<'_, Bench>, author_id: &str) -> String {
        let node = harness.get_by(|node| node.author_id() == Some(author_id));
        let node = node.accesskit_node();
        node.label().or_else(|| node.value()).unwrap_or_default()
    }

    #[test]
    fn no_bar_until_a_card_is_ticked_then_it_counts_them() {
        let (mut harness, _) = three();
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("selection-bar-count"))
                .is_none()
        );

        ctrl_click(&mut harness, "one");
        ctrl_click(&mut harness, "two");

        assert_eq!(said(&harness, "selection-count"), "2 selected");
        // Ticked, a card does not open.
        assert!(!harness.state().state.editor.is_open());
    }

    #[test]
    fn the_bar_tags_and_pins_every_ticked_note_and_only_those() {
        let (mut harness, [one, two, three]) = three();
        ctrl_click(&mut harness, "one");
        ctrl_click(&mut harness, "three");

        click_id(&mut harness, "selection-tag");
        harness
            .get_by(|node| node.author_id() == Some("selection-tag"))
            .type_text("batch");
        harness.key_press(egui::Key::Enter);
        settle(&mut harness);
        click_id(&mut harness, "selection-pin");

        assert_eq!(stored(&harness, &one).tags, ["batch"]);
        assert_eq!(stored(&harness, &three).tags, ["batch"]);
        assert!(stored(&harness, &two).tags.is_empty());
        assert!(stored(&harness, &one).pinned && stored(&harness, &three).pinned);
        assert!(!stored(&harness, &two).pinned);
    }

    #[test]
    fn a_selection_is_moved_and_its_deletion_takes_two_clicks() {
        let (mut harness, [one, ..]) = three();
        let archive = crate::notes::bench::spaces_of(&harness)
            .into_iter()
            .find(|space| space.name == "Archive")
            .unwrap();
        ctrl_click(&mut harness, "one");

        click_id(&mut harness, "selection-move");
        click(&mut harness, "selection-move-0", "Archive");
        assert_eq!(stored(&harness, &one).space_id, archive.id);

        // A batch keeps its selection: the cross lets it go.
        click_id(&mut harness, "selection-clear");
        ctrl_click(&mut harness, "two");
        click_id(&mut harness, "selection-delete");
        assert_eq!(cards(&harness), 3);
        click_id(&mut harness, "selection-delete");
        assert_eq!(cards(&harness), 2);
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("selection-count"))
                .is_none()
        );
    }

    #[test]
    fn shift_ticks_a_range_from_the_focused_card_and_escape_clears_it() {
        let (mut harness, _) = three();
        harness.key_press(egui::Key::ArrowDown);
        settle(&mut harness);

        let last = harness.state().state.on_screen().last().cloned().unwrap();
        let title = harness
            .state()
            .state
            .notes_query
            .view()
            .unwrap()
            .cards()
            .find(|card| card.id == last)
            .map(|card| card.title.clone())
            .unwrap();
        harness
            .get_by(|node| {
                node.author_id() == Some("note-card-open")
                    && node.label().as_deref() == Some(title.as_str())
            })
            .click_modifiers(Modifiers::SHIFT);
        settle(&mut harness);
        assert_eq!(said(&harness, "selection-count"), "3 selected");

        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("selection-count"))
                .is_none()
        );
    }
}
