//! Two texts compared line by line, word by word or character by character.

use egui::text::LayoutJob;
use egui::{FontId, Pos2, Rect, Sense, TextFormat, Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::text_diff::{
    Granularity, TextDiffAnswer, TextDiffLine, TextDiffRequest, TextDiffRow, TextRowKind,
};
use i18n::{Key, Language, Message, keys};

use super::{Kind, Layout, Mode, ROW, Sides, cell_head, gap, pill, rows_box, split_head};
use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{check, hint, label, note_import_into, paragraph, text_area};
use crate::tools::{Drawn, ToolResult};
use crate::widgets::{Button, Segmented, address, virtual_list};

const SAMPLE_A: &str = "server {
    listen 80;
    server_name devnotes.fr;
    root /var/www/devnotes;
    gzip off;
}
";
const SAMPLE_B: &str = "server {
    listen 443 ssl;
    server_name devnotes.fr www.devnotes.fr;
    root /var/www/devnotes;
    gzip on;
    ssl_certificate /etc/ssl/devnotes.pem;
}
";

/// Two whole texts: a keystroke is not worth comparing them again.
const DEBOUNCE: f64 = 0.250;

pub(super) fn fill_sample(sides: &mut Sides) {
    *sides = Sides {
        a: SAMPLE_A.to_owned(),
        b: SAMPLE_B.to_owned(),
        ..Sides::default()
    };
}

#[expect(clippy::struct_excessive_bools, reason = "each is its own option")]
pub(super) struct TextDiff {
    granularity: Granularity,
    layout: Layout,
    trailing: bool,
    whitespace: bool,
    case: bool,
    endings: bool,
    answer: Live<TextDiffRequest, TextDiffAnswer>,
}

impl Default for TextDiff {
    fn default() -> Self {
        Self {
            granularity: Granularity::Lines,
            layout: Layout::Split,
            trailing: false,
            whitespace: false,
            case: false,
            endings: true,
            answer: Live::slower(DEBOUNCE),
        }
    }
}

fn kind_of(kind: TextRowKind) -> Kind {
    match kind {
        TextRowKind::Same => Kind::Same,
        TextRowKind::Added => Kind::Added,
        TextRowKind::Removed => Kind::Removed,
        TextRowKind::Modified => Kind::Modified,
    }
}

impl TextDiff {
    fn request(&self, sides: &Sides) -> Option<TextDiffRequest> {
        (!sides.a.is_empty() || !sides.b.is_empty()).then(|| TextDiffRequest {
            a: sides.a.clone(),
            b: sides.b.clone(),
            granularity: self.granularity,
            ignore_trailing_whitespace: self.trailing,
            ignore_all_whitespace: self.whitespace,
            ignore_case: self.case,
            ignore_line_endings: self.endings,
        })
    }

    pub(super) fn update(&mut self, sides: &Sides, turn: &mut Turn<'_>) {
        let request = self.request(sides);
        self.answer.update(turn, request, api::tools::diff_text);
    }

    /// The unified diff, once there is a difference to write.
    pub(super) fn unified(&self) -> Option<&str> {
        let answer = self.answer.value().filter(|it| !it.identical)?;
        Some(&answer.unified)
    }

    pub(super) fn result(&self, sides: &Sides, language: Language) -> Option<ToolResult> {
        Some(ToolResult {
            title: Message::new(keys::tools::diff::text::NOTE_TITLE)
                .with("a", sides.title_of("a"))
                .with("b", sides.title_of("b"))
                .text(language),
            language: NoteLanguage::Diff,
            content: self.unified()?.to_owned(),
            warning: None,
        })
    }

    /// The mode to go to, when the offer is taken.
    pub(super) fn ui(
        &mut self,
        ui: &mut Ui,
        sides: &mut Sides,
        drawn: &mut Drawn<'_>,
    ) -> Option<Mode> {
        inputs(ui, sides, drawn);
        self.settings(ui, drawn.language);
        let language = drawn.language;
        let Some(answer) = self.answer.value() else {
            hint(
                ui,
                &i18n::translate(language, keys::tools::diff::text::EMPTY, &[]),
            );
            return None;
        };
        let asked = offer(ui, answer, language);
        if answer.identical {
            let words = i18n::translate(language, keys::tools::diff::text::IDENTICAL, &[]);
            let said = hint(ui, &words);
            address(ui, &said, "text-diff-identical");
            return asked;
        }
        let granularity = self.granularity;
        let mut layout = self.layout;
        counts(ui, answer, granularity, &mut layout, language);
        match layout {
            Layout::Split => split(ui, answer, sides, language),
            Layout::Unified => unified(ui, &answer.unified),
        }
        self.layout = layout;
        asked
    }

