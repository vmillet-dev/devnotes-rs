//! A reference's table: a search that filters as it is typed, the rows under their groups, a
//! count, and each row's key to copy.

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, RichText, Role, Sense, Stroke,
    Ui, UiBuilder, Vec2, WidgetInfo,
};
use i18n::{Language, Message, keys};

use super::CopyValue;
use crate::api;
use crate::theme::palette;
use crate::widgets::{self, SearchField, address};

const SEARCH_MAX: f32 = 480.0;
const COPY_COLUMN: f32 = 44.0;
const CARD_GAP: f32 = 16.0;

pub struct Group {
    pub id: String,
    pub label: String,
}

/// `share` of the card, or what is left.
pub struct Column {
    pub label: String,
    pub share: Option<f32>,
}

/// What a row shows in one column: a word, and what is said under it.
#[derive(Default)]
pub struct Cell {
    pub text: String,
    pub mono: bool,
    pub ink: Option<Color32>,
    pub below: Option<String>,
}

impl Cell {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    pub fn mono(text: impl Into<String>) -> Self {
        Self {
            mono: true,
            ..Self::text(text)
        }
    }

    #[must_use]
    pub fn ink(mut self, ink: Color32) -> Self {
        self.ink = Some(ink);
        self
    }

    #[must_use]
    pub fn below(mut self, words: impl Into<String>) -> Self {
        self.below = Some(words.into());
        self
    }
}

pub struct ReferenceRow {
    /// Unique in its table, and what the row's button copies: `404`, `Content-Type`.
    pub key: String,
    pub group: String,
    pub cells: Vec<Cell>,
    /// What a search reads, folded once.
    pub searchable: Vec<String>,
    /// What the row's button copies instead of the key, and its name aloud.
    pub copy: Option<(String, String)>,
}

impl ReferenceRow {
    pub fn new(key: impl Into<String>, group: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self {
            key: key.into(),
            group: group.into(),
            cells,
            searchable: Vec::new(),
            copy: None,
        }
    }

    #[must_use]
    pub fn searched_by<'a>(mut self, texts: impl IntoIterator<Item = &'a str>) -> Self {
        self.searchable = texts.into_iter().map(api::tools::fold).collect();
        self
    }

    pub fn matches(&self, folded: &str) -> bool {
        self.searchable.iter().any(|text| text.contains(folded))
    }
}

pub struct ReferenceTable<'a> {
    pub rows: &'a [ReferenceRow],
    pub groups: &'a [Group],
    pub columns: &'a [Column],
    pub caption: &'a str,
    pub placeholder: &'a str,
    /// A card is no narrower than this: as many abreast as the width holds.
    pub card_width: f32,
    /// The row the home's search opened this reference on.
    pub lit: Option<&'a str>,
    /// For a reference that reads a query its own way: a code in any base.
    pub matches: Option<fn(&ReferenceRow, &str) -> bool>,
}

