//! Two documents compared by their values, whatever their format.

use egui::{CornerRadius, FontFamily, Pos2, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::convert::DataFormat;
use engine::tools::diff::{
    JsonChange, JsonChangeKind, JsonDiffAnswer, JsonDiffCounts, JsonDiffLine, JsonDiffRequest,
    JsonDiffRow, JsonDiffRowKind, JsonDiffSide, ValueSummary,
};
use i18n::{Language, Message, keys};

use super::{Kind, Layout, Mode, ROW, Sides, cell_head, gap, pill, rows_box, split_head};
use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{check, hint, label, note_import_into, paragraph, problem, text_area};
use crate::tools::{Drawn, ToolResult};
use crate::widgets::{
    self, Button, ChoiceMenu, Segmented, address, hovered, interact, towards, virtual_list,
};

const SAMPLE_A: &str = "# DevNotes, as released
name: devnotes
version: 0.9.1
features:
  - notes
  - tools
window:
  width: 1280
  height: 800
";
const SAMPLE_B: &str = "{
  \"name\": \"devnotes\",
  \"version\": \"0.9.2\",
  \"features\": [\"notes\", \"tools\"],
  \"window\": { \"width\": 1280, \"height\": 800 }
}";

const DEBOUNCE: f64 = 0.250;
const CHANGES_WIDTH: f32 = 300.0;

fn format_name(format: DataFormat) -> &'static str {
    match format {
        DataFormat::Json => "JSON",
        DataFormat::Yaml => "YAML",
        DataFormat::Toml => "TOML",
        DataFormat::Xml => "XML",
    }
}

pub(super) struct StructuredDiff {
    format_a: Option<DataFormat>,
    format_b: Option<DataFormat>,
    ignore_key_order: bool,
    ignore_whitespace: bool,
    layout: Layout,
    selected: Option<String>,
    answer: Live<JsonDiffRequest, JsonDiffAnswer>,
}

impl Default for StructuredDiff {
    fn default() -> Self {
        Self {
            format_a: None,
            format_b: None,
            ignore_key_order: true,
            ignore_whitespace: true,
            layout: Layout::Split,
            selected: None,
            answer: Live::slower(DEBOUNCE),
        }
    }
}

/// What a value is, as the list of changes says it: itself when short, its size otherwise.
fn described(summary: &ValueSummary, language: Language) -> String {
    use keys::tools::diff::structured::{ARRAY, OBJECT, SCALAR};
    match summary {
        ValueSummary::Scalar { text } => Message::new(SCALAR).with("text", text.as_str()),
        ValueSummary::Object { keys } => Message::new(OBJECT).with("count", *keys),
        ValueSummary::Array { items } => Message::new(ARRAY).with("count", *items),
    }
    .text(language)
}

fn kind_of(kind: JsonDiffRowKind) -> Kind {
    match kind {
        JsonDiffRowKind::Same => Kind::Same,
        JsonDiffRowKind::Added => Kind::Added,
        JsonDiffRowKind::Removed => Kind::Removed,
        JsonDiffRowKind::Modified => Kind::Modified,
    }
}

impl StructuredDiff {
    fn request(&self, sides: &Sides) -> Option<JsonDiffRequest> {
        (!sides.a.trim().is_empty() && !sides.b.trim().is_empty()).then(|| JsonDiffRequest {
            a: sides.a.clone(),
            b: sides.b.clone(),
            format_a: self.format_a,
            format_b: self.format_b,
            ignore_key_order: self.ignore_key_order,
            ignore_whitespace: self.ignore_whitespace,
        })
    }

    pub(super) fn update(&mut self, sides: &Sides, turn: &mut Turn<'_>) {
        let request = self.request(sides);
        self.answer.update(turn, request, api::tools::diff_json);
    }

    pub(super) fn forget_selection(&mut self) {
        self.selected = None;
    }

    /// Both sides, and each read by its shape again: a format left chosen would misread them.
    pub(super) fn fill_sample(&mut self, sides: &mut Sides) {
        *sides = Sides {
            a: SAMPLE_A.to_owned(),
            b: SAMPLE_B.to_owned(),
            ..Sides::default()
        };
        self.format_a = None;
        self.format_b = None;
    }

    /// The patch has operations: there is something to copy or to keep.
    pub(super) fn patch(&self) -> Option<&str> {
        let Some(JsonDiffAnswer::Compared { changes, patch, .. }) = self.answer.value() else {
            return None;
        };
        changes
            .iter()
            .any(|change| change.kind != JsonChangeKind::Reordered)
            .then_some(patch.as_str())
    }