    fn settings(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::diff::text::{granularities, ignore};
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (Granularity::Lines, tr(granularities::LINES)),
            (Granularity::Words, tr(granularities::WORDS)),
            (Granularity::Characters, tr(granularities::CHARACTERS)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            Segmented::new(&options, "text-diff-granularity").show(ui, &mut self.granularity);
            for (value, key, id) in [
                (
                    &mut self.trailing,
                    ignore::TRAILING,
                    "text-diff-ignore-trailing",
                ),
                (
                    &mut self.whitespace,
                    ignore::WHITESPACE,
                    "text-diff-ignore-whitespace",
                ),
                (&mut self.case, ignore::CASE, "text-diff-ignore-case"),
                (
                    &mut self.endings,
                    ignore::ENDINGS,
                    "text-diff-ignore-endings",
                ),
            ] {
                check(ui, value, &tr(key), id);
            }
        });
    }
}

fn inputs(ui: &mut Ui, sides: &mut Sides, drawn: &mut Drawn<'_>) {
    let width = (ui.available_width() - 14.0) / 2.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for slot in ["a", "b"] {
            ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    input(ui, sides, drawn, slot);
                });
            });
        }
    });
}

fn input(ui: &mut Ui, sides: &mut Sides, drawn: &mut Drawn<'_>, slot: &'static str) {
    let language = drawn.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let name = sides.name_of(slot, language);
    ui.horizontal(|ui| {
        label(ui, &format!("{}  {name}", slot.to_uppercase()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let id = format!("text-diff-import-{slot}");
            note_import_into(ui, drawn, ("text", slot), &id);
        });
    });
    let (text, title) = if slot == "a" {
        (&mut sides.a, &mut sides.a_name)
    } else {
        (&mut sides.b, &mut sides.b_name)
    };
    let side = slot.to_uppercase();
    let aloud = Message::new(keys::tools::diff::text::INPUT_LABEL)
        .with("side", side.as_str())
        .text(language);
    let id = format!("text-diff-{slot}");
    let typed = text_area(
        ui,
        text,
        (&aloud, &id),
        &tr(keys::tools::diff::text::PLACEHOLDER),
        6,
    );
    if typed.changed() {
        *title = None;
    }
}

/// Both sides are JSON: their values can be compared instead.
fn offer(ui: &mut Ui, answer: &TextDiffAnswer, language: Language) -> Option<Mode> {
    if !answer.both_json {
        return None;
    }
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    let mut asked = None;
    ui.horizontal(|ui| {
        let said = paragraph(
            ui,
            &tr(keys::tools::diff::text::BOTH_JSON),
            (12.5, 400.0),
            p.text_1,
        );
        address(ui, &said, "text-diff-json");
        if Button::new(
            &tr(keys::tools::diff::text::AS_VALUES),
            "text-diff-as-values",
        )
        .show(ui)
        .clicked()
        {
            asked = Some(Mode::Structured);
        }
    });
    asked
}

fn added_key(granularity: Granularity) -> Key {
    use keys::tools::diff::text::added;
    match granularity {
        Granularity::Lines => added::LINES,
        Granularity::Words => added::WORDS,
        Granularity::Characters => added::CHARACTERS,
    }
}

fn removed_key(granularity: Granularity) -> Key {
    use keys::tools::diff::text::removed;
    match granularity {
        Granularity::Lines => removed::LINES,
        Granularity::Words => removed::WORDS,
        Granularity::Characters => removed::CHARACTERS,
    }
}