impl ReferenceTable<'_> {
    /// `query` is the tool's: a reference found again is searched as it was.
    pub fn show(&self, ui: &mut Ui, query: &mut String, language: Language) -> Response {
        ui.spacing_mut().item_spacing.y = 12.0;
        let field = self.head(ui, query, language);
        let folded = api::tools::fold(query.trim());
        let shown: Vec<&ReferenceRow> = self
            .rows
            .iter()
            .filter(|row| match self.matches {
                _ if folded.is_empty() => true,
                Some(matches) => matches(row, query),
                None => row.matches(&folded),
            })
            .collect();
        let cards: Vec<(&Group, Vec<&ReferenceRow>)> = self
            .groups
            .iter()
            .map(|group| {
                let rows = shown.iter().copied().filter(|row| row.group == group.id);
                (group, rows.collect::<Vec<_>>())
            })
            .filter(|(_, rows)| !rows.is_empty())
            .collect();
        self.count(ui, query, shown.len(), language);
        if cards.is_empty() {
            if !folded.is_empty() {
                let words = Message::new(keys::tools::reference::EMPTY)
                    .with("query", query.trim())
                    .text(language);
                let said = super::hint(ui, &words);
                address(ui, &said, "reference-empty");
            }
            return field;
        }
        egui::ScrollArea::vertical()
            .id_salt("reference-table")
            .auto_shrink(false)
            .show(ui, |ui| self.cards(ui, &cards, language));
        field
    }

    fn head(&self, ui: &mut Ui, query: &mut String, language: Language) -> Response {
        let label = Message::new(keys::tools::reference::SEARCH_LABEL)
            .with("what", self.caption)
            .text(language);
        let width = ui.available_width().min(SEARCH_MAX);
        SearchField::new(query, &label, "reference-search")
            .hint(self.placeholder)
            .width(width)
            .show(ui)
            .field
    }

    fn count(&self, ui: &mut Ui, query: &str, shown: usize, language: Language) {
        let total = u32::try_from(self.rows.len()).unwrap_or(u32::MAX);
        let words = if query.trim().is_empty() {
            Message::new(keys::tools::reference::COUNT).with("count", total)
        } else {
            Message::new(keys::tools::reference::FOUND)
                .with("count", u32::try_from(shown).unwrap_or(u32::MAX))
                .with("total", total)
        };
        let said = super::hint(ui, &words.text(language));
        address(ui, &said, "reference-count");
    }

    fn cards(&self, ui: &mut Ui, cards: &[(&Group, Vec<&ReferenceRow>)], language: Language) {
        let width = ui.available_width();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few columns"
        )]
        let across = (((width + CARD_GAP) / (self.card_width + CARD_GAP)).floor() as usize).max(1);
        #[expect(clippy::cast_precision_loss, reason = "a few columns")]
        let each = (width - CARD_GAP * (across - 1) as f32) / across as f32;
        let total: usize = cards.iter().map(|(_, rows)| rows.len() + 3).sum();
        let mut stacks = vec![Vec::new(); across];
        let (mut column, mut filled) = (0, 0);
        for card in cards {
            if filled * across >= total * (column + 1) && column + 1 < across {
                column += 1;
            }
            stacks[column].push(card);
            filled += card.1.len() + 3;
        }
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = CARD_GAP;
            for stack in stacks {
                ui.allocate_ui(Vec2::new(each, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(each);
                        ui.spacing_mut().item_spacing.y = CARD_GAP;
                        for (group, rows) in stack {
                            self.card(ui, group, rows, language);
                        }
                    });
                });
            }
        });
    }

    fn card(&self, ui: &mut Ui, group: &Group, rows: &[&ReferenceRow], language: Language) {
        let p = palette(ui.ctx());
        let shown = egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, p.line_soft))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                card_head(ui, group, rows.len());
                let widths = self.widths(ui.available_width());
                self.column_heads(ui, &widths);
                for row in rows {
                    self.row(ui, row, &widths, language);
                }
            });
        let label = group.label.as_str();
        shown
            .response
            .widget_info(|| WidgetInfo::labeled(Role::Table, true, label));
        address(ui, &shown.response, "reference-group");
    }

    /// The columns with a share take it, the others what is left.
    fn widths(&self, width: f32) -> Vec<f32> {
        let room = width - COPY_COLUMN;
        let taken: f32 = self.columns.iter().filter_map(|it| it.share).sum();
        let free = self.columns.iter().filter(|it| it.share.is_none()).count();
        #[expect(clippy::cast_precision_loss, reason = "a few columns")]
        let rest = (1.0 - taken).max(0.0) / free.max(1) as f32;
        self.columns
            .iter()
            .map(|column| column.share.unwrap_or(rest) * room)
            .collect()
    }

    fn column_heads(&self, ui: &mut Ui, widths: &[f32]) {
        let p = palette(ui.ctx());
        let total = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 26.0), Sense::hover());
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 0.5, (1.0, p.line_soft));
        let mut left = rect.left();
        for (column, width) in self.columns.iter().zip(widths) {
            let words = column.label.to_uppercase();
            let galley = widgets::one_line(ui, &words, (10.5, 600.0), p.text_2, width - 24.0);
            let at = Pos2::new(left + 12.0, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(at, galley, p.text_2);
            left += width;
        }
    }

    fn row(&self, ui: &mut Ui, row: &ReferenceRow, widths: &[f32], language: Language) {
        let p = palette(ui.ctx());
        let laid: Vec<_> = row
            .cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| laid_cell(ui, cell, width - 24.0))
            .collect();
        let tallest = laid.iter().map(LaidCell::height).fold(0.0, f32::max);
        let total = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(total, (tallest + 14.0).max(34.0)), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Row, true, &row.key));
        address(ui, &response, "reference-row");
        let lit = self.lit == Some(row.key.as_str());
        if lit {
            ui.painter()
                .rect_filled(rect, 0.0, p.amber_fill.gamma_multiply(0.12));
            let edge = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            ui.painter().rect_filled(edge, 0.0, p.amber_edge);
            if ui.data_mut(|data| !data.get_temp::<bool>(response.id).unwrap_or(false)) {
                ui.data_mut(|data| data.insert_temp(response.id, true));
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, p.bg_2);
        }
        ui.painter()
            .hline(rect.x_range(), rect.top() + 0.5, (1.0, p.line_soft));
        let mut left = rect.left();
        for (cell, width) in laid.into_iter().zip(widths) {
            cell.paint(ui, Pos2::new(left + 12.0, rect.top() + 7.0));
            left += width;
        }
        let at = Rect::from_center_size(
            Pos2::new(rect.right() - COPY_COLUMN / 2.0, rect.center().y),
            Vec2::splat(28.0),
        );
        let mut end = ui.new_child(UiBuilder::new().id_salt(&row.key).max_rect(at));
        let (value, what) = row
            .copy
            .as_ref()
            .map_or((row.key.as_str(), row.key.as_str()), |(value, what)| {
                (value.as_str(), what.as_str())
            });
        CopyValue::new(value, what, language)
            .compact()
            .show(&mut end);
    }
}

