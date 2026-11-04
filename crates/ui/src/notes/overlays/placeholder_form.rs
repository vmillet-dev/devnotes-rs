//! The form a note with `{{fields}}` asks before it is copied: one row a field, seeded with the
//! values the note stores. An empty field shows its default as a suggestion and never copies
//! it: the day the snippet's default changes, a stored copy would win.

use std::collections::BTreeMap;

use egui::{CornerRadius, FontFamily, Id, Margin, Sense, Stroke, Ui, vec2};
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;
use engine::notes::placeholder::Placeholder;
use i18n::{Language, keys};

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Fill};
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, Kind, TextField};

const WIDTH: f32 = 420.0;
const NAME_WIDTH: f32 = 120.0;

/// What is being typed, for the note it was opened on.
#[derive(Default)]
pub(in crate::notes) struct FillView {
    note: Option<String>,
    values: Vec<String>,
    focus: bool,
}

fn ask(actions: &mut Actions, asked: Fill) {
    actions.push(Action::Notes(Asked::Fill(asked)));
}

pub(in crate::notes) fn show(
    ui: &Ui,
    state: &AppState,
    view: &mut FillView,
    actions: &mut Actions,
) {
    let Some(note) = state.fill.target() else {
        *view = FillView::default();
        return;
    };
    if view.note.as_deref() != Some(note.id.as_str()) {
        *view = FillView {
            note: Some(note.id.clone()),
            values: note
                .placeholders
                .iter()
                .map(|field| field.value.clone())
                .collect(),
            focus: true,
        };
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
    let shown = egui::Modal::new(Id::unique("placeholder-form"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| form(ui, state, note, view));
    let closing = shown.should_close().then_some(Fill::Cancel);
    let asked = shown.inner.or(closing);
    if let Some(asked) = asked {
        ask(actions, asked);
    }
}

fn form(ui: &mut Ui, state: &AppState, note: &DisplayNote, view: &mut FillView) -> Option<Fill> {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 12.0;
    let title = tr(keys::placeholders::TITLE);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, _) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    // A Note holding a field was called a snippet.
    let hint = if note.kind == NoteKind::Snippet {
        keys::placeholders::HINT
    } else {
        keys::placeholders::HINT_NOTE
    };
    let count = u32::try_from(note.placeholders.len()).unwrap_or(u32::MAX);
    let hint = i18n::translate(language, hint, &[("count", count.into())]);
    let said =
        ui.add(egui::Label::new(egui::RichText::new(hint).size(11.5).color(p.text_2)).wrap());
    widgets::address(ui, &said, "placeholder-form-hint");
    let mut entered = false;
    // The first field still empty is where typing goes: Enter then copies what was typed.
    let first = view.values.iter().position(String::is_empty).unwrap_or(0);
    for (at, field) in note.placeholders.iter().enumerate() {
        let Some(value) = view.values.get_mut(at) else {
            continue;
        };
        let input = field_row(ui, field, value, language);
        if view.focus && at == first {
            input.request_focus();
            view.focus = false;
        }
        entered |= input.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    }
    let top = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), top, Stroke::new(1.0, p.line_soft));
    let mut asked = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::placeholders::COPY), "placeholder-submit")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
            || entered
        {
            asked = Some(Fill::Submit(values(note, &view.values)));
        }
        if Button::new(&tr(keys::placeholders::CANCEL), "placeholder-cancel")
            .show(ui)
            .clicked()
        {
            asked = Some(Fill::Cancel);
        }
        if Button::new(&tr(keys::placeholders::COPY_RAW), "placeholder-copy-raw")
            .show(ui)
            .clicked()
        {
            asked = Some(Fill::Raw);
        }
    });
    asked
}

/// A field's row, here and in the editor's panel. An empty one shows its default, which it never
/// holds.
pub(in crate::notes) fn field_row(
    ui: &mut Ui,
    field: &Placeholder,
    value: &mut String,
    language: Language,
) -> egui::Response {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        // The braces tie the row to the token in the text; the field's name is its label.
        let name = format!("{{{{{}}}}}", field.name);
        let galley = widgets::text(ui, &name, 12.0, 400.0, FontFamily::Monospace, p.amber_text);
        let (rect, _) = ui.allocate_exact_size(vec2(NAME_WIDTH, 28.0), Sense::hover());
        let at_y = rect.center().y - galley.size().y / 2.0;
        ui.painter().with_clip_rect(rect).galley(
            egui::pos2(rect.left(), at_y),
            galley,
            p.amber_text,
        );
        let label = i18n::translate(
            language,
            keys::placeholders::FIELD_LABEL,
            &[("name", field.name.as_str().into())],
        );
        TextField::new(value, &label, "placeholder-input")
            .hint(&field.default_value)
            .compact()
            .show(ui)
    })
    .inner
}

