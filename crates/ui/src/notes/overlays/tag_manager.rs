//! The library's tags, ticked then renamed, merged or deleted: one field for renaming and
//! merging, which are one operation in the core. What is asked is confirmed in place of the
//! actions, not beside them: the click that asked is where a second click would land.

use crate::theme::tokens::{font, gap, radius};
use egui::{
    Color32, CornerRadius, FontFamily, Id, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2,
    vec2,
};
use engine::notes::model::TagUsage;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Tags};
use crate::state::AppState;
use crate::state::tags::{ChangeKind, TagChange};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind, TextField};

const WIDTH: f32 = 480.0;
const CHIP: f32 = 26.0;
/// What the tags take of the window's height at most: the title stays above them and the form
/// below, whatever their number.
const LIST_SHARE: f32 = 0.45;

/// The name typed for the ticked tags.
#[derive(Default)]
pub(in crate::notes) struct TagManagerView {
    target: String,
    /// The name a tick put in the field, while nothing was typed over it: the next tick may
    /// replace it, never what was typed.
    filled: Option<String>,
    /// Focus the field and select what it holds, on the next frame it is drawn.
    select: bool,
}

impl TagManagerView {
    fn clear(&mut self) {
        self.target.clear();
        self.filled = None;
    }

    /// Ticking `tag` leaves one tag ticked, `only`: its name comes into the field, to be edited.
    fn fill(&mut self, only: &str) {
        if !self.target.is_empty() && self.filled.as_deref() != Some(self.target.as_str()) {
            return;
        }
        only.clone_into(&mut self.target);
        self.filled = Some(only.to_owned());
        self.select = true;
    }
}

fn ask(actions: &mut Actions, asked: Tags) {
    actions.push(Action::Notes(Asked::Tags(asked)));
}

pub(in crate::notes) fn show(
    ui: &Ui,
    state: &AppState,
    view: &mut TagManagerView,
    actions: &mut Actions,
) {
    if !state.tags.is_open() {
        view.clear();
        return;
    }
    let _p = palette(ui.ctx());
    let shown = crate::widgets::modal_panel(ui.ctx(), Id::unique("tag-manager"), |ui| {
        panel(ui, state, view, actions)
    });
    if shown.inner || shown.should_close() {
        ask(actions, Tags::Close);
    }
}

/// `true` once its close button is clicked.
fn panel(ui: &mut Ui, state: &AppState, view: &mut TagManagerView, actions: &mut Actions) -> bool {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = gap::INSET;
    let closed = ui
        .horizontal(|ui| {
            let title = tr(keys::tag_manager::TITLE);
            let words = widgets::text(
                ui,
                &title,
                font::TITLE,
                600.0,
                FontFamily::Proportional,
                p.text_0,
            );
            let room = ui.available_width() - widgets::MIN_TARGET;
            let (rect, _) = ui.allocate_exact_size(vec2(room, words.size().y), Sense::hover());
            ui.painter().galley(rect.min, words, p.text_0);
            IconButton::new(Icon::X, &tr(keys::tag_manager::CLOSE), "tag-manager-close")
                .show(ui)
                .clicked()
        })
        .inner;
    say(
        ui,
        &tr(keys::tag_manager::HINT),
        p.text_2,
        "tag-manager-hint",
    );
    let tags = &state.tags;
    if tags.is_loading() {
        say(
            ui,
            &tr(keys::tag_manager::LOADING),
            p.text_2,
            "tag-manager-loading",
        );
        return closed;
    }
    if tags.tags().is_empty() {
        say(
            ui,
            &tr(keys::tag_manager::EMPTY),
            p.text_2,
            "tag-manager-empty",
        );
        return closed;
    }
    list(ui, state, view, actions);
    if let Some(change) = tags.pending() {
        confirmation(ui, change, language, view, actions);
    } else {
        form(ui, state, view, actions);
    }
    closed
}

fn say(ui: &mut Ui, words: &str, colour: Color32, author_id: &str) {
    let response = ui
        .add(egui::Label::new(egui::RichText::new(words).size(font::CONTROL).color(colour)).wrap());
    widgets::address(ui, &response, author_id);
}