    pub(super) fn result(&self, sides: &Sides, language: Language) -> Option<ToolResult> {
        Some(ToolResult {
            title: Message::new(keys::tools::diff::structured::NOTE_TITLE)
                .with("a", sides.title_of("a"))
                .with("b", sides.title_of("b"))
                .text(language),
            language: NoteLanguage::Json,
            content: self.patch()?.to_owned(),
            warning: None,
        })
    }

    /// What each side was read as, once Rust has said.
    fn read_as(&self, side: JsonDiffSide) -> Option<DataFormat> {
        match self.answer.value()? {
            JsonDiffAnswer::Compared {
                format_a, format_b, ..
            } => Some(if side == JsonDiffSide::A {
                *format_a
            } else {
                *format_b
            }),
            JsonDiffAnswer::Unreadable {
                side: read, format, ..
            } => (*read == side).then_some(*format),
            JsonDiffAnswer::NotStructured { .. } => None,
        }
    }

    pub(super) fn ui(
        &mut self,
        ui: &mut Ui,
        sides: &mut Sides,
        drawn: &mut Drawn<'_>,
    ) -> Option<Mode> {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let line = hint(ui, &tr(keys::tools::diff::DATA_LINE));
        address(ui, &line, "diff-data-line");
        let width = (ui.available_width() - 14.0) / 2.0;
        let mut asked = None;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            for (slot, side) in [("a", JsonDiffSide::A), ("b", JsonDiffSide::B)] {
                ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        if self.input(ui, sides, drawn, (slot, side)) {
                            asked = Some(Mode::Text);
                        }
                    });
                });
            }
        });
        self.compared(ui, sides, language);
        asked
    }

    /// `true` when « Compare as text » is taken.
    fn input(
        &mut self,
        ui: &mut Ui,
        sides: &mut Sides,
        drawn: &mut Drawn<'_>,
        (slot, side): (&'static str, JsonDiffSide),
    ) -> bool {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = sides.name_of(slot, language);
        let mark = slot.to_uppercase();
        ui.horizontal(|ui| {
            label(ui, &format!("{mark}  {name}"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let id = format!("structured-diff-open-{slot}");
                note_import_into(ui, drawn, ("structured", slot), &id);
                self.format_menu(ui, (slot, side), language);
            });
        });
        let (text, title) = if slot == "a" {
            (&mut sides.a, &mut sides.a_name)
        } else {
            (&mut sides.b, &mut sides.b_name)
        };
        let aloud = Message::new(keys::tools::diff::structured::INPUT_LABEL)
            .with("side", mark.as_str())
            .text(language);
        let id = format!("structured-diff-{slot}");
        let hint_words = tr(keys::tools::diff::structured::PLACEHOLDER);
        if text_area(ui, text, (&aloud, &id), &hint_words, 6).changed() {
            *title = None;
        }
        self.about_the_side(ui, (slot, side), language)
    }

    fn format_menu(&mut self, ui: &mut Ui, (slot, side): (&str, JsonDiffSide), language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let auto = tr(keys::tools::diff::structured::formats::AUTO);
        let options = [
            (None, auto.as_str()),
            (Some(DataFormat::Json), "JSON"),
            (Some(DataFormat::Yaml), "YAML"),
            (Some(DataFormat::Toml), "TOML"),
            (Some(DataFormat::Xml), "XML"),
        ];
        let label = Message::new(keys::tools::diff::structured::FORMAT)
            .with("side", slot.to_uppercase())
            .text(language);
        let id = format!("structured-diff-format-{slot}");
        let chosen = if side == JsonDiffSide::A {
            &mut self.format_a
        } else {
            &mut self.format_b
        };
        let automatic = chosen.is_none();
        ChoiceMenu::new(&label, &id, &options).show(ui, chosen);
        if automatic && let Some(read) = self.read_as(side) {
            let words = Message::new(keys::tools::diff::structured::READ_AS)
                .with("format", format_name(read))
                .text(language);
            let said = hint(ui, &words);
            address(ui, &said, &format!("structured-diff-read-{slot}"));
        }
    }

    /// What Rust could not make of this side.
    fn about_the_side(
        &self,
        ui: &mut Ui,
        (slot, side): (&str, JsonDiffSide),
        language: Language,
    ) -> bool {
        let tr = |key| i18n::translate(language, key, &[]);
        match self.answer.value() {
            Some(JsonDiffAnswer::NotStructured { sides }) if sides.contains(&side) => {
                let p = palette(ui.ctx());
                let words = tr(keys::tools::diff::structured::NOT_STRUCTURED);
                let said = paragraph(ui, &words, (12.5, 400.0), p.text_1);
                address(ui, &said, &format!("structured-diff-not-structured-{slot}"));
                let id = format!("structured-diff-as-text-{slot}");
                Button::new(&tr(keys::tools::diff::structured::AS_TEXT), &id)
                    .show(ui)
                    .clicked()
            }
            Some(JsonDiffAnswer::Unreadable {
                side: read,
                format,
                line,
                column,
            }) if *read == side => {
                let words = Message::new(keys::tools::diff::structured::UNREADABLE)
                    .with("format", format_name(*format))
                    .with("line", *line)
                    .with("column", *column)
                    .text(language);
                problem(ui, &words, &format!("structured-diff-unreadable-{slot}"));
                false
            }
            _ => false,
        }
    }

    fn compared(&mut self, ui: &mut Ui, sides: &Sides, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let answer = self.answer.value().cloned();
        let Some(JsonDiffAnswer::Compared {
            changes,
            counts,
            rows,
            rows_truncated,
            format_a,
            format_b,
            ..
        }) = answer
        else {
            if self.answer.value().is_none() {
                let said = hint(ui, &tr(keys::tools::diff::structured::EMPTY));
                address(ui, &said, "structured-diff-empty");
            }
            return;
        };
        self.bar(ui, counts, language);
        if changes.is_empty() {
            let said = hint(ui, &tr(keys::tools::diff::structured::IDENTICAL));
            address(ui, &said, "structured-diff-identical");
        } else {
            let width = ui.available_width();
            let left = (width - CHANGES_WIDTH - 14.0).max(width * 0.5);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                ui.allocate_ui(Vec2::new(left, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(left);
                        self.rows(ui, &rows, sides, language);
                    });
                });
                ui.vertical(|ui| self.changes(ui, &changes, language));
            });
            if rows_truncated {
                hint(ui, &tr(keys::tools::diff::structured::TRUNCATED));
            }
            let said = hint(ui, &tr(keys::tools::diff::structured::AS_JSON));
            address(ui, &said, "structured-diff-as-json");
        }
        if format_a == DataFormat::Yaml || format_b == DataFormat::Yaml {
            let said = hint(ui, &tr(keys::tools::diff::structured::YAML_NOTE));
            address(ui, &said, "structured-diff-yaml-note");
        }
        if format_a == DataFormat::Xml || format_b == DataFormat::Xml {
            let said = hint(ui, &tr(keys::tools::diff::structured::XML_NOTE));
            address(ui, &said, "structured-diff-xml-note");
        }
    }

    fn bar(&mut self, ui: &mut Ui, counts: JsonDiffCounts, language: Language) {
        use keys::tools::diff::structured as words;
        let p = palette(ui.ctx());
        let tr = |key| i18n::translate(language, key, &[]);
        let count = |key, count| Message::new(key).with("count", count).text(language);
        let options = [
            (Layout::Split, tr(words::layouts::SPLIT)),
            (Layout::Unified, tr(words::layouts::UNIFIED)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            pill(
                ui,
                &count(words::ADDED, counts.added),
                p.green,
                "structured-diff-added",
            );
            pill(
                ui,
                &count(words::REMOVED, counts.removed),
                p.red,
                "structured-diff-removed",
            );
            let modified = count(words::MODIFIED, counts.modified);
            pill(ui, &modified, p.amber_text, "structured-diff-modified");
            let order = tr(words::IGNORE_KEY_ORDER);
            check(
                ui,
                &mut self.ignore_key_order,
                &order,
                "structured-diff-ignore-order",
            );
            let spaces = tr(words::IGNORE_WHITESPACE);
            check(
                ui,
                &mut self.ignore_whitespace,
                &spaces,
                "structured-diff-ignore-spaces",
            );
            Segmented::new(&options, "structured-diff-layout").show(ui, &mut self.layout);
        });
    }

    fn rows(&self, ui: &mut Ui, rows: &[JsonDiffRow], sides: &Sides, language: Language) {
        let selected = self.selected.as_deref();
        match self.layout {
            Layout::Split => {
                #[expect(clippy::cast_precision_loss, reason = "rows on a screen")]
                let height = 34.0 + rows.len() as f32 * ROW + 8.0;
                rows_box(ui, "structured-diff-rows", height, |ui| {
                    split_head(ui, sides, language);
                    virtual_list(ui, "structured-diff-list", ROW, rows.len(), |ui, at| {
                        row(ui, &rows[at], selected);
                    });
                });
            }
            Layout::Unified => {
                let lines = unified_lines(rows);
                let height = super::unified_height(lines.len());
                rows_box(ui, "structured-diff-rows", height, |ui| {
                    virtual_list(ui, "structured-diff-unified", ROW, lines.len(), |ui, at| {
                        unified_line(ui, lines[at]);
                    });
                });
            }
        }
    }

    fn changes(&mut self, ui: &mut Ui, changes: &[JsonChange], language: Language) {
        let p = palette(ui.ctx());
        let count = u32::try_from(changes.len()).unwrap_or(u32::MAX);
        let words = Message::new(keys::tools::diff::structured::CHANGES_COUNT)
            .with("count", count)
            .text(language);
        paragraph(ui, &words, (12.0, 600.0), p.text_2);
        egui::ScrollArea::vertical()
            .id_salt("structured-diff-changes")
            .max_height(super::ROWS_MAX)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for change in changes {
                    let on = self.selected.as_deref() == Some(change.path.as_str());
                    if change_row(ui, change, on, language).clicked() {
                        self.selected = Some(change.path.clone());
                    }
                }
            });
    }
}

