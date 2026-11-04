//! What a card offers under the pointer: copy, and the ⋯ of its menu — open, pin, priority ›,
//! copy, duplicate, file into a folder of its space, move to another space, delete in two
//! steps. A pill over the card's bottom edge, clear of its footer and costing it no room.

use egui::{
    CornerRadius, Id, Popup, PopupCloseBehavior, Rect, RectAlign, Response, Stroke, Ui, Vec2, pos2,
};
use engine::folders::model::Lineage;
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;
use engine::notes::priority::Priority;
use i18n::keys;

use super::Context;
use crate::action::{Action, Actions};
use crate::notes::Action as Asked;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, IconButton, MIN_TARGET};

const LEVELS: [Priority; 5] = [
    Priority::None,
    Priority::Low,
    Priority::Medium,
    Priority::High,
    Priority::Urgent,
];

fn push(actions: &mut Actions, action: Asked) {
    actions.push(Action::Notes(action));
}

/// Shown while the pointer is on the card or the pill, or its menu is open.
pub(super) fn show(
    ui: &mut Ui,
    card: Rect,
    note: &DisplayNote,
    context: &Context<'_>,
    actions: &mut Actions,
) {
    let ctx = ui.ctx().clone();
    let here = Id::unique("note-card-actions").with(&note.id);
    let (drawn, open) = ctx.data(|data| data.get_temp::<(Rect, bool)>(here)).unzip();
    let width = 2.0 * MIN_TARGET + 6.0 + 10.0;
    // Over the bottom edge, ending short of the next row.
    let pill = Rect::from_min_size(
        pos2(card.right() - 12.0 - width, card.bottom() - MIN_TARGET),
        Vec2::new(width, MIN_TARGET + 8.0),
    );
    // Where it was drawn last: the pointer on its way to a button stays on the pill.
    let pointed = ui.rect_contains_pointer(card) || ui.rect_contains_pointer(drawn.unwrap_or(pill));
    if !(pointed || open.unwrap_or(false)) {
        return;
    }
    let p = palette(&ctx);
    let language = context.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let title = if note.title.is_empty() {
        tr(keys::notes::UNTITLED)
    } else {
        note.title.clone()
    };
    // ⚠️ In the card's own layer, drawn after its click surface: a pill in an area above it
    // lost its clicks to that surface.
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(pill).id_salt(here));
    let shown = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(4))
        .show(&mut ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                // A note with fields asks for them before it is copied.
                let (icon, key, author_id) = if note.placeholders.is_empty() {
                    (Icon::Copy, keys::notes::COPY_CONTENT, "note-card-copy")
                } else {
                    (Icon::Zap, keys::placeholders::TITLE, "note-card-fill")
                };
                if note.kind != NoteKind::Drawing
                    && IconButton::new(icon, &tr(key), author_id)
                        .show(ui)
                        .clicked()
                {
                    push(actions, Asked::Copy(note.id.clone()));
                }
                let label = i18n::translate(
                    language,
                    keys::notes::NOTE_OPTIONS_LABEL,
                    &[("title", title.as_str().into())],
                );
                let trigger = IconButton::new(Icon::Ellipsis, &label, "note-card-menu").show(ui);
                menu(ui, &trigger, note, context, actions);
                Popup::is_id_open(ui.ctx(), Popup::default_response_id(&trigger))
            })
            .inner
        });
    let open = shown.inner;
    let drawn = shown.response.rect;
    ctx.data_mut(|data| data.insert_temp(here, (drawn, open)));
}

fn menu(
    ui: &Ui,
    trigger: &Response,
    note: &DisplayNote,
    context: &Context<'_>,
    actions: &mut Actions,
) {
    let language = context.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let armed_id = trigger.id.with("armed");
    let armed = ui
        .data(|data| data.get_temp::<bool>(armed_id))
        .unwrap_or(false);
    let shown = Popup::menu(trigger)
        .frame(widgets::menu_frame(ui.ctx()))
        .align(RectAlign::BOTTOM_END)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(240.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            let id = note.id.clone();
            first_entries(ui, note, language, actions);
            filing(ui, note, context, actions);
            moving(ui, note, context, actions);
            let delete = tr(if armed {
                keys::editor::DELETE_CONFIRM
            } else {
                keys::editor::DELETE
            });
            let deleted = widgets::menu_entry(ui, &delete, "note-card-delete", true, armed, false);
            if deleted.clicked() {
                if armed {
                    push(actions, Asked::Delete(id));
                    ui.close();
                } else {
                    ui.data_mut(|data| data.insert_temp(armed_id, true));
                }
            }
        });
    // Folded with the menu: the next opening starts unarmed.
    if shown.is_none() && armed {
        ui.data_mut(|data| data.insert_temp(armed_id, false));
    }
}

