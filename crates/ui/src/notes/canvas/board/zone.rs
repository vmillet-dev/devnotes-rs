//! A folder drawn as a titled, coloured region. Its notes flow inside it, in the columns
//! `folders::board::columns_in` counts — the same Rust that sizes the zone lays its cards out,
//! so the two cannot disagree — and its body scrolls when they outgrow it.

use egui::{
    CornerRadius, FontFamily, Popup, PopupCloseBehavior, Rect, Role, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};
use engine::folders::board::{
    BoardChild, BoardZone, CARD_HEIGHT, CARD_WIDTH, GAP, ZONE_BORDER, ZONE_HEADER, ZONE_PADDING,
    ZONE_SUBFOLDERS, columns_in,
};
use i18n::keys;

use super::{BoardScreen, px};
use crate::action::{Action, Actions};
use crate::notes::canvas::{CanvasView, card};
use crate::notes::panel::{self, FolderEdit};
use crate::notes::ui::folder_hue;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, address, interact};

pub(super) fn show(
    ui: &mut Ui,
    rect: Rect,
    zone: &BoardZone,
    context: &card::Context<'_>,
    screen: &mut BoardScreen,
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let folder = &zone.folder;
    let hue = folder_hue(ui, folder.colour);
    let mut zone_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("board-zone", &folder.id)),
    );
    zone_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    let ui = &mut zone_ui;
    let corner = CornerRadius::same(10);
    ui.painter().rect(
        rect,
        corner,
        hue.gamma_multiply(0.08),
        Stroke::new(px(ZONE_BORDER), hue),
        StrokeKind::Inside,
    );
    let node = ui.allocate_rect(rect, Sense::hover());
    node.widget_info(|| WidgetInfo::labeled(Role::Region, true, &folder.name));
    address(ui, &node, "board-zone");
    let inside = rect.shrink(px(ZONE_BORDER));
    let head = Rect::from_min_size(inside.min, vec2(inside.width(), px(ZONE_HEADER)));
    header(ui, head, zone, context.language, screen, actions);
    let mut top = head.bottom();
    if !zone.children.is_empty() {
        let row = Rect::from_min_size(
            pos2(inside.left(), top),
            vec2(inside.width(), px(ZONE_SUBFOLDERS)),
        );
        children(ui, row, zone, context.language, actions);
        top = row.bottom();
    }
    let body = Rect::from_min_max(pos2(inside.left(), top), inside.max);
    flow(ui, body, zone, context, (canvas_view, actions));
}

