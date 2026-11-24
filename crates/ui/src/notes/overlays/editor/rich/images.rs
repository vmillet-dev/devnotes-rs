//! An image of a Note picked, in the caret's place: a drawing is resized by its corners, by the
//! sizes over it or by `Alt` and an arrow; the keys of the text go past it or take it away.

use crate::theme::tokens::gap;
use egui::{CornerRadius, CursorIcon, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use i18n::{Language, keys};
use richtext::picture::{self, Beside};
use richtext::{Position, Selection, tree};

use super::{RichEditor, Step};
use crate::theme::palette;
use crate::widgets::rich::LaidImage;
use crate::widgets::{self, Segmented};

const HANDLE: f32 = 8.0;

impl RichEditor {
    /// The caret goes to the text past the image: what is typed goes there.
    pub(super) fn pick(&mut self, image: Vec<usize>) {
        let past = |forward| match picture::beside(&self.document, &image, forward) {
            Some(Beside::Leaf(position)) => Some(position),
            _ => None,
        };
        let place = past(true).or_else(|| past(false)).unwrap_or_else(|| {
            let start = Position {
                path: image.clone(),
                offset: 0,
            };
            tree::clamp(&self.document, &start)
        });
        self.selection = Selection::caret(place);
        self.goal_x = None;
        self.picked = Some(image);
    }

    fn set_width(&mut self, image: Vec<usize>, width: Option<u8>) {
        if picture::width(&self.document, &image) == width {
            return;
        }
        self.record(Step::Sizing);
        picture::set_width(&mut self.document, &image, width);
        self.reveal = false;
        self.picked = Some(image);
    }

    /// The width the image is drawn at, in percent of the text's.
    fn shown_width(&self, image: &[usize]) -> Option<u8> {
        let laid = self.images.iter().find(|laid| laid.path == image)?;
        let whole = laid.whole?;
        picture::width(&self.document, image)
            .or_else(|| Some(picture::width_near(laid.rect.width() / whole * 100.0)))
    }
}

/// `None` when the key is the text's; `true` when it was a shortcut.
pub(super) fn key_pressed(view: &mut RichEditor, key: Key, modifiers: Modifiers) -> Option<bool> {
    let image = view.picked.clone()?;
    let sideways = matches!(key, Key::ArrowLeft | Key::ArrowRight);
    if sideways && modifiers == Modifiers::ALT {
        if let Some(width) = view.shown_width(&image) {
            let beside = picture::width_beside(width, key == Key::ArrowRight);
            view.set_width(image, Some(beside));
        }
        return Some(true);
    }
    match key {
        Key::Backspace | Key::Delete => {
            view.record(Step::Other);
            view.selection = picture::remove(&mut view.document, &image);
        }
        Key::ArrowLeft | Key::ArrowUp => view.step_past(image, false, false),
        Key::ArrowRight | Key::ArrowDown => view.step_past(image, true, false),
        _ => return None,
    }
    Some(false)
}

/// Over the image picked: its outline, and on a drawing the corners to drag and the sizes.
pub(super) fn controls(
    ui: &mut Ui,
    view: &mut RichEditor,
    images: &[LaidImage],
    language: Language,
) {
    let Some(laid) = images
        .iter()
        .find(|laid| Some(&laid.path) == view.picked.as_ref())
    else {
        return;
    };
    let p = palette(ui.ctx());
    ui.painter().rect_stroke(
        laid.rect,
        CornerRadius::ZERO,
        Stroke::new(1.5, p.amber_edge),
        StrokeKind::Outside,
    );
    let Some(whole) = laid.whole else { return };
    sizes(ui, view, laid, language);
    let label = i18n::translate(language, keys::drawing::EMBED_RESIZE, &[]);
    let corners = [
        (
            "rich-embed-resize",
            laid.rect.right_bottom(),
            CursorIcon::ResizeNwSe,
        ),
        (
            "rich-embed-resize-top",
            laid.rect.right_top(),
            CursorIcon::ResizeNeSw,
        ),
    ];
    for (author_id, at, cursor) in corners {
        let room = Rect::from_center_size(at, Vec2::splat(widgets::MIN_TARGET));
        let corner_ui = egui::UiBuilder::new().id_salt(author_id).max_rect(room);
        let response = widgets::interact(
            &mut ui.new_child(corner_ui),
            room.size(),
            Sense::drag(),
            author_id,
            || egui::WidgetInfo::labeled(egui::Role::Button, true, &label),
            CornerRadius::ZERO,
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        ui.painter().rect(
            Rect::from_center_size(at, Vec2::splat(HANDLE)),
            CornerRadius::ZERO,
            p.amber_edge,
            Stroke::new(1.0, p.bg_1),
            StrokeKind::Inside,
        );
        if response.drag_started() {
            // A drag is one step of the history, whatever was resized before.
            view.last = None;
            view.refocus = true;
        }
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let percent = (pointer.x - laid.rect.left()) / whole * 100.0;
            view.set_width(laid.path.clone(), Some(picture::width_near(percent)));
        }
    }
}

/// Its own size, and the quarters of the text's width.
fn sizes(ui: &mut Ui, view: &mut RichEditor, laid: &LaidImage, language: Language) {
    let own = i18n::translate(language, keys::drawing::EMBED_OWN_SIZE, &[]);
    let stops = picture::STOPS.map(|stop| (Some(stop), format!("{stop} %")));
    let mut options: Vec<(Option<u8>, &str)> = vec![(None, own.as_str())];
    options.extend(stops.iter().map(|(stop, name)| (*stop, name.as_str())));
    let sizes = Segmented::new(&options, "rich-embed-size");
    let size = Vec2::new(sizes.width(ui), widgets::MIN_TARGET + 2.0);
    // Under the drawing, and never out of sight.
    let seen = ui.clip_rect().shrink(gap::CONTROLS);
    let under = laid.rect.bottom() + gap::CONTROLS;
    let top = under.min(seen.bottom() - size.y).max(seen.top());
    let at = egui::pos2(laid.rect.left(), top);
    let mut room = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("rich-embed-size")
            .max_rect(Rect::from_min_size(at, size)),
    );
    let mut width = picture::width(&view.document, &laid.path);
    if sizes.show(&mut room, &mut width).changed() {
        view.last = None;
        view.set_width(laid.path.clone(), width);
        view.refocus = true;
    }
}