/// Open, pin, priority, copy, duplicate: what is the note's own.
fn first_entries(ui: &mut Ui, note: &DisplayNote, language: i18n::Language, actions: &mut Actions) {
    let tr = |key| i18n::translate(language, key, &[]);
    let id = note.id.clone();
    if widgets::menu_entry(
        ui,
        &tr(keys::notes::OPEN_NOTE),
        "note-card-open-menu",
        false,
        false,
        false,
    )
    .clicked()
    {
        push(actions, Asked::OpenNote(id.clone()));
        ui.close();
    }
    let pin = tr(if note.pinned {
        keys::editor::PINNED
    } else {
        keys::editor::PIN
    });
    if widgets::menu_entry(ui, &pin, "note-card-pin-toggle", false, note.pinned, false).clicked() {
        push(
            actions,
            Asked::Pin {
                id: id.clone(),
                pinned: !note.pinned,
            },
        );
        ui.close();
    }
    priority(ui, note, language, actions);
    if note.kind != NoteKind::Drawing
        && widgets::menu_entry(
            ui,
            &tr(keys::notes::COPY),
            "note-card-copy-menu",
            false,
            false,
            false,
        )
        .clicked()
    {
        push(actions, Asked::Copy(id.clone()));
        ui.close();
    }
    if widgets::menu_entry(
        ui,
        &tr(keys::notes::DUPLICATE),
        "note-card-duplicate",
        false,
        false,
        false,
    )
    .clicked()
    {
        let copy = if note.title.is_empty() {
            String::new()
        } else {
            i18n::translate(
                language,
                keys::notes::COPY_TITLE,
                &[("title", note.title.as_str().into())],
            )
        };
        push(
            actions,
            Asked::Duplicate {
                id: id.clone(),
                title: copy,
            },
        );
        ui.close();
    }
}

