//! The find-and-replace bar over the code (the reference's `search-bar`): the field, how many
//! matches and which one, case, whole word and regular expression, then — once asked for — the
//! replacement, one or all. Ctrl+F opens it on the selection, Ctrl+H with the replace row,
//! Enter and Shift+Enter move between matches, Escape closes it. The code and a Note are searched
//! alike: each says what it holds through `Searchable`.

use egui::{FontFamily, Key, Margin, Modifiers, Stroke, TextEdit, Ui, Vec2};
use i18n::{Language, keys};
use text::{Count, Direction, Query};

use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::editor::Editor;
use crate::widgets::{self, IconButton, address};

#[derive(Debug, Default)]
pub(super) struct SearchBar {
    pub(super) open: bool,
    replacing: bool,
    pub(super) query: Query,
    /// The field takes the focus at the next frame.
    focus: bool,
}

/// What the bar looks in.
pub(super) trait Searchable {
    fn selected_text(&self) -> String;
    fn count(&self, query: &Query) -> Count;
    /// The selection onto the next match from `from`, wrapping round.
    fn seek(&mut self, query: &Query, from: SeekFrom, direction: Direction);
    /// The match selected replaced and the next one selected, or one selected first; the first
    /// line the text changed from.
    fn replace_one(&mut self, query: &Query) -> Option<usize>;
    /// Every match, as one step of undo.
    fn replace_all(&mut self, query: &Query) -> Option<usize>;
    fn focus(&mut self);
}

/// Where a seek starts: the selection's start (the query just changed) or past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SeekFrom {
    Start,
    Past,
}

impl Searchable for Editor {
    fn selected_text(&self) -> String {
        self.buffer.selected_text()
    }

    fn count(&self, query: &Query) -> Count {
        self.buffer.count(query)
    }

    fn seek(&mut self, query: &Query, from: SeekFrom, direction: Direction) {
        let range = self.buffer.selection().range();
        let at = match (from, direction) {
            (SeekFrom::Past, Direction::Forward) => range.end,
            _ => range.start,
        };
        self.buffer.seek(query, at, direction);
        self.reveal_selection();
    }

    fn replace_one(&mut self, query: &Query) -> Option<usize> {
        let edited = self
            .buffer
            .replace_one(query)
            .map(|edited| edited.first_line);
        self.reveal_selection();
        edited
    }

    fn replace_all(&mut self, query: &Query) -> Option<usize> {
        self.buffer
            .replace_all(query)
            .1
            .map(|edited| edited.first_line)
    }

    fn focus(&mut self) {
        Editor::focus(self);
    }
}

impl SearchBar {
    /// Ctrl+F, or Ctrl+H with the replace row: the selection, a word or so, is what is looked for.
    pub(super) fn open(&mut self, target: &impl Searchable, replacing: bool) {
        let selected = target.selected_text();
        if !selected.is_empty() && !selected.contains('\n') {
            self.query.search = selected;
        }
        self.open = true;
        self.replacing |= replacing;
        self.focus = true;
    }

    pub(super) fn close(&mut self) {
        self.open = false;
        self.replacing = false;
    }
}

/// Ctrl+F and Ctrl+H, read while the editor or the bar has the keys.
pub(super) fn shortcut(ui: &Ui) -> Option<bool> {
    ui.ctx().input_mut(|input| {
        if input.consume_key(Modifiers::COMMAND, Key::F) {
            Some(false)
        } else if input.consume_key(Modifiers::COMMAND, Key::H) {
            Some(true)
        } else {
            None
        }
    })
}

/// The bar; the first line a replacement changed.
pub(super) fn show(
    ui: &mut Ui,
    bar: &mut SearchBar,
    editor: &mut impl Searchable,
    language: Language,
) -> Option<usize> {
    let p = palette(ui.ctx());
    let mut edited = None;
    let count = editor.count(&bar.query);
    egui::Frame::new()
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            ui.horizontal(|ui| find_row(ui, bar, editor, (count, language)));
            if bar.replacing {
                ui.horizontal(|ui| replace_row(ui, bar, editor, language, &mut edited));
            }
        });
    let rect = ui.min_rect();
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        bar.close();
        editor.focus();
    }
    edited
}

