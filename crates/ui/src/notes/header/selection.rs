//! The header's second look while cards are ticked: what the next click acts on, in place of the
//! search and its filters. Same height, so the canvas neither moves nor hides under it. What does
//! not hold in the row waits under « … », the rarest first; its menus are commands, naming what
//! they do rather than a value chosen.

use crate::theme::tokens::{font, gap};
use egui::{FontFamily, Role, Sense, Ui, WidgetInfo, vec2};
use engine::notes::priority::Priority;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::Batch;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, IconButton, Kind, MenuItem, Naming, TextField};

const LEVELS: [Priority; 5] = [
    Priority::None,
    Priority::Low,
    Priority::Medium,
    Priority::High,
    Priority::Urgent,
];

/// No entry chosen: the menus name what they do.
const NONE: usize = usize::MAX;
const GAP: f32 = gap::CONTROLS;
const TAG_WIDTH: f32 = 120.0;

/// What only the row's drawing sees.
#[derive(Default)]
pub(super) struct SelectionView {
    tag_draft: String,
    confirm_delete: bool,
    /// The tag field asked from « … » when the row had no room for it: it stands in for the
    /// trash until it lets the focus go.
    tagging: bool,
    focus_tag: bool,
}

impl SelectionView {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// A card on screen, the list's or the board's.
struct Shown<'a> {
    id: &'a str,
    pinned: bool,
    tags: &'a [String],
}