fn counts(
    ui: &mut Ui,
    answer: &TextDiffAnswer,
    granularity: Granularity,
    layout: &mut Layout,
    language: Language,
) {
    use keys::tools::diff::text::layouts;
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let options = [
        (Layout::Split, tr(layouts::SPLIT)),
        (Layout::Unified, tr(layouts::UNIFIED)),
    ];
    let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let added = Message::new(added_key(granularity))
            .with("count", answer.added)
            .text(language);
        pill(ui, &added, p.green, "text-diff-added");
        let removed = Message::new(removed_key(granularity))
            .with("count", answer.removed)
            .text(language);
        pill(ui, &removed, p.red, "text-diff-removed");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Segmented::new(&options, "text-diff-layout").show(ui, layout);
        });
    });
}

fn split(ui: &mut Ui, answer: &TextDiffAnswer, sides: &Sides, language: Language) {
    #[expect(clippy::cast_precision_loss, reason = "rows on a screen")]
    let height = 34.0 + answer.rows.len() as f32 * ROW + 8.0;
    rows_box(ui, "text-diff-rows", height, |ui| {
        split_head(ui, sides, language);
        virtual_list(ui, "text-diff-list", ROW, answer.rows.len(), |ui, at| {
            row(ui, &answer.rows[at]);
        });
    });
    if answer.rows_truncated {
        hint(
            ui,
            &i18n::translate(language, keys::tools::diff::text::TRUNCATED, &[]),
        );
    }
}

fn row(ui: &mut Ui, row: &TextDiffRow) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    let label = row
        .right
        .as_ref()
        .or(row.left.as_ref())
        .map_or_else(String::new, |line| {
            line.pieces
                .iter()
                .map(|piece| piece.text.as_str())
                .collect()
        });
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Row, true, &label));
    address(ui, &response, "text-diff-row");
    let kind = kind_of(row.kind);
    let (left, right) = rect.split_left_right_at_x(rect.center().x);
    for (rect, line, is_right) in [(left, &row.left, false), (right, &row.right, true)] {
        match line {
            Some(line) => cell(ui, rect, kind, is_right, line),
            None => gap(ui, rect),
        }
    }
    let p = palette(ui.ctx());
    ui.painter()
        .vline(rect.center().x, rect.y_range(), (1.0, p.line_soft));
}

fn cell(ui: &Ui, rect: Rect, kind: Kind, right: bool, line: &TextDiffLine) {
    let p = palette(ui.ctx());
    let code = cell_head(ui, rect, kind, right, line.number);
    let ink = if kind == Kind::Same {
        p.text_1
    } else {
        p.text_0
    };
    let hue = if right { p.green } else { p.red };
    let mut job = LayoutJob::default();
    for piece in &line.pieces {
        let background = if piece.changed && kind == Kind::Modified {
            hue.gamma_multiply(0.28)
        } else {
            egui::Color32::TRANSPARENT
        };
        let format = TextFormat {
            background,
            ..TextFormat::simple(FontId::monospace(12.0), ink)
        };
        job.append(&piece.text, 0.0, format);
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let at = Pos2::new(code.left(), code.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(code).galley(at, galley, ink);
}

/// A line of `diff -u`, by how it starts.
fn unified_ink(line: &str, ui: &Ui) -> (egui::Color32, egui::Color32) {
    let p = palette(ui.ctx());
    let none = egui::Color32::TRANSPARENT;
    if line.starts_with("+++") || line.starts_with("---") {
        (none, p.text_2)
    } else if line.starts_with("@@") {
        (none, p.blue)
    } else if line.starts_with('+') {
        (p.green.gamma_multiply(0.12), p.text_0)
    } else if line.starts_with('-') {
        (p.red.gamma_multiply(0.12), p.text_0)
    } else {
        (none, p.text_1)
    }
}

fn unified(ui: &mut Ui, text: &str) {
    let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
    rows_box(
        ui,
        "text-diff-unified",
        super::unified_height(lines.len()),
        |ui| {
            virtual_list(ui, "text-diff-unified-list", ROW, lines.len(), |ui, at| {
                let width = ui.available_width();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
                response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Row, true, lines[at]));
                address(ui, &response, "text-diff-unified-line");
                let (tint, ink) = unified_ink(lines[at], ui);
                ui.painter().rect_filled(rect, 0.0, tint);
                let family = egui::FontFamily::Monospace;
                let galley = crate::widgets::text(ui, lines[at], 12.0, 400.0, family, ink);
                let at = Pos2::new(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0);
                ui.painter().with_clip_rect(rect).galley(at, galley, ink);
            });
        },
    );
}
