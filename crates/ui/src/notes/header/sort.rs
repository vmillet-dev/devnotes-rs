//! « Sort and group »: the button names the order the canvas is in, the menu changes it. It
//! stays open on a choice — order, direction and grouping are set together — and a new sort key
//! starts the way it reads first.

use egui::{
    CornerRadius, FontFamily, Popup, PopupCloseBehavior, Rect, Response, Role, Sense, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, pos2,
};
use engine::notes::view::{Grouping, NoteOrder, SortDirection, SortKey};
use i18n::{Key, keys};

use crate::action::{Action, Actions};
use crate::state::AppState;
use crate::state::notes_query::{Arrangement, DIRECTIONS, GROUPINGS, SORT_KEYS, natural_order};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, interact};

/// Past this window width the button says the order in words, as the reference's media query.
const WORDS_FROM: f32 = 1_280.0;

const MENU_WIDTH: f32 = 214.0;

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let now = state.notes_query.arrangement();
    let arrow = match now.order.direction {
        SortDirection::Descending => "↓",
        SortDirection::Ascending => "↑",
    };
    let short = tr(short_key(now.order.key));
    let label = format!("{} · {short}", tr(keys::sort::LABEL));
    let words = ui.ctx().content_rect().width() >= WORDS_FROM;
    let trigger = trigger(ui, &label, &format!("{short} {arrow}"), words);
    let trigger = widgets::tooltip(trigger, &format!("{label} {arrow}"));
    let chosen = Popup::menu(&trigger)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| menu(ui, now, language))
        .and_then(|shown| shown.inner);
    if let Some(arrangement) = chosen {
        actions.push(Action::Notes(crate::notes::Action::Arrange(arrangement)));
    }
}

/// 32 × 32, a glyph; the order in words once the window is wide enough for it.
fn trigger(ui: &mut Ui, label: &str, now: &str, words: bool) -> Response {
    let p = palette(ui.ctx());
    let width = if words {
        widgets::arrowed(ui, None, now, 12.0, p.text_1) + 14.0
    } else {
        32.0
    };
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        Vec2::new(width, 32.0),
        Sense::click(),
        "sort-trigger",
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
    let lit = open || response.hovered();
    let (edge, ink) = if lit {
        (p.amber_dim, p.amber_text)
    } else {
        (p.line_soft, p.text_1)
    };
    ui.painter().rect(
        response.rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    if words {
        let rect = response.rect;
        widgets::arrowed(
            ui,
            Some(pos2(rect.left() + 7.0, rect.center().y)),
            now,
            12.0,
            ink,
        );
    } else {
        let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
        icons::paint(ui, glyph, Icon::ArrowUpDown, ink);
    }
    response
}

/// The arrangement chosen, if one was.
fn menu(ui: &mut Ui, now: Arrangement, language: i18n::Language) -> Option<Arrangement> {
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(MENU_WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut chosen = None;
    title(ui, &tr(keys::sort::SORT_BY));
    for key in SORT_KEYS {
        let hint = hint_key(key).map(tr);
        if item(
            ui,
            &tr(long_key(key)),
            hint.as_deref(),
            "sort-key",
            key == now.order.key,
        ) && key != now.order.key
        {
            chosen = Some(Arrangement {
                order: natural_order(key),
                ..now
            });
        }
    }
    title(ui, &tr(keys::sort::DIRECTION));
    for direction in DIRECTIONS {
        let on = direction == now.order.direction;
        if item(
            ui,
            &tr(direction_key(direction)),
            None,
            "sort-direction",
            on,
        ) && !on
        {
            chosen = Some(Arrangement {
                order: NoteOrder {
                    key: now.order.key,
                    direction,
                },
                ..now
            });
        }
    }
    title(ui, &tr(keys::sort::GROUP_BY));
    for grouping in GROUPINGS {
        let on = grouping == now.grouping;
        if item(ui, &tr(grouping_key(grouping)), None, "sort-grouping", on) && !on {
            chosen = Some(Arrangement { grouping, ..now });
        }
    }
    ui.add_space(4.0);
    let pinned = tr(keys::sort::PINNED_FIRST);
    if item(ui, &pinned, None, "sort-pinned-first", now.pinned_first) {
        chosen = Some(Arrangement {
            pinned_first: !now.pinned_first,
            ..now
        });
    }
    chosen
}

fn title(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let upper = words.to_uppercase();
    let galley = widgets::text(ui, &upper, 10.5, 600.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x, 22.0), Sense::hover());
    ui.painter().galley(
        pos2(rect.left() + 8.0, rect.bottom() - galley.size().y - 4.0),
        galley,
        p.text_2,
    );
}

/// A choice of the menu, ticked when it is the one in force. `true` when clicked.
fn item(ui: &mut Ui, label: &str, hint: Option<&str>, author_id: &str, on: bool) -> bool {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let role = if author_id == "sort-pinned-first" {
        Role::MenuItemCheckBox
    } else {
        Role::MenuItemRadio
    };
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), 26.0),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(role, true, on, label),
        corner,
    );
    let rect = response.rect;
    if response.hovered() || on {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    let ink = if on { p.text_0 } else { p.text_1 };
    if on {
        let glyph =
            Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), Vec2::splat(12.0));
        icons::paint(ui, glyph, Icon::Check, p.amber_text);
    }
    widgets::arrowed(
        ui,
        Some(pos2(rect.left() + 28.0, rect.center().y)),
        label,
        12.5,
        ink,
    );
    if let Some(hint) = hint {
        let width = widgets::arrowed(ui, None, hint, 11.0, p.text_2);
        let at = pos2(rect.right() - 8.0 - width, rect.center().y);
        widgets::arrowed(ui, Some(at), hint, 11.0, p.text_2);
    }
    response.clicked()
}

fn short_key(key: SortKey) -> Key {
    match key {
        SortKey::Modified => keys::sort::short::MODIFIED,
        SortKey::Created => keys::sort::short::CREATED,
        SortKey::Priority => keys::sort::short::PRIORITY,
        SortKey::Format => keys::sort::short::FORMAT,
        SortKey::Title => keys::sort::short::TITLE,
    }
}

fn long_key(key: SortKey) -> Key {
    match key {
        SortKey::Modified => keys::sort::keys::MODIFIED,
        SortKey::Created => keys::sort::keys::CREATED,
        SortKey::Priority => keys::sort::keys::PRIORITY,
        SortKey::Format => keys::sort::keys::FORMAT,
        SortKey::Title => keys::sort::keys::TITLE,
    }
}

fn hint_key(key: SortKey) -> Option<Key> {
    match key {
        SortKey::Modified => Some(keys::sort::hints::MODIFIED),
        SortKey::Format => Some(keys::sort::hints::FORMAT),
        SortKey::Title => Some(keys::sort::hints::TITLE),
        SortKey::Created | SortKey::Priority => None,
    }
}

fn direction_key(direction: SortDirection) -> Key {
    match direction {
        SortDirection::Descending => keys::sort::directions::DESCENDING,
        SortDirection::Ascending => keys::sort::directions::ASCENDING,
    }
}

fn grouping_key(grouping: Grouping) -> Key {
    match grouping {
        Grouping::Date => keys::sort::groupings::DATE,
        Grouping::Priority => keys::sort::groupings::PRIORITY,
        Grouping::Format => keys::sort::groupings::FORMAT,
        Grouping::None => keys::sort::groupings::NONE,
    }
}