fn shown(state: &AppState) -> Vec<Shown<'_>> {
    if state.board_place().is_some() {
        return state
            .board
            .cards()
            .map(|entry| Shown {
                id: &entry.note.id,
                pinned: entry.note.pinned,
                tags: &entry.note.tags,
            })
            .collect();
    }
    state
        .notes_query
        .view()
        .map(|view| {
            view.cards()
                .map(|card| Shown {
                    id: &card.id,
                    pinned: card.pinned,
                    tags: &card.tags,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The notes ticked among those shown, in their order.
pub(super) fn ticked(state: &AppState) -> Vec<String> {
    let order: Vec<&str> = shown(state).iter().map(|card| card.id).collect();
    state.selection.checked_in(&order)
}

fn batch(actions: &mut Actions, batch: Batch) {
    actions.push(Action::Notes(crate::notes::Action::Batch(batch)));
}

/// The controls that give way, in the order they are kept: the row holds the first `kept`.
const TAG: usize = 0;
const PIN: usize = 1;
const MOVE: usize = 2;
const FILE: usize = 3;

const fn holds(kept: usize, rank: usize) -> bool {
    rank < kept
}

/// The row, `room` across; `checked` is never empty.
pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (view, checked): (&mut SelectionView, &[String]),
    room: f32,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let cards = shown(state);
    let ticked: Vec<&Shown<'_>> = cards
        .iter()
        .filter(|card| checked.iter().any(|id| id == card.id))
        .collect();
    let all_pinned = ticked.iter().all(|card| card.pinned);
    let pin = tr(if all_pinned {
        keys::selection::UNPIN
    } else {
        keys::selection::PIN
    });
    let kept = kept(ui, state, (view, &pin, checked.len()), room);
    ui.spacing_mut().item_spacing = vec2(GAP, 0.0);
    count(ui, checked.len(), language);
    if holds(kept, MOVE) {
        move_to(ui, state, actions);
    }
    if holds(kept, FILE) {
        file_into(ui, state, actions);
    }
    if holds(kept, TAG) || view.tagging {
        tag_field(ui, view, actions, language);
    }
    if holds(kept, PIN) && Button::new(&pin, "selection-pin").show(ui).clicked() {
        batch(actions, Batch::Pin(!all_pinned));
    }
    if !view.tagging {
        delete(ui, view, actions, language);
    }
    more(ui, state, (view, &ticked, &pin, all_pinned), kept, actions);
    let clear = tr(keys::selection::CLEAR);
    if IconButton::new(Icon::X, &clear, "selection-clear")
        .show(ui)
        .clicked()
    {
        actions.push(Action::Notes(crate::notes::Action::ClearSelection));
    }
}

/// The count, the trash, « … » and the cross always; then, while they hold, the tag field, the
/// pin, the move and the filing — given up the other way round. A move with no space to go to,
/// a filing with no folder, take no room.
fn kept(
    ui: &Ui,
    state: &AppState,
    (view, pin, count): (&SelectionView, &str, usize),
    room: f32,
) -> usize {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let ink = palette(ui.ctx()).text_1;
    let width = |label: &str, size: f32, weight: f32| {
        widgets::text(ui, label, size, weight, FontFamily::Proportional, ink)
            .size()
            .x
    };
    let menu = |key| (width(&tr(key), 11.5, 400.0) + 30.0).min(190.0);
    let button = |label: &str| width(label, 12.0, 400.0) + 24.0;
    let delete =
        button(&tr(keys::selection::DELETE)).max(button(&tr(keys::selection::DELETE_CONFIRM)));
    let counted = width(&count_words(count, language), 12.5, 600.0) + 4.0;
    let icons = 2.0 * (widgets::MIN_TARGET + 8.0);
    // While it is asked for, the tag field takes the trash's place.
    let (tag, delete) = if view.tagging {
        (0.0, TAG_WIDTH)
    } else {
        (TAG_WIDTH, delete)
    };
    let mut left = room - counted - icons - delete - 3.0 * GAP;
    let wanted = [
        tag,
        button(pin),
        if state.spaces.spaces().is_empty() {
            0.0
        } else {
            menu(keys::selection::MOVE_TO)
        },
        if folders(state).is_empty() {
            0.0
        } else {
            menu(keys::selection::FILE_INTO)
        },
    ];
    wanted
        .iter()
        .take_while(|wanted| {
            left -= *wanted + GAP;
            left >= 0.0
        })
        .count()
}

/// The folders of the space shown, or of every space.
fn folders(state: &AppState) -> Vec<&engine::folders::model::Folder> {
    let shown_space = state.spaces.active_space();
    state
        .folders
        .list()
        .iter()
        .filter(|folder| shown_space.is_none_or(|space| folder.space_id == space))
        .collect()
}

fn move_to(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    if state.spaces.spaces().is_empty() {
        return;
    }
    let label = i18n::translate(state.language, keys::selection::MOVE_TO, &[]);
    let spaces: Vec<(usize, &str)> = state
        .spaces
        .spaces()
        .iter()
        .enumerate()
        .map(|(at, space)| (at, space.name.as_str()))
        .collect();
    let mut picked = NONE;
    if ChoiceMenu::new(&label, "selection-move", &spaces)
        .naming(Naming::Label)
        .show(ui, &mut picked)
        .changed()
        && let Some(space) = state.spaces.spaces().get(picked)
    {
        batch(actions, Batch::MoveTo(space.id.clone()));
    }
}

/// Into a folder of the space shown, or out of every one.
fn file_into(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let folders = folders(state);
    if folders.is_empty() {
        return;
    }
    let unfile = i18n::translate(language, keys::selection::UNFILE, &[]);
    let mut options: Vec<(usize, &str)> = folders
        .iter()
        .enumerate()
        .map(|(at, folder)| (at, folder.name.as_str()))
        .collect();
    options.push((folders.len(), unfile.as_str()));
    let mut picked = NONE;
    let label = i18n::translate(language, keys::selection::FILE_INTO, &[]);
    if ChoiceMenu::new(&label, "selection-file", &options)
        .naming(Naming::Label)
        .show(ui, &mut picked)
        .changed()
    {
        let folder = folders.get(picked).map(|folder| folder.id.clone());
        batch(actions, Batch::File(folder));
    }
}

fn count_words(count: usize, language: i18n::Language) -> String {
    #[expect(clippy::cast_possible_truncation, reason = "a selection on screen")]
    let count = count as u32;
    i18n::translate(language, keys::selection::COUNT, &[("count", count.into())])
}

/// « 2 selected »: the one thing saying how many notes the next click acts on.
fn count(ui: &mut Ui, count: usize, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = count_words(count, language);
    let galley = widgets::text(
        ui,
        &words,
        font::BODY,
        600.0,
        FontFamily::Proportional,
        p.amber_text,
    );
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(4.0, 0.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "selection-count");
    ui.painter().galley(rect.min, galley, p.amber_text);
}

/// A tag typed then Enter, onto every ticked note.
fn tag_field(
    ui: &mut Ui,
    view: &mut SelectionView,
    actions: &mut Actions,
    language: i18n::Language,
) {
    let tr = |key| i18n::translate(language, key, &[]);
    let (label, hint) = (
        tr(keys::selection::ADD_TAG_LABEL),
        tr(keys::selection::ADD_TAG_PLACEHOLDER),
    );
    let field = ui
        .allocate_ui(vec2(TAG_WIDTH, 32.0), |ui| {
            TextField::new(&mut view.tag_draft, &label, "selection-tag")
                .hint(&hint)
                .compact()
                .show(ui)
        })
        .inner;
    if std::mem::take(&mut view.focus_tag) {
        field.request_focus();
    }
    if !field.lost_focus() {
        return;
    }
    view.tagging = false;
    let tag = view.tag_draft.trim().to_owned();
    if ui.input(|input| input.key_pressed(egui::Key::Enter)) && !tag.is_empty() {
        batch(actions, Batch::Tag(tag));
        view.tag_draft.clear();
    }
}

/// Two clicks: the first arms it.
fn delete(ui: &mut Ui, view: &mut SelectionView, actions: &mut Actions, language: i18n::Language) {
    let key = if view.confirm_delete {
        keys::selection::DELETE_CONFIRM
    } else {
        keys::selection::DELETE
    };
    let delete = i18n::translate(language, key, &[]);
    let deleting = Button::new(&delete, "selection-delete")
        .kind(Kind::Destructive)
        .armed(view.confirm_delete)
        .show(ui);
    if deleting.clicked() {
        if view.confirm_delete {
            batch(actions, Batch::Delete);
        }
        view.confirm_delete = !view.confirm_delete;
    }
}

/// Entries named `words`, addressed `prefix-<rank>`.
fn named<'a>(words: impl Iterator<Item = &'a str>, prefix: &str) -> Vec<(String, String)> {
    words
        .enumerate()
        .map(|(at, words)| (words.to_owned(), format!("{prefix}-{at}")))
        .collect()
}

fn entries(named: &[(String, String)]) -> Vec<MenuItem<'_>> {
    named
        .iter()
        .map(|(words, author_id)| MenuItem::new(words, author_id))
        .collect()
}

/// « … »: the priority, the tags to take off, the copy, and what the row had no room for.
fn more(
    ui: &mut Ui,
    state: &AppState,
    (view, ticked, pin, all_pinned): (&mut SelectionView, &[&Shown<'_>], &str, bool),
    kept: usize,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let spaces = state.spaces.spaces();
    let folders = folders(state);
    let unfile = tr(keys::selection::UNFILE);
    let levels: Vec<String> = LEVELS.iter().map(|level| tr(level_key(*level))).collect();
    let mut carried: Vec<&str> = ticked
        .iter()
        .flat_map(|card| card.tags.iter().map(String::as_str))
        .collect();
    carried.sort_unstable();
    carried.dedup();
    let moves = named(
        spaces.iter().map(|space| space.name.as_str()),
        "selection-more-move",
    );
    let filings = named(
        folders
            .iter()
            .map(|folder| folder.name.as_str())
            .chain([unfile.as_str()]),
        "selection-more-file",
    );
    let priorities = named(levels.iter().map(String::as_str), "selection-more-priority");
    let untags = named(carried.iter().copied(), "selection-more-untag");
    let (moves_in, filings_in) = (entries(&moves), entries(&filings));
    let (priorities_in, untags_in) = (entries(&priorities), entries(&untags));
    let labels = [
        tr(keys::selection::MOVE_TO),
        tr(keys::selection::FILE_INTO),
        tr(keys::selection::ADD_TAG),
        tr(keys::selection::PRIORITY),
        tr(keys::selection::REMOVE_TAG),
        tr(keys::selection::COPY_MARKDOWN),
    ];
    let mut items = Vec::new();
    if !holds(kept, MOVE) && !spaces.is_empty() {
        items.push(MenuItem::new(&labels[0], "selection-more-move").children(&moves_in));
    }
    if !holds(kept, FILE) && !folders.is_empty() {
        items.push(MenuItem::new(&labels[1], "selection-more-file").children(&filings_in));
    }
    if !holds(kept, TAG) && !view.tagging {
        items.push(MenuItem::new(&labels[2], "selection-more-tag"));
    }
    if !holds(kept, PIN) {
        items.push(MenuItem::new(pin, "selection-more-pin"));
    }
    items.push(MenuItem::new(&labels[3], "selection-more-priority").children(&priorities_in));
    if !carried.is_empty() {
        items.push(MenuItem::new(&labels[4], "selection-more-untag").children(&untags_in));
    }
    items.push(MenuItem::new(&labels[5], "selection-more-copy"));
    let label = tr(keys::selection::MORE);
    let trigger = IconButton::new(Icon::Ellipsis, &label, "selection-more").show(ui);
    let Some(picked) = widgets::menu(&trigger, &items) else {
        return;
    };
    let rank = |prefix: &str| {
        picked
            .strip_prefix(prefix)
            .and_then(|rest| rest.parse::<usize>().ok())
    };
    if let Some(space) = rank("selection-more-move-").and_then(|at| spaces.get(at)) {
        batch(actions, Batch::MoveTo(space.id.clone()));
    } else if let Some(at) = rank("selection-more-file-") {
        let folder = folders.get(at).map(|folder| folder.id.clone());
        batch(actions, Batch::File(folder));
    } else if let Some(level) = rank("selection-more-priority-").and_then(|at| LEVELS.get(at)) {
        batch(actions, Batch::Prioritise(*level));
    } else if let Some(tag) = rank("selection-more-untag-").and_then(|at| carried.get(at)) {
        batch(actions, Batch::Untag((*tag).to_owned()));
    } else if picked == "selection-more-tag" {
        view.tagging = true;
        view.focus_tag = true;
    } else if picked == "selection-more-pin" {
        batch(actions, Batch::Pin(!all_pinned));
    } else if picked == "selection-more-copy" {
        actions.push(Action::Menu(crate::titlebar::Action::CopyMarkdown));
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
            engine::notes::get_note(engine, &id)
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
                .query_by(|node| node.author_id() == Some("selection-count"))
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

    fn card_box(harness: &Harness<'_, Bench>, title: &str) -> egui::accesskit::Rect {
        harness
            .get_by(|node| {
                node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
            })
            .accesskit_node()
            .bounding_box()
            .unwrap()
    }

    #[test]
    fn ticking_a_card_moves_no_card_and_covers_none() {
        let (mut harness, _) = three();
        let before = card_box(&harness, "one").y0;

        ctrl_click(&mut harness, "two");

        let first = card_box(&harness, "one").y0;
        assert!((first - before).abs() < 0.5, "{before} → {first}");
        let row = harness
            .get_by(|node| node.author_id() == Some("selection-clear"))
            .accesskit_node()
            .bounding_box()
            .unwrap();
        assert!(
            row.y1 < first,
            "the row ends at {}, the cards start at {first}",
            row.y1
        );
    }

    #[test]
    fn in_a_narrow_window_what_does_not_hold_is_under_more() {
        let (mut harness, [one, ..]) = three();
        let archive = crate::notes::bench::spaces_of(&harness)
            .into_iter()
            .find(|space| space.name == "Archive")
            .unwrap();
        harness.set_size(egui::vec2(640.0, 700.0));
        settle(&mut harness);
        ctrl_click(&mut harness, "one");
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("selection-move"))
                .is_none()
        );

        click_id(&mut harness, "selection-more");
        click_id(&mut harness, "selection-more-move");
        harness
            .get_by(|node| {
                node.author_id()
                    .is_some_and(|id| id.starts_with("selection-more-move-"))
                    && node.label().as_deref() == Some("Archive")
            })
            .click();
        settle(&mut harness);

        assert_eq!(stored(&harness, &one).space_id, archive.id);
    }
}