/// The tags as a cloud that scrolls, only its lines in view drawn.
fn list(ui: &mut Ui, state: &AppState, view: &mut TagManagerView, actions: &mut Actions) {
    let (tags, language) = (&state.tags, state.language);
    let widths: Vec<f32> = tags
        .tags()
        .iter()
        .map(|usage| chip_width(ui, usage, language))
        .collect();
    let room = ui.available_width() - ui.spacing().scroll.allocated_width();
    let lines = lines_of(&widths, room);
    ui.spacing_mut().item_spacing.y = gap::SNUG;
    egui::ScrollArea::vertical()
        .id_salt("tag-manager-list")
        .max_height(ui.ctx().content_rect().height() * LIST_SHARE)
        .auto_shrink([false, true])
        .show_rows(ui, CHIP, lines.len(), |ui, shown| {
            for line in &lines[shown] {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap::SNUG;
                    for usage in &tags.tags()[line.clone()] {
                        if item(ui, usage, tags.is_selected(&usage.tag), language) {
                            if let Some(only) = only_after_toggle(state, &usage.tag) {
                                view.fill(only);
                            }
                            ask(actions, Tags::Toggle(usage.tag.clone()));
                        }
                    }
                });
            }
        });
    ui.spacing_mut().item_spacing.y = gap::INSET;
}

/// The one tag ticked once `tag` is toggled, if one alone is.
fn only_after_toggle<'a>(state: &'a AppState, tag: &'a str) -> Option<&'a str> {
    let tags = &state.tags;
    match (tags.is_selected(tag), tags.selected()) {
        (false, 0) => Some(tag),
        (true, 2) => tags
            .tags()
            .iter()
            .map(|usage| usage.tag.as_str())
            .find(|other| *other != tag && tags.is_selected(other)),
        _ => None,
    }
}

/// Why « Rename » or « Merge » would change nothing, when it would not.
fn refusal(state: &AppState, target: &str) -> Option<i18n::Key> {
    let Some(into) = crate::api::notes::normalize_tag(target) else {
        return Some(keys::tag_manager::NO_NAME);
    };
    let tags = &state.tags;
    // Exactly: a change of case alone is written.
    let same = tags.selected() == 1
        && tags
            .tags()
            .iter()
            .any(|usage| tags.is_selected(&usage.tag) && usage.tag == into);
    same.then_some(keys::tag_manager::SAME_NAME)
}

/// The chips each line holds, filled left to right.
fn lines_of(widths: &[f32], room: f32) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut taken = 0.0;
    for (rank, width) in widths.iter().enumerate() {
        if rank > start && taken + width > room {
            lines.push(start..rank);
            start = rank;
            taken = 0.0;
        }
        taken += width + gap::SNUG;
    }
    lines.push(start..widths.len());
    lines
}

fn chip_width(ui: &Ui, usage: &TagUsage, language: i18n::Language) -> f32 {
    let (name, count) = chip_texts(ui, usage, language);
    name.size().x + 6.0 + count.size().x + 20.0
}

/// Its name, then how many notes carry it.
fn chip_texts(
    ui: &Ui,
    usage: &TagUsage,
    language: i18n::Language,
) -> (std::sync::Arc<egui::Galley>, std::sync::Arc<egui::Galley>) {
    let p = palette(ui.ctx());
    let name = format!("#{}", usage.tag);
    let count = i18n::translate(
        language,
        keys::tag_manager::NOTE_COUNT,
        &[("count", usage.note_count.into())],
    );
    (
        widgets::text(
            ui,
            &name,
            font::CONTROL,
            400.0,
            FontFamily::Monospace,
            p.text_1,
        ),
        widgets::text(
            ui,
            &count,
            font::CAPTION,
            400.0,
            FontFamily::Proportional,
            p.text_2,
        ),
    )
}