fn row(ui: &mut Ui, row: &JsonDiffRow, selected: Option<&str>) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    let shown = row.right.as_ref().or(row.left.as_ref());
    let label = shown.map_or("", |line| line.text.as_str());
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, label));
    address(ui, &response, "structured-diff-row");
    let kind = kind_of(row.kind);
    let (left, right) = rect.split_left_right_at_x(rect.center().x);
    for (rect, line, is_right) in [(left, &row.left, false), (right, &row.right, true)] {
        match line {
            Some(line) => cell(ui, rect, kind, is_right, line),
            None => gap(ui, rect),
        }
    }
    ui.painter()
        .vline(rect.center().x, rect.y_range(), (1.0, p.line_soft));
    if selected == Some(row.path.as_str()) {
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, p.amber_edge),
            egui::StrokeKind::Inside,
        );
        let id = response.id.with(row.path.as_str());
        if !ui.data(|data| data.get_temp::<bool>(id).unwrap_or(false)) {
            ui.data_mut(|data| data.insert_temp(id, true));
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
        }
    }
}

fn cell(ui: &Ui, rect: Rect, kind: Kind, right: bool, line: &JsonDiffLine) {
    let p = palette(ui.ctx());
    let code = cell_head(ui, rect, kind, right, line.number);
    let ink = if kind == Kind::Same {
        p.text_1
    } else {
        p.text_0
    };
    let galley = widgets::text(ui, &line.text, 12.0, 400.0, FontFamily::Monospace, ink);
    let at = Pos2::new(code.left(), code.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(code).galley(at, galley, ink);
}

/// The side-by-side rows, read one after the other.
fn unified_lines(rows: &[JsonDiffRow]) -> Vec<(Kind, bool, &JsonDiffLine)> {
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        if row.kind == JsonDiffRowKind::Same {
            lines.extend(row.left.iter().map(|line| (Kind::Same, false, line)));
            continue;
        }
        lines.extend(row.left.iter().map(|line| (Kind::Removed, false, line)));
        lines.extend(row.right.iter().map(|line| (Kind::Added, true, line)));
    }
    lines
}

