//! What a reader counts and what a program does, both Rust's; the page draws the bars.

use egui::{
    CornerRadius, FontFamily, Margin, Pos2, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::stats::{Frequency, Invisible, StatsRequest, TextStats};
use i18n::{Key, Language, Message, keys};

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{check, columns, hint, note_import, section_head, section_title, text_area};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{self, address};

const COUNT_MIN: f32 = 230.0;
const ROW: f32 = 26.0;

#[derive(Default)]
pub(super) struct TextStatsTool {
    text: String,
    fold_case: bool,
    count_whitespace: bool,
    answer: Live<StatsRequest, TextStats>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<TextStatsTool>::default()
}

/// Each count: its name, what is said under it, its address and its figure.
fn counts(stats: &TextStats) -> [(Key, Option<Key>, &'static str, u32); 9] {
    use keys::tools::text_stats::{hints, rows};
    [
        (
            rows::CHARACTERS,
            Some(hints::CHARACTERS),
            "characters",
            stats.characters,
        ),
        (
            rows::NON_WHITESPACE,
            None,
            "non-whitespace",
            stats.non_whitespace,
        ),
        (rows::WORDS, None, "words", stats.words),
        (rows::LINES, None, "lines", stats.lines),
        (
            rows::NON_EMPTY_LINES,
            None,
            "non-empty-lines",
            stats.non_empty_lines,
        ),
        (rows::PARAGRAPHS, None, "paragraphs", stats.paragraphs),
        (rows::CODE_POINTS, None, "code-points", stats.code_points),
        (
            rows::UTF16_UNITS,
            Some(hints::UTF16_UNITS),
            "utf16-units",
            stats.utf16_units,
        ),
        (rows::UTF8_BYTES, None, "utf8-bytes", stats.utf8_bytes),
    ]
}

fn invisible_name(invisible: Invisible) -> Key {
    use keys::tools::text_stats::invisible as name;
    match invisible {
        Invisible::Space => name::SPACE,
        Invisible::Tab => name::TAB,
        Invisible::LineBreak => name::LINE_BREAK,
        Invisible::NoBreakSpace => name::NO_BREAK_SPACE,
        Invisible::NarrowNoBreakSpace => name::NARROW_NO_BREAK_SPACE,
        Invisible::OtherSpace => name::OTHER_SPACE,
        Invisible::ZeroWidthSpace => name::ZERO_WIDTH_SPACE,
        Invisible::ZeroWidthJoiner => name::ZERO_WIDTH_JOINER,
        Invisible::ZeroWidthNonJoiner => name::ZERO_WIDTH_NON_JOINER,
        Invisible::ByteOrderMark => name::BYTE_ORDER_MARK,
        Invisible::SoftHyphen => name::SOFT_HYPHEN,
        Invisible::Control => name::CONTROL,
    }
}

/// A count is read, not pasted: the label, and its figure at the right of the row.
fn count_cell(
    ui: &mut Ui,
    (name, under, id, figure): (Key, Option<Key>, &'static str, u32),
    language: Language,
) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let name = i18n::translate(language, name, &[]);
    let figure = figure.to_string();
    let label = widgets::wrapped(ui, &name, (12.0, 400.0), p.text_2, width * 0.6);
    let under = under.map(|key| {
        let words = i18n::translate(language, key, &[]);
        widgets::wrapped(ui, &words, (10.5, 400.0), p.text_2, width * 0.6)
    });
    let number = widgets::text(ui, &figure, 16.0, 400.0, FontFamily::Monospace, p.text_0);
    let left = label.size().y + under.as_ref().map_or(0.0, |it| it.size().y);
    let height = left.max(number.size().y) + 18.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::Label, true, &name);
        info.current_text_value = Some(figure.clone());
        info
    });
    address(ui, &response, &format!("text-stats-count-{id}"));
    ui.painter()
        .hline(rect.x_range(), rect.bottom() - 0.5, (1.0, p.line_soft));
    let label_height = label.size().y;
    label.paint(ui, rect.min + Vec2::new(0.0, 9.0), p.text_2);
    if let Some(under) = under {
        under.paint(ui, rect.min + Vec2::new(0.0, 9.0 + label_height), p.text_2);
    }
    let at = Pos2::new(
        rect.right() - number.size().x,
        rect.center().y - number.size().y / 2.0,
    );
    ui.painter().galley(at, number, p.text_0);
}

fn count_card(ui: &mut Ui, stats: &TextStats, language: Language) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 2))
        .show(ui, |ui| {
            let width = ui.available_width();
            ui.set_width(width);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a few columns"
            )]
            let across = (((width + 24.0) / (COUNT_MIN + 24.0)).floor() as usize).max(1);
            #[expect(clippy::cast_precision_loss, reason = "a few columns")]
            let each = (width - 24.0 * (across - 1) as f32) / across as f32;
            ui.spacing_mut().item_spacing = Vec2::new(24.0, 0.0);
            for line in counts(stats).chunks(across) {
                ui.horizontal_top(|ui| {
                    for count in line {
                        ui.allocate_ui(Vec2::new(each, 0.0), |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(each);
                                count_cell(ui, *count, language);
                            });
                        });
                    }
                });
            }
        });
}

/// The columns of `frequency_row`, named.
fn frequency_head(ui: &mut Ui, language: Language) {
    use keys::tools::text_stats as words;
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    let middle = rect.center().y;
    let column = |key, left: f32, right: bool| {
        let family = FontFamily::Proportional;
        let shown = widgets::text(ui, &tr(key), 11.0, 500.0, family, p.text_2);
        let x = rect.left() + left - if right { shown.size().x } else { 0.0 };
        let at = Pos2::new(x, middle - shown.size().y / 2.0);
        ui.painter().galley(at, shown, p.text_2);
    };
    column(words::CHARACTER, 10.0, false);
    column(words::CODE_POINT, width * 0.34, false);
    column(words::COUNT, width * 0.68, true);
    column(words::SHARE, width - 10.0, true);
}