/// Every field by name, raw: `notes::placeholder::fill` decides what an empty one is worth.
fn values(note: &DisplayNote, typed: &[String]) -> BTreeMap<String, String> {
    note.placeholders
        .iter()
        .zip(typed)
        .map(|(field, value)| (field.name.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use engine::notes::model::NotePatch;

    use crate::notes::bench::{Bench, area, click_id, settle};
    use crate::testing;

    const BODY: &str = "psql -h {{host}} -p {{port=5432}} -U {{user}}";

    /// A snippet with three fields, its form open from the card.
    fn opened() -> (Harness<'static, Bench>, String) {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "psql").id.clone();
        let patch = NotePatch {
            content: Some(BODY.into()),
            ..NotePatch::default()
        };
        let edited = id.clone();
        engine::db::run_here(backend.engine(), move |engine| {
            engine::notes::update_note(engine, edited, patch)
        })
        .unwrap();
        let mut harness = area(backend, profile);
        harness
            .get_by(|node| node.author_id() == Some("note-card-open"))
            .hover();
        settle(&mut harness);
        click_id(&mut harness, "note-card-fill");
        (harness, id)
    }

    /// `author_id` clicked, then what reached the clipboard: the frame that copies is not the
    /// last one a settle leaves, so each frame's output is read.
    fn copy_from(harness: &mut Harness<'_, Bench>, author_id: &str) -> Option<String> {
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .click();
        for _ in 0..60 {
            harness.step();
            harness.state().backend.settle();
            let copied =
                harness.output().platform_output.commands.iter().find_map(
                    |command| match command {
                        egui::OutputCommand::CopyText(text) => Some(text.clone()),
                        _ => None,
                    },
                );
            if copied.is_some() {
                settle(harness);
                return copied;
            }
        }
        None
    }

    fn stored(harness: &Harness<'_, Bench>, id: &str) -> Vec<(String, String)> {
        let id = id.to_owned();
        let note = engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::notes::get_note(engine, id)
        })
        .unwrap();
        note.placeholders
            .into_iter()
            .map(|field| (field.name, field.value))
            .collect()
    }

    #[test]
    fn the_card_asks_for_the_fields_the_first_empty_one_focused() {
        let (harness, _) = opened();
        let inputs: Vec<_> = harness
            .query_all_by(|node| node.author_id() == Some("placeholder-input"))
            .collect();
        assert_eq!(inputs.len(), 3);
        assert!(inputs[0].is_focused());
    }

    #[test]
    fn copied_filled_the_typed_values_are_kept() {
        let (mut harness, id) = opened();
        harness
            .get_all_by(|node| node.author_id() == Some("placeholder-input"))
            .next()
            .unwrap()
            .type_text("db.internal");
        settle(&mut harness);
        let copied = copy_from(&mut harness, "placeholder-submit");

        assert_eq!(copied.as_deref(), Some("psql -h db.internal -p 5432 -U "));
        let host = stored(&harness, &id)
            .into_iter()
            .find(|(name, _)| name == "host");
        assert_eq!(host.map(|(_, value)| value).as_deref(), Some("db.internal"));
    }

    #[test]
    fn as_is_copies_the_tokens_and_cancel_writes_nothing() {
        let (mut harness, id) = opened();
        let copied = copy_from(&mut harness, "placeholder-copy-raw");
        assert_eq!(copied.as_deref(), Some(BODY));

        harness
            .get_by(|node| node.author_id() == Some("note-card-open"))
            .hover();
        settle(&mut harness);
        click_id(&mut harness, "note-card-fill");
        harness
            .get_all_by(|node| node.author_id() == Some("placeholder-input"))
            .next()
            .unwrap()
            .type_text("discarded");
        click_id(&mut harness, "placeholder-cancel");

        assert!(
            stored(&harness, &id)
                .iter()
                .all(|(_, value)| value.is_empty())
        );
        assert!(!harness.state().state.fill.is_open());
    }
}