fn unified_line(ui: &mut Ui, (kind, right, line): (Kind, bool, &JsonDiffLine)) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, &line.text));
    address(ui, &response, "structured-diff-line");
    cell(ui, rect, kind, right, line);
}

fn change_mark(kind: JsonChangeKind) -> &'static str {
    match kind {
        JsonChangeKind::Added => "+",
        JsonChangeKind::Removed => "-",
        JsonChangeKind::Modified | JsonChangeKind::Reordered => "~",
    }
}

fn change_row(ui: &mut Ui, change: &JsonChange, on: bool, language: Language) -> egui::Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let path = format!("{} {}", change_mark(change.kind), change.path);
    let detail = change_detail(change, language);
    let top = widgets::wrapped(ui, &path, (12.0, 500.0), p.text_0, width - 20.0);
    let under = widgets::wrapped(ui, &detail, (11.5, 400.0), p.text_2, width - 20.0);
    let corner = CornerRadius::same(7);
    let response = interact(
        ui,
        Vec2::new(width, top.size().y + under.size().y + 14.0),
        Sense::click(),
        "structured-diff-change",
        || WidgetInfo::selected(Role::Button, true, on, &change.path),
        corner,
    );
    let rect = response.rect;
    let fill = if on {
        p.bg_3
    } else {
        towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response))
    };
    ui.painter().rect_filled(rect, corner, fill);
    let height = top.size().y;
    top.paint(ui, rect.min + Vec2::new(10.0, 6.0), p.text_0);
    under.paint(ui, rect.min + Vec2::new(10.0, 8.0 + height), p.text_2);
    response
}

fn change_detail(change: &JsonChange, language: Language) -> String {
    if change.kind == JsonChangeKind::Reordered {
        return i18n::translate(language, keys::tools::diff::structured::REORDERED, &[]);
    }
    let said = |value: &Option<ValueSummary>| value.as_ref().map(|it| described(it, language));
    match (said(&change.before), said(&change.after)) {
        (Some(before), Some(after)) => format!("{before} → {after}"),
        (Some(one), None) | (None, Some(one)) => one,
        (None, None) => String::new(),
    }
}