/// The swatch, the title that opens the folder, the count, and the ⋯ that edits it.
fn header(
    ui: &mut Ui,
    head: Rect,
    zone: &BoardZone,
    language: i18n::Language,
    screen: &mut BoardScreen,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let folder = &zone.folder;
    let hue = folder_hue(ui, folder.colour);
    let middle = head.center().y;
    let mut left = head.left() + 12.0;
    let swatch = Rect::from_min_size(pos2(left, middle - 4.5), Vec2::splat(9.0));
    ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
    left = swatch.right() + 9.0;

    let menu_room = Rect::from_center_size(
        pos2(head.right() - 12.0 - widgets::MIN_TARGET / 2.0, middle),
        Vec2::splat(widgets::MIN_TARGET),
    );
    let count = zone.notes.len().to_string();
    let count_galley = widgets::text(ui, &count, 11.5, 400.0, FontFamily::Proportional, p.text_2);
    let name_room = (menu_room.left() - 9.0 - count_galley.size().x - 9.0 - left).max(0.0);
    let name = widgets::text(
        ui,
        &folder.name,
        13.0,
        500.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let name_size = vec2(name.size().x.min(name_room), name.size().y);
    let target = Rect::from_min_size(
        pos2(left, middle - widgets::MIN_TARGET / 2.0),
        vec2(name_size.x, widgets::MIN_TARGET),
    );
    let mut name_ui = ui.new_child(UiBuilder::new().max_rect(target));
    let open = interact(
        &mut name_ui,
        target.size(),
        Sense::click(),
        "board-zone-open",
        || WidgetInfo::labeled(Role::Button, true, &folder.name),
        CornerRadius::same(3),
    );
    let at = pos2(left, middle - name.size().y / 2.0);
    let clip = Rect::from_min_size(at, name_size);
    ui.painter()
        .with_clip_rect(clip.intersect(ui.clip_rect()))
        .galley(at, name, p.text_0);
    if open.hovered() {
        let under = clip.bottom() + 1.0;
        ui.painter()
            .hline(clip.x_range(), under, Stroke::new(1.0, hue));
    }
    if open.clicked() {
        actions.push(Action::Notes(crate::notes::Action::ChooseFolder(
            folder.id.clone(),
        )));
    }
    let count_at = pos2(target.right() + 9.0, middle - count_galley.size().y / 2.0);
    let count_rect = Rect::from_min_size(count_at, count_galley.size());
    let shown = ui.allocate_rect(count_rect, Sense::hover());
    shown.widget_info(|| WidgetInfo::labeled(Role::Label, true, &count));
    address(ui, &shown, "board-zone-count");
    ui.painter().galley(count_at, count_galley, p.text_2);
    menu(ui, menu_room, zone, language, screen, actions);
}

/// The same panel the breadcrumb and the rail open: rename, recolour, a folder inside, delete.
fn menu(
    ui: &mut Ui,
    room: Rect,
    zone: &BoardZone,
    language: i18n::Language,
    screen: &mut BoardScreen,
    actions: &mut Actions,
) {
    let folder = &zone.folder;
    let label = i18n::translate(
        language,
        keys::folders::OPTIONS_LABEL,
        &[("name", folder.name.as_str().into())],
    );
    let mut menu_ui = ui.new_child(UiBuilder::new().max_rect(room));
    let button =
        widgets::IconButton::new(Icon::Ellipsis, &label, "board-zone-menu").show(&mut menu_ui);
    let shown = Popup::menu(&button)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(260.0);
            let edit = match &mut screen.menu {
                Some(edit) if edit.id == folder.id => edit,
                other => other.insert(FolderEdit::open(folder)),
            };
            if panel::folder(ui, edit, folder, language, actions) {
                ui.close();
            }
        });
    let open = screen
        .menu
        .as_ref()
        .is_some_and(|edit| edit.id == folder.id);
    if shown.is_none() && open {
        screen.menu = None;
    }
}

/// One line of the zone's sub-folders, never wrapped: its height is the one Rust counts in the
/// zone's. It scrolls sideways when they do not hold.
fn children(
    ui: &mut Ui,
    row: Rect,
    zone: &BoardZone,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let label = i18n::translate(
        language,
        keys::board::SUB_FOLDERS,
        &[("name", zone.folder.name.as_str().into())],
    );
    let line = Rect::from_min_max(row.min + vec2(12.0, 0.0), row.max - vec2(12.0, 6.0));
    let node = ui.allocate_rect(line, Sense::hover());
    node.widget_info(|| WidgetInfo::labeled(Role::Navigation, true, &label));
    address(ui, &node, "board-zone-children");
    let caption = i18n::translate(language, keys::board::SUB_FOLDERS_LABEL, &[]);
    let galley = widgets::text(
        ui,
        &caption,
        11.0,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    );
    let at = pos2(line.left(), line.center().y - galley.size().y / 2.0);
    let width = galley.size().x;
    ui.painter().galley(at, galley, p.text_2);
    let list = Rect::from_min_max(pos2(line.left() + width + 6.0, line.top()), line.max);
    let mut list_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(list)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    egui::ScrollArea::horizontal()
        .id_salt(("board-zone-children", &zone.folder.id))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut list_ui, |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.horizontal_centered(|ui| {
                for child in &zone.children {
                    if chip(ui, child).clicked() {
                        actions.push(Action::Notes(crate::notes::Action::ChooseFolder(
                            child.id.clone(),
                        )));
                    }
                }
            });
        });
}

