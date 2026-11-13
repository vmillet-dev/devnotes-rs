//! The trash: what was deleted, when, and when the retention erases it; restored one at a time
//! or all that the search found, purged for good in two clicks. Ages without a round trip: the
//! times are worded as it draws.

use chrono::{DateTime, Utc};
use egui::{
    CornerRadius, FontFamily, Id, Margin, Rect, Role, Sense, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};
use engine::notes::kind::NoteKind;
use engine::notes::trash::TrashedNote;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Trash};
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, IconButton, Kind, TextField};

const WIDTH: f32 = 560.0;
/// Every row as tall: the list draws only those in view.
const ROW: f32 = 84.0;
const GAP: f32 = 6.0;
const SNIPPET_LINES: usize = 2;

#[derive(Default)]
pub(in crate::notes) struct TrashView {
    /// The row awaiting its second click.
    confirming_purge: Option<String>,
    confirming_empty: bool,
    /// Searching or not, last frame: the count the warning announced no longer holds once it
    /// changes.
    searching: bool,
}

pub(in crate::notes) fn show(
    ui: &Ui,
    state: &AppState,
    view: &mut TrashView,
    actions: &mut Actions,
) {
    if !state.trash.is_open() {
        *view = TrashView::default();
        return;
    }
    let searching = state.trash.searching();
    if searching != view.searching {
        view.searching = searching;
        view.confirming_empty = false;
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
    let shown = egui::Modal::new(Id::unique("trash-panel"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| panel(ui, state, view, actions));
    if shown.inner || shown.should_close() {
        actions.push(Action::Notes(Asked::Trash(Trash::Close)));
    }
}

/// `true` once its close button is clicked.
fn panel(ui: &mut Ui, state: &AppState, view: &mut TrashView, actions: &mut Actions) -> bool {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let closed = ui
        .horizontal(|ui| {
            let title = tr(keys::trash::TITLE);
            let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
            let room = ui.available_width() - widgets::MIN_TARGET;
            let (rect, _) = ui.allocate_exact_size(vec2(room, words.size().y), Sense::hover());
            ui.painter().galley(rect.min, words, p.text_0);
            IconButton::new(Icon::X, &tr(keys::trash::CLOSE), "trash-close")
                .show(ui)
                .clicked()
        })
        .inner;
    say(ui, &tr(keys::trash::RETENTION_HINT), "trash-hint");
    let notes = state.trash.notes();
    if !notes.is_empty() || state.trash.searching() {
        let mut typed = state.trash.search().to_owned();
        let (label, hint) = (
            tr(keys::trash::SEARCH_LABEL),
            tr(keys::trash::SEARCH_PLACEHOLDER),
        );
        if TextField::new(&mut typed, &label, "trash-search")
            .hint(&hint)
            .show(ui)
            .changed()
        {
            actions.push(Action::Notes(Asked::Trash(Trash::Search(typed))));
        }
    }
    if state.trash.is_loading() {
        say(ui, &tr(keys::trash::LOADING), "trash-loading");
    } else if notes.is_empty() {
        let key = if state.trash.searching() {
            keys::trash::NO_MATCH
        } else {
            keys::trash::EMPTY
        };
        say(ui, &tr(key), "trash-empty-state");
    } else {
        list(ui, state, notes, view, actions);
        footer(ui, state, notes.len(), view, actions);
    }
    closed
}

/// A line of quiet text, read by its name.
fn say(ui: &mut Ui, words: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let response =
        ui.add(egui::Label::new(egui::RichText::new(words).size(12.0).color(p.text_2)).wrap());
    widgets::address(ui, &response, author_id);
}

fn list(
    ui: &mut Ui,
    state: &AppState,
    notes: &[TrashedNote],
    view: &mut TrashView,
    actions: &mut Actions,
) {
    #[expect(clippy::cast_precision_loss, reason = "rows on a screen")]
    let whole = notes.len() as f32 * (ROW + GAP) - GAP;
    let tallest = (ui.ctx().content_rect().height() * 0.5).max(ROW * 2.0);
    // As tall as it will be from the first frame: grown from the last one's content, the
    // modal crept up the screen for a few frames and a click landed where it had been.
    let height = whole.min(tallest);
    ui.spacing_mut().item_spacing.y = GAP;
    egui::ScrollArea::vertical()
        .min_scrolled_height(height)
        .max_height(height)
        .auto_shrink([false, true])
        .show_rows(ui, ROW, notes.len(), |ui, range| {
            for note in &notes[range] {
                row(ui, state, note, view, actions);
            }
        });
    ui.spacing_mut().item_spacing.y = 10.0;
}

fn row(
    ui: &mut Ui,
    state: &AppState,
    note: &TrashedNote,
    view: &mut TrashView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW), Sense::hover());
    let title = if note.note.title.trim().is_empty() {
        tr(keys::trash::UNTITLED)
    } else {
        note.note.title.clone()
    };
    response.widget_info(|| WidgetInfo::labeled(Role::ListItem, true, &title));
    widgets::address(ui, &response, "trash-row");
    let corner = CornerRadius::same(8);
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(vec2(11.0, 9.0));
    let left = buttons(ui, inner, state, &note.note.id, view, actions);
    let text = Rect::from_min_max(inner.min, pos2(left - 12.0, inner.bottom()));
    let mut words = ui.new_child(UiBuilder::new().max_rect(text));
    words.set_clip_rect(text.intersect(ui.clip_rect()));
    words.spacing_mut().item_spacing.y = 4.0;
    line(
        &mut words,
        &title,
        12.5,
        FontFamily::Proportional,
        p.text_0,
        "trash-row-title",
        1,
    );
    let preview = if note.note.kind == NoteKind::Checklist {
        tr(keys::trash::CHECKLIST_NOTE)
    } else {
        note.preview.clone()
    };
    let mono = if note.note.kind == NoteKind::Checklist {
        FontFamily::Proportional
    } else {
        FontFamily::Monospace
    };
    line(
        &mut words,
        &preview,
        11.0,
        mono,
        p.text_2,
        "trash-row-snippet",
        SNIPPET_LINES,
    );
    let meta = meta(ui, state, note.deleted_at, note.purge_at);
    line(
        &mut words,
        &meta,
        11.0,
        FontFamily::Proportional,
        p.text_2,
        "trash-row-meta",
        1,
    );
}