/// « Priority › », its five levels to the side, the note's ticked.
fn priority(ui: &mut Ui, note: &DisplayNote, language: i18n::Language, actions: &mut Actions) {
    let label = i18n::translate(language, keys::notes::PRIORITY_LABEL, &[]);
    let parent = widgets::menu_entry(ui, &label, "note-card-priority", false, false, true);
    let open_id = parent.id.with("levels");
    let mut open = parent.clicked()
        || parent.hovered()
        || ui.data(|data| data.get_temp(open_id)).unwrap_or(false);
    Popup::from_response(&parent)
        .id(open_id)
        .align(RectAlign::RIGHT_START)
        .frame(widgets::menu_frame(ui.ctx()))
        .open_bool(&mut open)
        .show(|ui| {
            ui.set_width(180.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            for level in LEVELS {
                let name = i18n::translate(language, level_key(level), &[]);
                let on = level == note.priority;
                if widgets::menu_entry(ui, &name, "note-card-priority-level", false, on, false)
                    .clicked()
                {
                    if !on {
                        push(
                            actions,
                            Asked::Prioritise {
                                id: note.id.clone(),
                                priority: level,
                            },
                        );
                    }
                    ui.close();
                }
            }
        });
    ui.data_mut(|data| data.insert_temp(open_id, open));
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

/// The folders of the note's space, in the tree's order, a nested one hinted by its parents;
/// where it is, left out. A folder belongs to one space, so moving out of it is below.
fn filing(ui: &mut Ui, note: &DisplayNote, context: &Context<'_>, actions: &mut Actions) {
    let language = context.language;
    let inside: Vec<_> = context
        .library
        .folders()
        .iter()
        .filter(|folder| folder.space_id == note.space_id)
        .collect();
    let current = note.folder_id.as_deref();
    if inside.is_empty() && current.is_none() {
        return;
    }
    title(ui, &i18n::translate(language, keys::notes::FILE_INTO, &[]));
    if current.is_some()
        && widgets::menu_entry(
            ui,
            &i18n::translate(language, keys::notes::NO_FOLDER, &[]),
            "note-card-unfile",
            false,
            false,
            false,
        )
        .clicked()
    {
        push(
            actions,
            Asked::File {
                id: note.id.clone(),
                folder: None,
            },
        );
        ui.close();
    }
    let lineage = Lineage::new(
        inside
            .iter()
            .map(|folder| (folder.id.clone(), folder.parent_id.clone())),
    );
    for folder in inside
        .iter()
        .filter(|folder| Some(folder.id.as_str()) != current)
    {
        let path: Vec<&str> = lineage
            .path(&folder.id)
            .iter()
            .filter_map(|id| inside.iter().find(|other| other.id == *id))
            .map(|other| other.name.as_str())
            .collect();
        let label = path.join(" › ");
        if widgets::menu_entry(ui, &label, "note-card-file", false, false, false).clicked() {
            push(
                actions,
                Asked::File {
                    id: note.id.clone(),
                    folder: Some(folder.id.clone()),
                },
            );
            ui.close();
        }
    }
}

/// The other spaces: moving there takes the note out of its folder.
fn moving(ui: &mut Ui, note: &DisplayNote, context: &Context<'_>, actions: &mut Actions) {
    let others: Vec<_> = context
        .library
        .spaces()
        .iter()
        .filter(|space| space.id != note.space_id)
        .collect();
    if others.is_empty() {
        return;
    }
    title(
        ui,
        &i18n::translate(context.language, keys::notes::MOVE_TO_SPACE, &[]),
    );
    for space in others {
        if widgets::menu_entry(ui, &space.name, "note-card-move", false, false, false).clicked() {
            push(
                actions,
                Asked::MoveTo {
                    id: note.id.clone(),
                    space: space.id.clone(),
                },
            );
            ui.close();
        }
    }
}

fn title(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let upper = words.to_uppercase();
    let galley = widgets::text(
        ui,
        &upper,
        10.5,
        600.0,
        egui::FontFamily::Proportional,
        p.text_2,
    );
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x, 24.0), egui::Sense::hover());
    ui.painter().galley(
        pos2(rect.left() + 8.0, rect.bottom() - galley.size().y - 4.0),
        galley,
        p.text_2,
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::model::DisplayNote;
    use engine::notes::priority::Priority;

    use crate::notes::bench::{Bench, area, cards, click, click_id, folder, settle, space};
    use crate::testing;

    fn stored(harness: &Harness<'_, Bench>, id: &str) -> DisplayNote {
        let id = id.to_owned();
        engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::notes::get_note(engine, id)
        })
        .unwrap()
    }

    /// The pill shows under the pointer: the menu opens from it.
    fn open_menu(harness: &mut Harness<'_, Bench>) {
        harness
            .get_by(|node| node.author_id() == Some("note-card-open"))
            .hover();
        settle(harness);
        click_id(harness, "note-card-menu");
    }

    #[test]
    fn the_menu_pins_and_sets_a_priority() {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "psql").id.clone();
        let mut harness = area(backend, profile);

        open_menu(&mut harness);
        click_id(&mut harness, "note-card-pin-toggle");
        assert!(stored(&harness, &id).pinned);

        open_menu(&mut harness);
        click_id(&mut harness, "note-card-priority");
        click(&mut harness, "note-card-priority-level", "Urgent");
        assert_eq!(stored(&harness, &id).priority, Priority::Urgent);
    }

    #[test]
    fn deleting_from_the_menu_takes_two_clicks() {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "psql");
        let mut harness = area(backend, profile);

        open_menu(&mut harness);
        click_id(&mut harness, "note-card-delete");
        assert_eq!(cards(&harness), 1);
        let armed = harness
            .get_by(|node| node.author_id() == Some("note-card-delete"))
            .accesskit_node()
            .label()
            .unwrap_or_default();
        assert_eq!(armed, "Confirm?");
        click_id(&mut harness, "note-card-delete");

        assert_eq!(cards(&harness), 0);
    }

    #[test]
    fn a_note_is_filed_then_moved_to_another_space() {
        let (backend, profile) = testing::unlocked();
        let id = testing::note(&backend, "psql").id.clone();
        let spaces = engine::db::run_here(backend.engine(), engine::spaces::list_spaces).unwrap();
        let runbooks = folder(&backend, &spaces[0], None, "Runbooks");
        let work = space(&backend, "Work");
        let mut harness = area(backend, profile);

        open_menu(&mut harness);
        click(&mut harness, "note-card-file", "Runbooks");
        assert_eq!(stored(&harness, &id).folder_id, Some(runbooks.id));

        open_menu(&mut harness);
        click(&mut harness, "note-card-move", "Work");
        let moved = stored(&harness, &id);
        assert_eq!(moved.space_id, work.id);
        assert_eq!(moved.folder_id, None);
    }

    #[test]
    fn a_copy_opens_in_the_editor_under_its_own_title() {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "psql");
        let mut harness = area(backend, profile);

        open_menu(&mut harness);
        click_id(&mut harness, "note-card-duplicate");

        assert_eq!(cards(&harness), 2);
        let opened = harness.state().state.editor.opened().unwrap();
        assert_eq!(opened.note().unwrap().title, "psql (copy)");
    }

    #[test]
    fn copying_reads_the_note_whole_and_says_so() {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "psql");
        let mut harness = area(backend, profile);

        harness
            .get_by(|node| node.author_id() == Some("note-card-open"))
            .hover();
        settle(&mut harness);
        click_id(&mut harness, "note-card-copy");

        let (_, said) = harness.state().state.status.latest().unwrap();
        assert_eq!(said.key, i18n::keys::notes::COPIED_NOTE);
    }
}