/// The field, the count, the three options, previous, next, close.
fn find_row(
    ui: &mut Ui,
    bar: &mut SearchBar,
    editor: &mut impl Searchable,
    (count, language): (Count, Language),
) {
    let tr = |key| i18n::translate(language, key, &[]);
    let (icon, words) = if bar.replacing {
        (Icon::ChevronRight, tr(keys::editor::search::HIDE_REPLACE))
    } else {
        (Icon::ChevronRight, tr(keys::editor::search::SHOW_REPLACE))
    };
    if IconButton::new(icon, &words, "search-replace-toggle")
        .pressed(bar.replacing)
        .show(ui)
        .clicked()
    {
        bar.replacing = !bar.replacing;
    }
    let valid = bar.query.is_valid();
    let find = tr(keys::editor::search::FIND);
    let searched = field(ui, &mut bar.query.search, &find, "search-field", valid);
    if std::mem::take(&mut bar.focus) {
        searched.request_focus();
    }
    if searched.changed() {
        editor.seek(&bar.query, SeekFrom::Start, Direction::Forward);
    }
    if widgets::takes_keys(ui, &searched) || searched.lost_focus() {
        seek_on_enter(ui, bar, editor);
    }
    count_label(ui, count, language);
    toggle(
        ui,
        &mut bar.query.case_sensitive,
        "Aa",
        &tr(keys::editor::search::MATCH_CASE),
        "search-case",
    );
    toggle(
        ui,
        &mut bar.query.whole_word,
        "ab",
        &tr(keys::editor::search::WHOLE_WORD),
        "search-word",
    );
    toggle(
        ui,
        &mut bar.query.regexp,
        ".*",
        &tr(keys::editor::search::REGEXP),
        "search-regexp",
    );
    for (icon, key, author_id, direction) in [
        (
            Icon::ArrowUp,
            keys::editor::search::PREVIOUS,
            "search-previous",
            Direction::Backward,
        ),
        (
            Icon::ArrowDown,
            keys::editor::search::NEXT,
            "search-next",
            Direction::Forward,
        ),
    ] {
        if IconButton::new(icon, &tr(key), author_id)
            .show(ui)
            .clicked()
        {
            step(bar, editor, direction);
        }
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if IconButton::new(Icon::X, &tr(keys::editor::search::CLOSE), "search-close")
            .show(ui)
            .clicked()
        {
            bar.close();
        }
    });
}

/// The replacement, and replacing one or all; the first line changed into `edited`.
fn replace_row(
    ui: &mut Ui,
    bar: &mut SearchBar,
    editor: &mut impl Searchable,
    language: Language,
    edited: &mut Option<usize>,
) {
    let tr = |key| i18n::translate(language, key, &[]);
    ui.add_space(30.0);
    let replaced = field(
        ui,
        &mut bar.query.replace,
        &tr(keys::editor::search::REPLACE_WITH),
        "search-replace-field",
        true,
    );
    let enter = widgets::takes_keys(ui, &replaced)
        && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    let one = widgets::Button::new(&tr(keys::editor::search::REPLACE_ONE), "search-replace-one")
        .show(ui)
        .clicked();
    if one || enter {
        *edited = editor.replace_one(&bar.query);
    }
    if widgets::Button::new(&tr(keys::editor::search::REPLACE_ALL), "search-replace-all")
        .show(ui)
        .clicked()
    {
        *edited = editor.replace_all(&bar.query);
    }
}

/// Enter: the next match; Shift+Enter: the previous one.
fn seek_on_enter(ui: &Ui, bar: &SearchBar, editor: &mut impl Searchable) {
    let (next, previous) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::Enter),
            input.consume_key(Modifiers::SHIFT, Key::Enter),
        )
    });
    if next {
        step(bar, editor, Direction::Forward);
    } else if previous {
        step(bar, editor, Direction::Backward);
    }
}

fn step(bar: &SearchBar, editor: &mut impl Searchable, direction: Direction) {
    editor.seek(&bar.query, SeekFrom::Past, direction);
}

fn field(
    ui: &mut Ui,
    value: &mut String,
    name: &str,
    author_id: &str,
    valid: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let edge = if valid { p.line } else { p.red };
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 5))
        .show(ui, |ui| {
            let response = ui.add(
                TextEdit::singleline(value)
                    .frame(egui::Frame::new())
                    .margin(Margin::ZERO)
                    .hint_text(name)
                    .text_color(p.text_0)
                    .desired_width(220.0),
            );
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(name.to_owned()));
            address(ui, &response, author_id);
            response
        })
        .inner
}

/// « 3 matches », « No match », « 2 / 5 » once the selection is on one.
fn count_label(ui: &mut Ui, count: Count, language: Language) {
    let p = palette(ui.ctx());
    let total = u32::try_from(count.total).unwrap_or(u32::MAX);
    let mut words = i18n::translate(
        language,
        keys::editor::search::MATCHES,
        &[("count", total.into())],
    );
    if let Some(current) = count.current {
        words = format!(
            "{current} / {}{}",
            count.total,
            if count.capped { "+" } else { "" }
        );
    }
    let galley = widgets::text(ui, &words, 11.0, 400.0, FontFamily::Monospace, p.text_2);
    let response = ui.label(galley);
    address(ui, &response, "search-count");
}

fn toggle(ui: &mut Ui, on: &mut bool, glyph: &str, name: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let response = widgets::interact(
        ui,
        Vec2::new(26.0, 24.0),
        egui::Sense::click(),
        author_id,
        || egui::WidgetInfo::selected(egui::Role::Button, true, *on, name),
        egui::CornerRadius::same(6),
    );
    if *on {
        ui.painter()
            .rect_filled(response.rect, egui::CornerRadius::same(6), p.bg_3);
    }
    let ink = if *on { p.amber_text } else { p.text_1 };
    let galley = widgets::text(ui, glyph, 11.5, 600.0, FontFamily::Monospace, ink);
    ui.painter()
        .galley(response.rect.center() - galley.size() / 2.0, galley, ink);
    if response.clicked() {
        *on = !*on;
    }
}