/// Restore, and the purge in two clicks, at the row's right; answers where they begin.
fn buttons(
    ui: &mut Ui,
    inner: Rect,
    state: &AppState,
    id: &str,
    view: &mut TrashView,
    actions: &mut Actions,
) -> f32 {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let mut buttons = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Min)),
    );
    buttons.spacing_mut().item_spacing.x = 6.0;
    let confirming = view.confirming_purge.as_deref() == Some(id);
    let purge = tr(if confirming {
        keys::trash::PURGE_CONFIRM
    } else {
        keys::trash::PURGE
    });
    if Button::new(&purge, "trash-purge")
        .kind(Kind::Destructive)
        .armed(confirming)
        .show(&mut buttons)
        .clicked()
    {
        if confirming {
            view.confirming_purge = None;
            actions.push(Action::Notes(Asked::Trash(Trash::Purge(id.to_owned()))));
        } else {
            view.confirming_purge = Some(id.to_owned());
        }
    }
    // Kept apart from the purge, which it must not look like.
    buttons.add_space(8.0);
    if Button::new(&tr(keys::trash::RESTORE), "trash-restore")
        .show(&mut buttons)
        .clicked()
    {
        actions.push(Action::Notes(Asked::Trash(Trash::Restore(id.to_owned()))));
    }
    buttons.min_rect().left()
}

fn line(
    ui: &mut Ui,
    words: &str,
    size: f32,
    family: FontFamily,
    colour: egui::Color32,
    author_id: &str,
    rows: usize,
) {
    // Cut at `rows`, an ellipsis after: every row is as tall, or the list could not skip rows.
    let mut job = egui::text::LayoutJob::simple(
        words.to_owned(),
        egui::FontId::new(size, family),
        colour,
        ui.available_width(),
    );
    job.wrap.max_rows = rows;
    let galley = ui.painter().layout_job(job);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    widgets::address(ui, &response, author_id);
    ui.painter().galley(rect.min, galley, colour);
}