/// A sub-folder: its swatch, its name, what its whole sub-tree holds.
fn chip(ui: &mut Ui, child: &BoardChild) -> egui::Response {
    let p = palette(ui.ctx());
    let hue = folder_hue(ui, child.colour);
    let name = widgets::text(
        ui,
        &child.name,
        11.5,
        400.0,
        FontFamily::Proportional,
        p.text_1,
    );
    let count = child.count.to_string();
    let counted = widgets::text(ui, &count, 11.5, 400.0, FontFamily::Proportional, p.text_2);
    let width = 8.0 + 7.0 + 6.0 + name.size().x + 6.0 + counted.size().x + 9.0;
    let corner = CornerRadius::same(12);
    let response = interact(
        ui,
        vec2(width, widgets::MIN_TARGET),
        Sense::click(),
        "board-zone-child",
        || WidgetInfo::labeled(Role::Button, true, &child.name),
        corner,
    );
    let rect = response.rect;
    let edge = if response.hovered() { hue } else { p.line };
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let middle = rect.center().y;
    let swatch = Rect::from_min_size(pos2(rect.left() + 8.0, middle - 3.5), Vec2::splat(7.0));
    ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
    let ink = if response.hovered() {
        p.text_0
    } else {
        p.text_1
    };
    let name_at = pos2(swatch.right() + 6.0, middle - name.size().y / 2.0);
    let name_width = name.size().x;
    ui.painter().galley(name_at, name, ink);
    let count_at = pos2(
        name_at.x + name_width + 6.0,
        middle - counted.size().y / 2.0,
    );
    ui.painter().galley(count_at, counted, p.text_2);
    response
}

/// The cards, `columns_in` across, a row at a time, only the rows the ground shows.
fn flow(
    ui: &mut Ui,
    body: Rect,
    zone: &BoardZone,
    context: &card::Context<'_>,
    (canvas_view, actions): (&mut CanvasView, &mut Actions),
) {
    let padding = px(ZONE_PADDING);
    if zone.notes.is_empty() {
        let p = palette(ui.ctx());
        let words = i18n::translate(context.language, keys::board::EMPTY_ZONE, &[]);
        let galley = widgets::text(ui, &words, 11.5, 400.0, FontFamily::Proportional, p.text_2);
        let rect = Rect::from_min_size(body.min + Vec2::splat(padding), galley.size());
        let response = ui.allocate_rect(rect, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
        address(ui, &response, "board-zone-empty");
        ui.painter().galley(rect.min, galley, p.text_2);
        return;
    }
    let columns = usize::try_from(columns_in(zone.frame.width))
        .unwrap_or(1)
        .max(1);
    let rows = zone.notes.len().div_ceil(columns);
    let (width, height, gap) = (px(CARD_WIDTH), px(CARD_HEIGHT), px(GAP));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a zone holds far under 2²⁴ rows"
    )]
    let tall = 2.0 * padding + rows as f32 * (height + gap) - gap;
    let mut body_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(body)
            .id_salt(("board-zone-body", &zone.folder.id)),
    );
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt(("board-zone-body", &zone.folder.id))
        .auto_shrink([false, false])
        .show(&mut body_ui, |ui| {
            let origin = ui.min_rect().min + Vec2::splat(padding);
            ui.set_min_size(vec2(body.width(), tall));
            #[expect(clippy::cast_precision_loss, reason = "a handful of columns")]
            let seat = |at: usize| {
                let (column, row) = ((at % columns) as f32, (at / columns) as f32);
                Rect::from_min_size(
                    origin + vec2(column * (width + gap), row * (height + gap)),
                    vec2(width, height),
                )
            };
            let requested = canvas_view.request.as_deref();
            if let Some(at) = zone
                .notes
                .iter()
                .position(|entry| Some(entry.note.id.as_str()) == requested)
            {
                ui.scroll_to_rect(seat(at), None);
            }
            let clip = ui.clip_rect();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "rows, clamped at zero"
            )]
            let row_at = |y: f32| ((y - origin.y) / (height + gap)).max(0.0) as usize;
            let first = row_at(clip.top()) * columns;
            let last = ((row_at(clip.bottom()) + 1) * columns).min(zone.notes.len());
            for (at, entry) in zone.notes.iter().enumerate().take(last).skip(first) {
                let rect = seat(at);
                let mut card_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect)
                        .id_salt(("board-card", &entry.note.id)),
                );
                card::show(
                    &mut card_ui,
                    rect,
                    &entry.note,
                    context,
                    (canvas_view, actions),
                );
            }
        });
}
