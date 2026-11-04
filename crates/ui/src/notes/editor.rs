//! The stand-in editor: a title and the body as plain text, over the canvas. A development tool
//! until phase 4's editors replace it (D4); a list is one task a line, `[x] ` before a done one,
//! and a Dessin's scene is not text, so only its title is edited here.

use egui::{FontFamily, FontId, Id, Margin, Stroke, TextEdit, Ui};
use engine::notes::kind::NoteKind;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::Action as Asked;
use crate::state::AppState;
use crate::state::editor::{Body, Opened, body_of};
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{IconButton, TextField, address};

/// What is being typed, for the opening it began in.
pub(super) struct Draft {
    session: u64,
    title: String,
    body: String,
    focus: bool,
}

impl Draft {
    fn of(open: &Opened) -> Self {
        Self {
            session: open.session,
            title: open.title.clone(),
            body: open.text.clone(),
            focus: true,
        }
    }
}

pub(super) fn show(ui: &Ui, state: &AppState, draft: &mut Option<Draft>, actions: &mut Actions) {
    let Some(open) = state.editor.opened().filter(|open| open.is_ready()) else {
        *draft = None;
        return;
    };
    if draft
        .as_ref()
        .is_none_or(|draft| draft.session != open.session)
    {
        *draft = Some(Draft::of(open));
    }
    let Some(editing) = draft else { return };
    let p = palette(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let padding = spacing(ui.ctx()).canvas.round() as i8;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(egui::CornerRadius::same(14))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(padding));
    let shown = egui::Modal::new(Id::unique("stand-in-editor"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| fields(ui, state, open, editing, actions));
    if shown.inner || shown.should_close() {
        save(open, editing, actions);
        actions.push(Action::Notes(Asked::CloseNote));
        *draft = None;
    }
}

/// The title and the body; `true` once the close button is clicked.
fn fields(
    ui: &mut Ui,
    state: &AppState,
    open: &Opened,
    editing: &mut Draft,
    actions: &mut Actions,
) -> bool {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let size = ui.ctx().content_rect().size();
    ui.set_width((size.x * 0.8).min(860.0));
    ui.spacing_mut().item_spacing.y = 12.0;
    let mut closed = false;
    let mut left = false;
    ui.horizontal(|ui| {
        let (label, untitled) = (tr(keys::editor::TITLE_LABEL), tr(keys::notes::UNTITLED));
        let close = tr(keys::editor::CLOSE);
        let room = ui.available_width() - 24.0 - ui.spacing().item_spacing.x;
        let title = ui
            .allocate_ui(egui::vec2(room, 40.0), |ui| {
                TextField::new(&mut editing.title, &label, "editor-title")
                    .hint(&untitled)
                    .show(ui)
            })
            .inner;
        if std::mem::take(&mut editing.focus) {
            title.request_focus();
        }
        left |= title.lost_focus();
        closed = IconButton::new(Icon::X, &close, "editor-close")
            .show(ui)
            .clicked();
    });
    if open.kind != NoteKind::Drawing {
        let p = palette(ui.ctx());
        let label = tr(keys::editor::CONTENT_LABEL);
        let rows = ((size.y * 0.6) / 18.0).max(6.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a screenful"
        )]
        let rows = rows as usize;
        let body = ui.add(
            TextEdit::multiline(&mut editing.body)
                .font(FontId::new(13.0, FontFamily::Monospace))
                .text_color(p.text_0)
                .hint_text(tr(keys::editor::EMPTY_BODY))
                .desired_rows(rows)
                .desired_width(f32::INFINITY),
        );
        // Its name only: the field's own node carries what it holds.
        ui.ctx()
            .accesskit_node_builder(body.id, |node| node.set_label(label.clone()));
        address(ui, &body, "editor-body");
        left |= body.lost_focus();
    }
    if left {
        save(open, editing, actions);
    }
    closed
}