/// « deleted 3 min ago · erased in 30d », and drawn again when the first part reads otherwise.
fn meta(ui: &Ui, state: &AppState, deleted: DateTime<Utc>, purge: DateTime<Utc>) -> String {
    let language = state.language;
    let (ago, wait) = crate::services::time::relative(deleted, state.clock.as_ref());
    ui.ctx().request_repaint_after(wait);
    let deleted = i18n::translate(
        language,
        keys::trash::DELETED_AT,
        &[("when", ago.text(language).into())],
    );
    let purges = match days_left(purge, state.clock.now()) {
        0 => i18n::translate(language, keys::trash::PURGES_TODAY, &[]),
        days => i18n::translate(language, keys::trash::PURGES_IN, &[("count", days.into())]),
    };
    format!("{deleted} · {purges}")
}

/// Rounded to the nearest day: up said « erased in 31 d » under « kept 30 days » for a note
/// just deleted, down said 29.
fn days_left(purge: DateTime<Utc>, now: DateTime<Utc>) -> u32 {
    const DAY: f64 = 86_400_000.0;
    #[expect(clippy::cast_precision_loss, reason = "days")]
    let days = ((purge - now).num_milliseconds() as f64 / DAY).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "thirty days at most, none past"
    )]
    let days = days.max(0.0) as u32;
    days
}