fn card_head(ui: &mut Ui, group: &Group, count: usize) {
    let p = palette(ui.ctx());
    let title = widgets::wrapped(ui, &group.label, (12.5, 600.0), p.text_0, f32::INFINITY);
    let number = count.to_string();
    let family = FontFamily::Proportional;
    let number = widgets::text(ui, &number, 12.5, 400.0, family, p.text_2);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, title.size().y + 16.0), Sense::hover());
    let title_width = title.size().x;
    title.paint(ui, rect.min + Vec2::new(12.0, 12.0), p.text_0);
    let at = rect.min + Vec2::new(12.0 + title_width + 8.0, 12.0);
    ui.painter().galley(at, number, p.text_2);
}

struct LaidCell {
    text: std::sync::Arc<egui::Galley>,
    ink: Color32,
    below: Option<widgets::Wrapped>,
    below_ink: Color32,
}

impl LaidCell {
    fn height(&self) -> f32 {
        self.text.size().y + self.below.as_ref().map_or(0.0, |it| it.size().y + 2.0)
    }

    fn paint(self, ui: &Ui, at: Pos2) {
        let height = self.text.size().y;
        ui.painter().galley(at, self.text, self.ink);
        if let Some(below) = self.below {
            below.paint(ui, at + Vec2::new(0.0, height + 2.0), self.below_ink);
        }
    }
}

fn laid_cell(ui: &Ui, cell: &Cell, width: f32) -> LaidCell {
    let p = palette(ui.ctx());
    let ink = cell.ink.unwrap_or(p.text_0);
    let family = if cell.mono {
        FontFamily::Monospace
    } else {
        FontFamily::Proportional
    };
    let text = RichText::new(&cell.text)
        .font(FontId::new(12.5, family))
        .color(ink);
    let text = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        width.max(1.0),
        egui::TextStyle::Body,
    );
    let below = cell
        .below
        .as_deref()
        .map(|words| widgets::wrapped(ui, words, (11.5, 400.0), p.text_2, width));
    LaidCell {
        text,
        ink,
        below,
        below_ink: p.text_2,
    }
}