/// « #staging 2 notes », amber while ticked.
fn item(ui: &mut Ui, usage: &TagUsage, on: bool, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let name = format!("#{}", usage.tag);
    let (_, counted) = chip_texts(ui, usage, language);
    let size = vec2(chip_width(ui, usage, language), CHIP);
    let corner = CornerRadius::same(u8::MAX);
    let response = widgets::interact(
        ui,
        size,
        Sense::click(),
        "tag-item",
        || WidgetInfo::selected(Role::Button, true, on, &name),
        corner,
    );
    let lit = if on {
        1.0
    } else {
        widgets::hovered(ui, &response)
    };
    let (fill, edge, ink) = if on {
        (
            p.amber_fill.gamma_multiply(0.15),
            p.amber_fill.gamma_multiply(0.5),
            p.amber_text,
        )
    } else {
        (
            p.bg_2,
            widgets::towards(p.line_soft, p.amber_dim, lit),
            widgets::towards(p.text_1, p.text_0, lit),
        )
    };
    let rect = response.rect;
    ui.painter().rect(
        rect,
        corner,
        fill,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let words = widgets::text(ui, &name, font::CONTROL, 400.0, FontFamily::Monospace, ink);
    let left = rect.left() + 10.0;
    let width = words.size().x;
    ui.painter().galley(
        pos2(left, rect.center().y - words.size().y / 2.0),
        words,
        ink,
    );
    let count_ink = if on {
        p.amber_text.gamma_multiply(0.75)
    } else {
        p.text_2
    };
    ui.painter().galley(
        pos2(left + width + 6.0, rect.center().y - counted.size().y / 2.0),
        counted,
        count_ink,
    );
    response.clicked()
}

/// The name, then « Rename » or « Merge » and « Delete », which need a tag ticked.
fn form(ui: &mut Ui, state: &AppState, view: &mut TagManagerView, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    say(
        ui,
        &tr(keys::tag_manager::SELECTION_HINT),
        p.text_2,
        "tag-selection-hint",
    );
    let top = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), top, Stroke::new(1.0, p.line_soft));
    ui.add_space(gap::TIGHT);
    let ticked = state.tags.selected() > 0;
    let refused = refusal(state, &view.target).filter(|_| ticked);
    let apply = tr(if state.tags.selected() > 1 {
        keys::tag_manager::MERGE
    } else {
        keys::tag_manager::RENAME
    });
    let delete = tr(keys::tag_manager::DELETE);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap::CONTROLS;
        let buttons = 2.0 * 8.0 + Button::width(ui, &apply) + Button::width(ui, &delete);
        let (label, hint) = (
            tr(keys::tag_manager::TARGET_LABEL),
            tr(keys::tag_manager::TARGET_PLACEHOLDER),
        );
        let field = ui
            .allocate_ui(vec2(ui.available_width() - buttons, 32.0), |ui| {
                TextField::new(&mut view.target, &label, "tag-target")
                    .hint(&hint)
                    .mono()
                    .compact()
                    .show(ui)
            })
            .inner;
        if std::mem::take(&mut view.select) {
            select_all(ui, &field, &view.target);
        }
        if field.changed() {
            view.filled = None;
        }
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let applied = Button::new(&apply, "tag-apply")
            .enabled(ticked && refused.is_none())
            .show(ui)
            .clicked();
        if ticked && refused.is_none() && (applied || entered) {
            ask(actions, Tags::Rename(view.target.clone()));
        }
        if Button::new(&delete, "tag-delete")
            .kind(Kind::Destructive)
            .enabled(ticked)
            .show(ui)
            .clicked()
        {
            ask(actions, Tags::Delete);
        }
    });
    if let Some(refused) = refused {
        say(ui, &tr(refused), p.text_2, "tag-target-reason");
    }
}

/// The field focused, all it holds selected: what is typed replaces it.
fn select_all(ui: &Ui, field: &egui::Response, text: &str) {
    use egui::text::{CCursor, CCursorRange};
    field.request_focus();
    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), field.id).unwrap_or_default();
    let all = CCursorRange::two(CCursor::new(0), CCursor::new(text.chars().count()));
    state.cursor.set_char_range(Some(all));
    state.store(ui.ctx(), field.id);
}