/// What differs from the note as last saved; nothing for a field left as it was.
fn save(open: &Opened, editing: &Draft, actions: &mut Actions) {
    let title = (editing.title != open.title).then(|| editing.title.clone());
    let body: Option<Body> = (open.kind != NoteKind::Drawing && editing.body != open.text)
        .then(|| body_of(open.kind, &editing.body));
    if title.is_some() || body.is_some() {
        actions.push(Action::Notes(Asked::SaveNote {
            session: open.session,
            title,
            body,
        }));
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::model::NotePatch;

    use crate::notes::bench::{Bench, area, cards, click_id, settle};
    use crate::testing;

    fn stored(
        harness: &egui_kittest::Harness<'_, Bench>,
        id: &str,
    ) -> engine::notes::model::DisplayNote {
        let id = id.to_owned();
        engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::notes::get_note(engine, id)
        })
        .unwrap()
    }

    fn written(harness: &egui_kittest::Harness<'_, Bench>) -> usize {
        harness
            .state()
            .state
            .notes_query
            .view()
            .unwrap()
            .cards()
            .count()
    }

    #[test]
    fn a_note_opened_edited_and_closed_shows_its_change_on_its_card() {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "psql prod").id.clone();
        let mut harness = area(backend, profile);

        click_id(&mut harness, "note-card-open");
        harness.get_by(|node| node.author_id() == Some("editor-title"));
        click_id(&mut harness, "editor-body");
        harness
            .get_by(|node| node.author_id() == Some("editor-body"))
            .type_text("\nSELECT 2;");
        settle(&mut harness);
        click_id(&mut harness, "editor-close");

        assert!(
            harness
                .query_by(|node| node.author_id() == Some("editor-title"))
                .is_none()
        );
        assert_eq!(stored(&harness, &id).content, "SELECT 1;\nSELECT 2;");
        let snippet = harness
            .get_by(|node| node.author_id() == Some("note-card-snippet"))
            .accesskit_node()
            .value()
            .unwrap_or_default();
        assert!(snippet.contains("SELECT 2;"), "{snippet}");
    }

    #[test]
    fn a_title_is_written_when_its_field_is_left() {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "psql").id.clone();
        let mut harness = area(backend, profile);

        click_id(&mut harness, "note-card-open");
        harness
            .get_by(|node| node.author_id() == Some("editor-title"))
            .type_text(" prod");
        settle(&mut harness);
        click_id(&mut harness, "editor-body");

        assert_eq!(stored(&harness, &id).title, "psql prod");
        harness.get_by(|node| node.author_id() == Some("editor-title"));
    }

    #[test]
    fn a_list_is_edited_one_task_a_line() {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "groceries").id.clone();
        let patch = NotePatch {
            kind: Some(engine::notes::kind::NoteKind::Checklist),
            content: Some(String::new()),
            ..NotePatch::default()
        };
        let at = id.clone();
        engine::db::run_here(backend.engine(), move |engine| {
            engine::notes::update_note(engine, at, patch)
        })
        .unwrap();
        let mut harness = area(backend, profile);

        click_id(&mut harness, "note-card-open");
        click_id(&mut harness, "editor-body");
        harness
            .get_by(|node| node.author_id() == Some("editor-body"))
            .type_text("milk\n[x] bread");
        settle(&mut harness);
        click_id(&mut harness, "editor-close");

        let items: Vec<_> = stored(&harness, &id)
            .items
            .iter()
            .map(|item| (item.text.clone(), item.done))
            .collect();
        assert_eq!(
            items,
            [("milk".to_string(), false), ("bread".to_string(), true)]
        );
        let progress = harness
            .get_by(|node| node.author_id() == Some("note-card-progress"))
            .accesskit_node()
            .value()
            .unwrap_or_default();
        assert_eq!(progress, "1/2");
    }

    #[test]
    fn a_new_note_is_written_once_and_only_once_worth_keeping() {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "psql prod");
        let mut harness = area(backend, profile);

        click_id(&mut harness, "new-note");
        assert_eq!(written(&harness), 1);
        click_id(&mut harness, "editor-close");
        assert_eq!(written(&harness), 1);

        click_id(&mut harness, "new-note");
        harness
            .get_by(|node| node.author_id() == Some("editor-title"))
            .type_text("Rotate the certificate");
        settle(&mut harness);
        click_id(&mut harness, "editor-body");
        harness
            .get_by(|node| node.author_id() == Some("editor-body"))
            .type_text("openssl req -new");
        settle(&mut harness);
        click_id(&mut harness, "editor-close");

        assert_eq!(written(&harness), 2);
        assert_eq!(cards(&harness), 2);
    }
}