/// Restore all, empty: the one action here nothing puts back says what it would erase and is
/// confirmed elsewhere than on the button that asked. It stands aside while the trash is
/// searched: it would erase notes not shown.
fn footer(
    ui: &mut Ui,
    state: &AppState,
    count: usize,
    view: &mut TrashView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let top = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), top, Stroke::new(1.0, p.line_soft));
    ui.add_space(4.0);
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if view.confirming_empty {
            if Button::new(&tr(keys::trash::EMPTY_TRASH_CONFIRM), "trash-empty-confirm")
                .kind(Kind::Destructive)
                .armed(true)
                .show(ui)
                .clicked()
            {
                view.confirming_empty = false;
                actions.push(Action::Notes(Asked::Trash(Trash::Empty)));
            }
            if Button::new(&tr(keys::trash::EMPTY_TRASH_CANCEL), "trash-empty-cancel")
                .show(ui)
                .clicked()
            {
                view.confirming_empty = false;
            }
            // Two strings and not one with a count: French keeps the singular at one.
            let warning = if count == 1 {
                tr(keys::trash::EMPTY_TRASH_WARNING_ONE)
            } else {
                i18n::translate(
                    language,
                    keys::trash::EMPTY_TRASH_WARNING,
                    &[("count", count.into())],
                )
            };
            let room = ui.available_width();
            ui.allocate_ui(vec2(room, 24.0), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let response = ui.add(
                        egui::Label::new(egui::RichText::new(&warning).size(11.5).color(p.red))
                            .wrap(),
                    );
                    response.widget_info(|| WidgetInfo::labeled(Role::Alert, true, &warning));
                    widgets::address(ui, &response, "trash-empty-warning");
                });
            });
            return;
        }
        if !state.trash.searching()
            && Button::new(&tr(keys::trash::EMPTY_TRASH), "trash-empty")
                .kind(Kind::Destructive)
                .show(ui)
                .clicked()
        {
            view.confirming_empty = true;
        }
        let key = if state.trash.searching() {
            keys::trash::RESTORE_FOUND
        } else {
            keys::trash::RESTORE_ALL
        };
        let restore = i18n::translate(language, key, &[("count", count.into())]);
        if Button::new(&restore, "trash-restore-all")
            .show(ui)
            .clicked()
        {
            actions.push(Action::Notes(Asked::Trash(Trash::RestoreAll)));
        }
    });
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::days_left;
    use crate::notes::bench::{Bench, area, cards, click_id, settle, until};
    use crate::testing;

    /// `kept` on the canvas, `gone` in the trash, the panel open.
    fn trashed(gone: &[&str]) -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "kept");
        let ids: Vec<String> = gone
            .iter()
            .map(|title| testing::note(&backend, title).id.clone())
            .collect();
        engine::db::run_here(backend.engine(), move |engine| {
            engine::notes::delete_notes(engine, ids)
        })
        .unwrap();
        let mut harness = area(backend, profile);
        click_id(&mut harness, "trash-open");
        harness
    }

    fn rows(harness: &Harness<'_, Bench>) -> Vec<String> {
        harness
            .query_all_by(|node| node.author_id() == Some("trash-row-title"))
            .map(|node| {
                let node = node.accesskit_node();
                node.label().or_else(|| node.value()).unwrap_or_default()
            })
            .collect()
    }

    fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
        harness
            .query_by(|node| node.author_id() == Some(author_id))
            .is_some()
    }

    #[test]
    fn a_deleted_note_is_listed_and_put_back() {
        let mut harness = trashed(&["gone"]);
        assert_eq!(rows(&harness), ["gone"]);
        assert_eq!(cards(&harness), 1);

        click_id(&mut harness, "trash-restore");

        assert!(has(&harness, "trash-empty-state"));
        assert_eq!(cards(&harness), 2);
    }

    #[test]
    fn purging_takes_the_second_click() {
        let mut harness = trashed(&["gone"]);
        click_id(&mut harness, "trash-purge");
        assert_eq!(rows(&harness), ["gone"]);

        click_id(&mut harness, "trash-purge");

        assert!(has(&harness, "trash-empty-state"));
        assert_eq!(cards(&harness), 1);
    }

    #[test]
    fn a_search_narrows_the_list_and_restoring_reaches_what_it_found() {
        let mut harness = trashed(&["Rollout script", "Rollout notes", "Leftover"]);
        click_id(&mut harness, "trash-search");
        harness
            .get_by(|node| node.author_id() == Some("trash-search"))
            .type_text("rollout");
        until(&mut harness, "the search narrows the trash", |harness| {
            rows(harness).len() == 2
        });
        assert!(!has(&harness, "trash-empty"), "emptying stands aside");

        click_id(&mut harness, "trash-restore-all");
        until(&mut harness, "the found notes leave the trash", |harness| {
            has(harness, "trash-empty-state")
        });

        assert_eq!(cards(&harness), 3);
    }

    #[test]
    fn emptying_says_what_it_erases_then_erases_it() {
        let mut harness = trashed(&["one", "two"]);
        click_id(&mut harness, "trash-empty");
        assert!(has(&harness, "trash-empty-warning"));
        click_id(&mut harness, "trash-empty-cancel");
        assert_eq!(rows(&harness).len(), 2);

        click_id(&mut harness, "trash-empty");
        click_id(&mut harness, "trash-empty-confirm");

        assert!(has(&harness, "trash-empty-state"));
    }

    #[test]
    fn a_deleted_drawing_shows_its_words_not_its_scene() {
        let (backend, profile) = testing::unlocked();
        let seed = testing::note(&backend, "kept");
        let space = seed.space_id.clone();
        engine::db::run_here(backend.engine(), move |engine| {
            let draft = engine::notes::model::NoteDraft {
                kind: engine::notes::kind::NoteKind::Drawing,
                content: r#"{"version":1,"scene":{"version":"7.4.0","objects":[{"type":"Textbox","text":"Plan du réseau"}]}}"#.to_owned(),
                ..testing::snippet(space, "Archi".to_owned())
            };
            let id = engine::notes::create_note(engine, draft)?.id.clone();
            engine::notes::delete_notes(engine, vec![id])
        })
        .unwrap();
        let mut harness = area(backend, profile);
        click_id(&mut harness, "trash-open");

        let node = harness.get_by(|node| node.author_id() == Some("trash-row-snippet"));
        let node = node.accesskit_node();
        let shown = node.label().or_else(|| node.value()).unwrap_or_default();

        assert_eq!(shown, "Plan du réseau");
    }

    #[test]
    fn the_close_button_and_escape_close_it() {
        let mut harness = trashed(&[]);
        click_id(&mut harness, "trash-close");
        assert!(!harness.state().state.trash.is_open());

        click_id(&mut harness, "trash-open");
        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().state.trash.is_open());
    }

    #[test]
    fn a_note_just_deleted_is_erased_in_thirty_days_not_thirty_one() {
        let now = chrono::Utc::now();
        let purge = now + TimeDelta::days(30) - TimeDelta::milliseconds(5);

        assert_eq!(days_left(purge, now), 30);
        assert_eq!(days_left(now + TimeDelta::hours(5), now), 0);
        assert_eq!(days_left(now - TimeDelta::hours(5), now), 0);
    }
}