fn frequency_row(ui: &mut Ui, frequency: &Frequency, most: u32, language: Language) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    let label = frequency.invisible.map_or_else(
        || frequency.character.clone(),
        |it| i18n::translate(language, invisible_name(it), &[]),
    );
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::Row, true, &label);
        info.current_text_value = Some(frequency.count.to_string());
        info
    });
    address(ui, &response, "text-stats-frequency");
    ui.painter()
        .hline(rect.x_range(), rect.top() + 0.5, (1.0, p.line_soft));
    let middle = rect.center().y;
    let cell = |ink| {
        let family = FontFamily::Monospace;
        move |ui: &Ui, words: &str, size: f32| widgets::text(ui, words, size, 400.0, family, ink)
    };
    let name = if frequency.invisible.is_some() {
        widgets::one_line(ui, &label, (11.0, 400.0), p.teal, width * 0.3)
    } else {
        cell(p.text_0)(ui, &label, 13.0)
    };
    let at = Pos2::new(rect.left() + 10.0, middle - name.size().y / 2.0);
    ui.painter().galley(at, name, p.text_0);
    let code = cell(p.text_2)(ui, &frequency.code_points, 11.0);
    let at = Pos2::new(rect.left() + width * 0.34, middle - code.size().y / 2.0);
    ui.painter().galley(at, code, p.text_2);
    let number = cell(p.text_0)(ui, &frequency.count.to_string(), 12.0);
    let at = Pos2::new(
        rect.left() + width * 0.68 - number.size().x,
        middle - number.size().y / 2.0,
    );
    ui.painter().galley(at, number, p.text_0);
    let share = Rect::from_min_max(
        Pos2::new(rect.left() + width * 0.7, rect.top() + 6.0),
        Pos2::new(rect.right() - 10.0, rect.bottom() - 6.0),
    );
    #[expect(clippy::cast_precision_loss, reason = "a share on screen")]
    let filled = frequency.count as f32 / most.max(1) as f32;
    let bar = Rect::from_min_size(share.min, Vec2::new(share.width() * filled, share.height()));
    ui.painter().rect_filled(
        bar,
        CornerRadius::same(2),
        p.amber_fill.gamma_multiply(0.25),
    );
    let percent = format!("{:.1} %", frequency.share);
    let percent = cell(p.text_1)(ui, &percent, 11.0);
    let at = Pos2::new(
        share.right() - percent.size().x,
        middle - percent.size().y / 2.0,
    );
    ui.painter().galley(at, percent, p.text_1);
}

impl TextStatsTool {
    fn request(&self) -> Option<StatsRequest> {
        (!self.text.is_empty()).then(|| StatsRequest {
            text: self.text.clone(),
            fold_case: self.fold_case,
            count_whitespace: self.count_whitespace,
        })
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_head(ui, &tr(keys::tools::INPUT), |ui| {
            note_import(ui, drawn, "text");
        });
        text_area(
            ui,
            &mut self.text,
            (
                &tr(keys::tools::text_stats::INPUT_LABEL),
                "text-stats-input",
            ),
            &tr(keys::tools::text_stats::PLACEHOLDER),
            14,
        );
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::text_stats::COUNTS));
        let Some(stats) = self.answer.value() else {
            hint(ui, &tr(keys::tools::text_stats::EMPTY));
            return;
        };
        count_card(ui, stats, language);
        let distinct = Message::new(keys::tools::text_stats::DISTINCT)
            .with("count", stats.distinct)
            .text(language);
        section_head(ui, &tr(keys::tools::text_stats::FREQUENCY), |ui| {
            let said = hint(ui, &distinct);
            address(ui, &said, "text-stats-distinct");
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let words = tr(keys::tools::text_stats::FOLD_CASE);
            check(ui, &mut self.fold_case, &words, "text-stats-fold");
            let words = tr(keys::tools::text_stats::COUNT_WHITESPACE);
            check(
                ui,
                &mut self.count_whitespace,
                &words,
                "text-stats-whitespace",
            );
        });
        let Some(stats) = self.answer.value() else {
            return;
        };
        let p = palette(ui.ctx());
        let most = stats
            .frequencies
            .iter()
            .map(|it| it.count)
            .max()
            .unwrap_or(1);
        egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                frequency_head(ui, language);
                for frequency in &stats.frequencies {
                    frequency_row(ui, frequency, most, language);
                }
            });
        if stats.frequencies_truncated {
            let shown = u32::try_from(stats.frequencies.len()).unwrap_or(u32::MAX);
            let words = Message::new(keys::tools::text_stats::TRUNCATED)
                .with("shown", shown)
                .text(language);
            let said = hint(ui, &words);
            address(ui, &said, "text-stats-truncated");
        }
    }
}

impl Tool for TextStatsTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.answer.update(turn, request, api::tools::text_stats);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let stats = self.answer.value()?;
        let rows: Vec<(String, u32)> = counts(stats)
            .into_iter()
            .map(|(name, _, _, figure)| (i18n::translate(language, name, &[]), figure))
            .collect();
        let width = rows.iter().map(|(name, _)| name.chars().count()).max()?;
        let content: Vec<String> = rows
            .iter()
            .map(|(name, figure)| format!("{name:width$}  {figure}"))
            .collect();
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::text_stats::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: content.join("\n"),
            warning: None,
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, language: Language) {
        self.text = i18n::translate(language, keys::tools::text_stats::SAMPLE, &[]);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}
