//! A row of a results card: the name, the value, and a copy icon at the end. A long value takes
//! the width of the card under its name, held to a height it scrolls in.

use std::sync::Arc;

use egui::{
    FontFamily, FontId, Galley, Pos2, Rect, Response, RichText, Role, Sense, Ui, UiBuilder, Vec2,
    WidgetInfo,
};
use i18n::Language;

use super::CopyValue;
use crate::theme::palette;
use crate::widgets::{self, address};

/// Past this many characters a value no longer fits beside its name: it goes under it. Above a
/// SHA-512 in hexadecimal, which still reads in a column with the digests around it.
const LONG_RESULT: usize = 200;
/// The rows of a card go two abreast once it holds two of this.
const ROW_MIN: f32 = 440.0;
const LINE: f32 = 12.5 * 1.55;

#[derive(Clone, Copy)]
pub struct ResultRow<'a> {
    name: &'a str,
    value: &'a str,
    copied: Option<&'a str>,
    copyable: bool,
    meta: &'a str,
    matched: bool,
    tag: Option<&'a str>,
}

impl<'a> ResultRow<'a> {
    pub fn new(name: &'a str, value: &'a str) -> Self {
        Self {
            name,
            value,
            copied: None,
            copyable: true,
            meta: "",
            matched: false,
            tag: None,
        }
    }

    /// What the icon copies when it is not what is shown: the exact value behind a rounded one.
    #[must_use]
    pub fn copied(mut self, copied: &'a str) -> Self {
        self.copied = Some(copied);
        self
    }

    /// A reading, not a value: nothing to paste anywhere.
    #[must_use]
    pub fn reading(mut self) -> Self {
        self.copyable = false;
        self
    }

    /// « 256 bits »: what the value is, under its name.
    #[must_use]
    pub fn meta(mut self, meta: &'a str) -> Self {
        self.meta = meta;
        self
    }

    /// A state beside the name: « obsolete ».
    #[must_use]
    pub fn tag(mut self, tag: Option<&'a str>) -> Self {
        self.tag = tag;
        self
    }

    /// Tinted, when this is the one that answers what was asked: a signature it matches.
    #[must_use]
    pub fn matched(mut self, matched: bool) -> Self {
        self.matched = matched;
        self
    }

    fn is_long(&self) -> bool {
        self.value.chars().count() > LONG_RESULT
    }

    pub fn show(self, ui: &mut Ui, language: Language) -> Response {
        let p = palette(ui.ctx());
        let width = ui.available_width();
        let inner = width - 16.0 - 10.0;
        let copy = if self.copyable { 28.0 + 12.0 } else { 0.0 };
        let long = self.is_long();
        let name_room = if long {
            inner - copy
        } else {
            (inner * 0.32).max(96.0)
        };
        let ink = if self.matched { p.green } else { p.text_0 };
        let name = widgets::wrapped(ui, self.name, (12.5, 500.0), ink, name_room);
        let meta = (!self.meta.is_empty())
            .then(|| widgets::wrapped(ui, self.meta, (11.0, 400.0), p.text_2, name_room));
        let label_height = name.size().y + meta.as_ref().map_or(0.0, |it| it.size().y + 2.0);
        let value_room = if long {
            inner
        } else {
            inner - name_room - 12.0 - copy
        };
        let galley = value_galley(ui, self.value, value_room);
        let value_height = if long {
            galley.size().y.min(8.0 * LINE)
        } else {
            galley.size().y
        };
        let height = if long {
            8.0 + label_height.max(28.0) + 6.0 + value_height + 10.0
        } else {
            (label_height.max(value_height) + 8.0).max(36.0)
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Row, true, self.name));
        address(ui, &response, "output-row");
        if self.matched {
            ui.painter()
                .rect_filled(rect, 0.0, p.green.gamma_multiply(0.08));
        }
        ui.painter()
            .hline(rect.x_range(), rect.top() + 0.5, (1.0, p.line_soft));
        let left = rect.left() + 16.0;
        let head = if long {
            rect.top() + 8.0
        } else {
            rect.center().y - label_height / 2.0
        };
        let name_height = name.size().y;
        let name_right = left + name.size().x;
        name.paint(ui, Pos2::new(left, head), ink);
        if let Some(tag) = self.tag {
            let words = widgets::text(ui, tag, 10.5, 500.0, FontFamily::Proportional, p.text_2);
            let size = words.size() + Vec2::new(10.0, 2.0);
            let chip = Rect::from_min_size(Pos2::new(name_right + 8.0, head + 1.0), size);
            ui.painter()
                .rect_filled(chip, egui::CornerRadius::same(4), p.bg_3);
            ui.painter()
                .galley(chip.center() - words.size() / 2.0, words, p.text_2);
        }
        if let Some(meta) = meta {
            meta.paint(ui, Pos2::new(left, head + name_height + 2.0), p.text_2);
        }
        let place = if long {
            Rect::from_min_size(
                Pos2::new(left, rect.top() + 8.0 + label_height.max(28.0) + 6.0),
                Vec2::new(value_room, value_height),
            )
        } else {
            Rect::from_min_size(
                Pos2::new(
                    left + name_room + 12.0,
                    rect.center().y - value_height / 2.0,
                ),
                Vec2::new(value_room, value_height),
            )
        };
        self.value_of(ui, place, galley, long);
        if self.copyable {
            let top = if long {
                rect.top() + 8.0
            } else {
                rect.center().y - 14.0
            };
            let at = Rect::from_min_size(
                Pos2::new(rect.right() - 10.0 - 28.0, top),
                Vec2::splat(28.0),
            );
            let mut end = ui.new_child(UiBuilder::new().max_rect(at));
            CopyValue::new(self.copied.unwrap_or(self.value), self.name, language)
                .compact()
                .show(&mut end);
        }
        response
    }

    fn value_of(self, ui: &mut Ui, place: Rect, galley: Arc<Galley>, long: bool) {
        let mut cell = ui.new_child(UiBuilder::new().max_rect(place));
        if long {
            egui::ScrollArea::vertical()
                .id_salt(self.name)
                .max_height(place.height())
                .show(&mut cell, |ui| value_label(ui, galley));
        } else {
            value_label(&mut cell, galley);
        }
    }
}

pub(super) fn value_galley(ui: &Ui, value: &str, width: f32) -> Arc<Galley> {
    let p = palette(ui.ctx());
    let text = RichText::new(value)
        .font(FontId::new(12.5, FontFamily::Monospace))
        .color(p.text_0);
    egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        width.max(1.0),
        egui::TextStyle::Body,
    )
}

/// Selectable, as a value on a page is.
pub(super) fn value_label(ui: &mut Ui, galley: Arc<Galley>) {
    let response = ui.add(egui::Label::new(galley).selectable(true));
    address(ui, &response, "output-value");
}

/// Rows in a card, two abreast once it is wide enough.
pub fn result_rows(ui: &mut Ui, rows: &[ResultRow<'_>], language: Language) {
    let width = ui.available_width();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few rows"
    )]
    let across = ((width / ROW_MIN).floor() as usize).max(1);
    #[expect(clippy::cast_precision_loss, reason = "a few rows")]
    let each = width / across as f32;
    for line in rows.chunks(across) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for row in line {
                ui.allocate_ui(Vec2::new(each, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(each);
                        row.show(ui, language);
                    });
                });
            }
        });
    }
}
