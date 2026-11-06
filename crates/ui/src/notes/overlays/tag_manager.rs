//! The library's tags, ticked then renamed, merged or deleted: one field for renaming and
//! merging, which are one operation in the core. What is asked is confirmed in place of the
//! actions, not beside them: the click that asked is where a second click would land.

use egui::{
    Color32, CornerRadius, FontFamily, Id, Margin, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    pos2, vec2,
};
use engine::notes::model::TagUsage;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Tags};
use crate::state::AppState;
use crate::state::tags::{ChangeKind, TagChange};
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, IconButton, Kind, TextField};

const WIDTH: f32 = 480.0;

/// The name typed for the ticked tags.
#[derive(Default)]
pub(in crate::notes) struct TagManagerView {
    target: String,
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
        view.target.clear();
        return;
    }
    let p = palette(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let padding = spacing(ui.ctx()).canvas.round() as i8;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(14))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(padding));
    let shown = egui::Modal::new(Id::unique("tag-manager"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| panel(ui, state, view, actions));
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
    ui.spacing_mut().item_spacing.y = 10.0;
    let closed = ui
        .horizontal(|ui| {
            let title = tr(keys::tag_manager::TITLE);
            let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
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
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for usage in tags.tags() {
            if item(ui, usage, tags.is_selected(&usage.tag), language) {
                ask(actions, Tags::Toggle(usage.tag.clone()));
            }
        }
    });
    if let Some(change) = tags.pending() {
        confirmation(ui, change, language, view, actions);
    } else {
        form(ui, state, view, actions);
    }
    closed
}

fn say(ui: &mut Ui, words: &str, colour: Color32, author_id: &str) {
    let response =
        ui.add(egui::Label::new(egui::RichText::new(words).size(11.5).color(colour)).wrap());
    widgets::address(ui, &response, author_id);
}

/// « #staging 2 notes », amber while ticked.
fn item(ui: &mut Ui, usage: &TagUsage, on: bool, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let name = format!("#{}", usage.tag);
    let count = i18n::translate(
        language,
        keys::tag_manager::NOTE_COUNT,
        &[("count", usage.note_count.into())],
    );
    let mono = FontFamily::Monospace;
    let words = widgets::text(ui, &name, 11.5, 400.0, mono.clone(), p.text_1);
    let counted = widgets::text(ui, &count, 10.5, 400.0, FontFamily::Proportional, p.text_2);
    let size = vec2(words.size().x + 6.0 + counted.size().x + 20.0, 26.0);
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
    let words = widgets::text(ui, &name, 11.5, 400.0, mono, ink);
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
    ui.add_space(4.0);
    let ticked = state.tags.selected() > 0;
    let apply = tr(if state.tags.selected() > 1 {
        keys::tag_manager::MERGE
    } else {
        keys::tag_manager::RENAME
    });
    let delete = tr(keys::tag_manager::DELETE);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
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
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let applied = Button::new(&apply, "tag-apply")
            .enabled(ticked)
            .show(ui)
            .clicked();
        if ticked && (applied || entered) && !view.target.trim().is_empty() {
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
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(12));
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 10.0;
        let text =
            ui.add(egui::Label::new(egui::RichText::new(&words).size(13.0).color(p.text_0)).wrap());
        widgets::address(ui, &text, "tag-confirm-text");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if Button::new(&tr(keys::tag_manager::CONFIRM_ACTION), "tag-confirm-apply")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
            {
                view.target.clear();
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
                engine::notes::update_note(engine, id, patch)?;
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

    fn propose(harness: &mut Harness<'_, Bench>, into: &str) {
        click_id(harness, "tag-target");
        harness
            .get_by(|node| node.author_id() == Some("tag-target"))
            .type_text(into);
        settle(harness);
        click_id(harness, "tag-apply");
    }

    fn said(harness: &Harness<'_, Bench>) -> String {
        let node = harness.get_by(|node| node.author_id() == Some("tag-confirm-text"));
        let node = node.accesskit_node();
        node.label().or_else(|| node.value()).unwrap_or_default()
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