/// What it would do, to how many notes, said once more before it runs.
fn confirmation(
    ui: &mut Ui,
    change: &TagChange,
    language: i18n::Language,
    view: &mut TagManagerView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let key = match change.kind {
        ChangeKind::Rename => keys::tag_manager::CONFIRM_RENAME,
        ChangeKind::Merge => keys::tag_manager::CONFIRM_MERGE,
        ChangeKind::Delete => keys::tag_manager::CONFIRM_DELETE,
    };
    let tags = u32::try_from(change.tags.len()).unwrap_or(u32::MAX);
    let first = change.tags.first().cloned().unwrap_or_default();
    let words = i18n::translate(
        language,
        key,
        &[
            ("count", change.notes.into()),
            ("tags", tags.into()),
            ("tag", first.into()),
            ("into", change.into.clone().into()),
        ],
    );
    let frame = egui::Frame::new()
        .fill(p.amber_fill.gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, p.amber_edge))
        .corner_radius(CornerRadius::same(radius::CONTROL))
        .inner_margin(gap::margin(gap::GROUPS, gap::GROUPS));
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = gap::INSET;
        let text = ui.add(
            egui::Label::new(
                egui::RichText::new(&words)
                    .size(font::READING)
                    .color(p.text_0),
            )
            .wrap(),
        );
        widgets::address(ui, &text, "tag-confirm-text");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap::CONTROLS;
            if Button::new(&tr(keys::tag_manager::CONFIRM_ACTION), "tag-confirm-apply")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
            {
                view.clear();
                ask(actions, Tags::Confirm);
            }
            if Button::new(&tr(keys::tag_manager::CONFIRM_CANCEL), "tag-confirm-cancel")
                .show(ui)
                .clicked()
            {
                ask(actions, Tags::Cancel);
            }
        });
    });
    let response = shown.response;
    response.widget_info(|| WidgetInfo::labeled(Role::Note, true, &words));
    widgets::address(ui, &response, "tag-confirm");
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::model::NotePatch;

    use crate::notes::bench::{Bench, area, click_id, settle};
    use crate::testing;

    /// alpha `staging`, beta `staging` and `urgent`, gamma `preprod`; the manager open.
    fn tagged() -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        let notes = [
            ("alpha", vec!["staging"]),
            ("beta", vec!["staging", "urgent"]),
            ("gamma", vec!["preprod"]),
        ]
        .map(|(title, tags)| (testing::note(&backend, title).id.clone(), tags));
        engine::db::run_here(backend.engine(), move |engine| {
            for (id, tags) in notes {
                let patch = NotePatch {
                    tags: Some(tags.into_iter().map(str::to_owned).collect()),
                    ..NotePatch::default()
                };
                engine::notes::update_note(engine, &id, &patch)?;
            }
            Ok::<(), engine::error::AppError>(())
        })
        .unwrap();
        let mut harness = area(backend, profile);
        click_id(&mut harness, "facets-toggle");
        click_id(&mut harness, "tag-manage");
        harness
    }

    fn listed(harness: &Harness<'_, Bench>) -> Vec<String> {
        harness
            .query_all_by(|node| node.author_id() == Some("tag-item"))
            .map(|node| node.accesskit_node().label().unwrap_or_default())
            .collect()
    }

    fn tick(harness: &mut Harness<'_, Bench>, tag: &str) {
        let name = format!("#{tag}");
        harness
            .get_by(|node| {
                node.author_id() == Some("tag-item")
                    && node.label().as_deref() == Some(name.as_str())
            })
            .click();
        settle(harness);
    }

    /// Over what the field holds: a tick fills it.
    fn propose(harness: &mut Harness<'_, Bench>, into: &str) {
        click_id(harness, "tag-target");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        harness.key_press(egui::Key::Backspace);
        harness
            .get_by(|node| node.author_id() == Some("tag-target"))
            .type_text(into);
        settle(harness);
        click_id(harness, "tag-apply");
    }

    fn field(harness: &Harness<'_, Bench>) -> String {
        let node = harness.get_by(|node| node.author_id() == Some("tag-target"));
        node.accesskit_node().value().unwrap_or_default()
    }

    fn applies(harness: &Harness<'_, Bench>) -> bool {
        let apply = harness.get_by(|node| node.author_id() == Some("tag-apply"));
        !apply.accesskit_node().is_disabled()
    }

    fn reason(harness: &Harness<'_, Bench>) -> Option<String> {
        let node = harness.query_by(|node| node.author_id() == Some("tag-target-reason"))?;
        let node = node.accesskit_node();
        node.label().or_else(|| node.value())
    }

    fn said(harness: &Harness<'_, Bench>) -> String {
        let node = harness.get_by(|node| node.author_id() == Some("tag-confirm-text"));
        let node = node.accesskit_node();
        node.label().or_else(|| node.value()).unwrap_or_default()
    }

    /// `count` tags over as many notes as hold them; the manager open.
    fn many(count: usize) -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        let tags: Vec<String> = (0..count).map(|rank| format!("tag-{rank:03}")).collect();
        for (rank, tags) in tags.chunks(10).enumerate() {
            let id = testing::note(&backend, &format!("note {rank}")).id.clone();
            let patch = NotePatch {
                tags: Some(tags.to_vec()),
                ..NotePatch::default()
            };
            engine::db::run_here(backend.engine(), move |engine| {
                engine::notes::update_note(engine, &id, &patch)
            })
            .unwrap();
        }
        let mut harness = area(backend, profile);
        click_id(&mut harness, "facets-toggle");
        click_id(&mut harness, "tag-manage");
        harness
    }

    #[test]
    fn three_hundred_tags_scroll_between_the_title_and_the_form() {
        let mut harness = many(300);
        let window = harness.ctx.content_rect();
        for author_id in ["tag-manager-close", "tag-target", "tag-apply", "tag-delete"] {
            let rect = harness
                .get_by(|node| node.author_id() == Some(author_id))
                .rect();
            assert!(window.contains_rect(rect), "{author_id} at {rect:?}");
        }
        assert!(listed(&harness).len() < 300, "only what shows is drawn");

        let first = harness
            .get_all_by(|node| node.author_id() == Some("tag-item"))
            .next()
            .unwrap()
            .rect()
            .center();
        harness.event(egui::Event::PointerMoved(first));
        for _ in 0..60 {
            harness.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -400.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
        }
        settle(&mut harness);
        assert!(listed(&harness).contains(&"#tag-299".to_owned()));
        tick(&mut harness, "tag-299");
        propose(&mut harness, "last");
        let confirm = harness
            .get_by(|node| node.author_id() == Some("tag-confirm-apply"))
            .rect();
        assert!(window.contains_rect(confirm), "{confirm:?}");
    }

    #[test]
    fn a_ticked_tag_fills_the_field_selected_and_a_second_keeps_it() {
        let mut harness = tagged();
        tick(&mut harness, "staging");
        assert_eq!(field(&harness), "staging");
        let focused = harness.get_by(|node| node.author_id() == Some("tag-target"));
        assert!(focused.accesskit_node().is_focused(), "edited in place");
        assert!(!applies(&harness), "nothing would change yet");
        assert!(reason(&harness).is_some());

        harness
            .get_by(|node| node.author_id() == Some("tag-target"))
            .type_text("-eu");
        settle(&mut harness);
        assert_eq!(field(&harness), "-eu", "typed over the selection");
        assert!(applies(&harness));

        tick(&mut harness, "urgent");
        assert_eq!(field(&harness), "-eu", "what was typed is kept");
    }

    #[test]
    fn a_field_emptied_or_holding_no_name_says_why_nothing_applies() {
        let mut harness = tagged();
        tick(&mut harness, "staging");
        for typed in ["", "#", "# staging "] {
            propose(&mut harness, typed);
            assert!(!applies(&harness), "{typed:?} changes nothing");
            assert!(reason(&harness).is_some(), "{typed:?} is said");
            assert!(
                harness
                    .query_by(|node| node.author_id() == Some("tag-confirm-text"))
                    .is_none()
            );
        }
        propose(&mut harness, "#recette");
        assert!(said(&harness).contains("recette"), "{}", said(&harness));
    }

    #[test]
    fn nothing_ticked_applies_nothing() {
        let harness = tagged();
        assert_eq!(listed(&harness), ["#preprod", "#staging", "#urgent"]);
        let apply = harness.get_by(|node| node.author_id() == Some("tag-apply"));
        assert!(apply.accesskit_node().is_disabled());
    }

    #[test]
    fn a_rename_says_what_it_touches_and_cancelling_writes_nothing() {
        let mut harness = tagged();
        tick(&mut harness, "staging");
        propose(&mut harness, "recette");
        assert!(
            said(&harness).contains('2'),
            "two notes carry it: {}",
            said(&harness)
        );

        click_id(&mut harness, "tag-confirm-cancel");
        assert_eq!(listed(&harness), ["#preprod", "#staging", "#urgent"]);

        click_id(&mut harness, "tag-apply");
        click_id(&mut harness, "tag-confirm-apply");
        assert_eq!(listed(&harness), ["#preprod", "#recette", "#urgent"]);
    }

    #[test]
    fn onto_an_existing_tag_it_merges() {
        let mut harness = tagged();
        tick(&mut harness, "preprod");
        propose(&mut harness, "staging");
        click_id(&mut harness, "tag-confirm-apply");

        assert_eq!(listed(&harness), ["#staging", "#urgent"]);
    }

    #[test]
    fn a_deleted_tag_leaves_its_notes() {
        let mut harness = tagged();
        tick(&mut harness, "urgent");
        click_id(&mut harness, "tag-delete");
        click_id(&mut harness, "tag-confirm-apply");

        assert_eq!(listed(&harness), ["#preprod", "#staging"]);
        assert_eq!(crate::notes::bench::cards(&harness), 3);
    }
}
