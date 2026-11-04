//! The one palette, drawn over whichever area is on screen. The focus stays in the field — the
//! arrows move a highlight, not the focus, which would lose what is being typed — so there is
//! one current row, and Enter or a click opens it, as everywhere else in the application.

pub mod commands;
pub mod options;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Id, Key, Margin, Modifiers, Rect, Role,
    Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use engine::notes::kind::NoteKind;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::api;
use crate::state::AppState;
use crate::state::palette::{Chip, Mode, Scope};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, IconButton};
use options::{Chosen, Group, Item};

const WIDTH: f32 = 640.0;

/// What the palette asks, applied at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Type(String),
    Chip(Chip),
    Unchip(usize),
    /// Opens it, runs it, or creates it.
    Choose(Chosen),
    /// A note's text, through its fields form when it has fields.
    Copy(String),
    Close,
}

/// The highlighted row, and the rows it was counted among.
#[derive(Default)]
pub struct PaletteView {
    highlighted: usize,
    keys: Vec<String>,
}

fn ask(actions: &mut Actions, asked: Asked) {
    actions.push(Action::Palette(asked));
}

/// A chip narrows the search; anything else is opened, run or created.
fn choose(item: &Item) -> Asked {
    match (item, item.chosen()) {
        (_, Some(chosen)) => Asked::Choose(chosen),
        (Item::Chip { chip, .. }, None) => Asked::Chip(chip.clone()),
        (_, None) => Asked::Close,
    }
}

fn field_id() -> Id {
    Id::unique("palette-input")
}

pub fn show(ctx: &egui::Context, state: &AppState, view: &mut PaletteView, actions: &mut Actions) {
    if !state.palette.is_open() {
        *view = PaletteView::default();
        return;
    }
    let groups = options::groups(state);
    let items: Vec<&Item> = groups.iter().flat_map(|group| &group.items).collect();
    let keys: Vec<String> = items.iter().map(|item| item.key()).collect();
    // Back to the first whenever the list changes: what was under it may have gone.
    if keys != view.keys {
        view.keys = keys;
        view.highlighted = 0;
    }
    view.highlighted = view.highlighted.min(items.len().saturating_sub(1));
    // The fields form sits in front of the palette, and the keys are its own.
    if !state.fill.is_open() {
        keyboard(ctx, state, &items, view, actions);
    }
    let p = palette(ctx);
    let top = ctx.content_rect().height() * 0.1;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(14))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(12));
    // Near the top rather than centred: the list grows downwards, and a centred panel would
    // slide under the pointer as it fills.
    let area =
        egui::Modal::default_area(Id::unique("palette")).anchor(Align2::CENTER_TOP, vec2(0.0, top));
    let shown = egui::Modal::new(Id::unique("palette"))
        .area(area)
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ctx, |ui| panel(ui, state, (&groups, &items), view, actions));
    if shown.should_close() && !state.fill.is_open() {
        ask(actions, Asked::Close);
    }
}

/// Taken before the field sees them: it would move its caret with the arrows and lose the focus
/// on Enter and Tab.
fn keyboard(
    ctx: &egui::Context,
    state: &AppState,
    items: &[&Item],
    view: &mut PaletteView,
    actions: &mut Actions,
) {
    let current = items.get(view.highlighted).copied();
    let last = items.len().saturating_sub(1);
    ctx.input_mut(|input| {
        // Stops at both ends: wrapping around would lose track of where one is.
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            view.highlighted = (view.highlighted + 1).min(last);
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            view.highlighted = view.highlighted.saturating_sub(1);
        }
        if input.consume_key(Modifiers::SHIFT, Key::Tab) {
            view.highlighted = view.highlighted.saturating_sub(1);
        }
        // Tab chooses a chip offered after `@` or `#`, and is a slower arrow everywhere else.
        if input.consume_key(Modifiers::NONE, Key::Tab) {
            match current {
                Some(Item::Chip { chip, .. }) => ask(actions, Asked::Chip(chip.clone())),
                _ => view.highlighted = (view.highlighted + 1).min(last),
            }
        }
        if input.consume_key(Modifiers::NONE, Key::Enter)
            && let Some(item) = current
        {
            ask(actions, choose(item));
        }
        let palette = &state.palette;
        if palette.text().is_empty()
            && !palette.chips().is_empty()
            && input.consume_key(Modifiers::NONE, Key::Backspace)
        {
            ask(actions, Asked::Unchip(palette.chips().len() - 1));
        }
    });
    // `Ctrl+C` and not a bare `c`, which the field types; a selection in the field keeps its own.
    let selecting = egui::TextEdit::load_state(ctx, field_id())
        .and_then(|field| field.cursor.char_range())
        .is_some_and(|range| !range.is_empty());
    if let Some(Item::Note(note)) = current
        && note.kind != NoteKind::Drawing
        && !selecting
        && ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::C))
    {
        ask(actions, Asked::Copy(note.id.clone()));
    }
}

fn panel(
    ui: &mut Ui,
    state: &AppState,
    (groups, items): (&[Group], &[&Item]),
    view: &mut PaletteView,
    actions: &mut Actions,
) {
    let language = state.language;
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.92));
    ui.spacing_mut().item_spacing.y = 8.0;
    field(ui, state, actions);
    let tallest = ui.ctx().content_rect().height() * 0.52;
    egui::ScrollArea::vertical()
        .id_salt("palette-results")
        .max_height(tallest)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            if let Some(empty) = options::empty(state, groups) {
                let words = empty.text(language);
                let said = ui.add(egui::Label::new(
                    egui::RichText::new(&words)
                        .size(12.5)
                        .color(palette(ui.ctx()).text_2),
                ));
                widgets::address(ui, &said, "palette-empty");
            }
            let mut at = 0;
            for group in groups {
                if let Some(heading) = group.heading {
                    heading_row(ui, heading, group.count, language);
                }
                for item in &group.items {
                    let on = at == view.highlighted;
                    let (chosen, hovered, copy) = row(ui, state, item, on);
                    if hovered && !on {
                        view.highlighted = at;
                    }
                    if chosen {
                        ask(actions, choose(item));
                    }
                    if copy && let Item::Note(note) = item {
                        ask(actions, Asked::Copy(note.id.clone()));
                    }
                    at += 1;
                }
            }
            if state.palette.recent() {
                prefixes(ui, language);
            }
        });
    hints(ui, state, items.get(view.highlighted).copied());
}

/// The chips, then the field; the focus is kept there.
fn field(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let frame = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.amber_edge))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 4));
    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (at, chip) in state.palette.chips().iter().enumerate() {
                if chip_shown(ui, chip, language) {
                    ask(actions, Asked::Unchip(at));
                }
            }
            let mut text = state.palette.text().to_owned();
            let hint = tr(if state.palette.chips().is_empty() {
                keys::palette::PLACEHOLDER
            } else {
                keys::palette::PLACEHOLDER_NARROWED
            });
            let typed = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .id(field_id())
                    .frame(egui::Frame::NONE)
                    .hint_text(hint)
                    .font(FontId::new(13.5, FontFamily::Proportional))
                    .text_color(p.text_0)
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(140.0, 32.0)),
            );
            let label = tr(keys::palette::SEARCH_LABEL);
            ui.ctx()
                .accesskit_node_builder(typed.id, |node| node.set_label(label.clone()));
            widgets::address(ui, &typed, "palette-input");
            if !state.fill.is_open() && !typed.has_focus() {
                typed.request_focus();
            }
            if typed.changed() {
                ask(actions, Asked::Type(text));
            }
        });
    });
}

fn chip_label(chip: &Chip, language: i18n::Language) -> String {
    match chip {
        Chip::Scope(scope) => format!("@{}", i18n::translate(language, scope_key(*scope), &[])),
        Chip::Space { name, .. } => format!("@{name}"),
        Chip::Tag(tag) => format!("#{tag}"),
    }
}

fn scope_key(scope: Scope) -> i18n::Key {
    match scope {
        Scope::Notes => keys::palette::scopes::NOTES,
        Scope::Spaces => keys::palette::scopes::SPACES,
    }
}

/// A chip in the field, in the hue its kind is drawn in elsewhere; `true` when its × is clicked.
fn chip_shown(ui: &mut Ui, chip: &Chip, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let hue = match chip {
        Chip::Scope(_) => p.teal,
        Chip::Space { .. } => p.blue,
        Chip::Tag(_) => p.pink,
    };
    let label = chip_label(chip, language);
    let words = widgets::text(ui, &label, 12.0, 600.0, FontFamily::Proportional, hue);
    let shown = egui::Frame::new()
        .fill(p.bg_3)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin {
            left: 8,
            right: 2,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let (rect, _) = ui.allocate_exact_size(vec2(words.size().x, 24.0), Sense::hover());
                ui.painter().galley(
                    pos2(rect.left(), rect.center().y - words.size().y / 2.0),
                    words,
                    hue,
                );
                let remove = i18n::translate(
                    language,
                    keys::palette::REMOVE_CHIP,
                    &[("chip", label.as_str().into())],
                );
                IconButton::new(Icon::X, &remove, "palette-chip-remove")
                    .show(ui)
                    .clicked()
            })
            .inner
        });
    widgets::address(ui, &shown.response, "palette-chip");
    shown.inner
}

fn heading_row(ui: &mut Ui, heading: i18n::Key, count: Option<usize>, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, heading, &[]).to_uppercase();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        let galley = widgets::text(ui, &words, 10.5, 600.0, FontFamily::Proportional, p.text_2);
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
        widgets::address(ui, &response, "palette-group");
        ui.painter().galley(rect.min, galley, p.text_2);
        if let Some(count) = count {
            let counted = widgets::text(
                ui,
                &count.to_string(),
                10.5,
                400.0,
                FontFamily::Monospace,
                p.text_2,
            );
            let (rect, _) = ui.allocate_exact_size(counted.size(), Sense::hover());
            ui.painter().galley(rect.min, counted, p.text_2);
        }
    });
}

/// One option: chosen (clicked), hovered, and its copy clicked.
fn row(ui: &mut Ui, state: &AppState, item: &Item, on: bool) -> (bool, bool, bool) {
    let p = palette(ui.ctx());
    let language = state.language;
    let compact = !matches!(item, Item::Note(_) | Item::Space { .. });
    let height = if compact { 34.0 } else { 44.0 };
    let title = title_of(item, language);
    let author_id = if matches!(item, Item::Create(_)) {
        "palette-create"
    } else {
        "palette-option"
    };
    let width = ui.available_width();
    let corner = CornerRadius::same(8);
    let response = widgets::interact(
        ui,
        vec2(width, height),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::ListBoxOption, true, on, &title),
        corner,
    );
    let rect = response.rect;
    if on {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    let copyable = matches!(item, Item::Note(note) if note.kind != NoteKind::Drawing);
    let copy_room = if copyable { 34.0 } else { 0.0 };
    let inner = Rect::from_min_max(
        rect.min + vec2(10.0, 0.0),
        pos2(rect.right() - 10.0 - copy_room, rect.bottom()),
    );
    let main_left = mark(ui, item, inner, language);
    let side_width = side(ui, state, item, inner, on);
    let main = Rect::from_min_max(
        pos2(main_left, inner.top()),
        pos2(inner.right() - side_width - 8.0, inner.bottom()),
    );
    main_text(ui, state, item, &title, main);
    let mut copied = false;
    if copyable {
        let place = Rect::from_min_size(
            pos2(
                rect.right() - 6.0 - widgets::MIN_TARGET - 4.0,
                rect.center().y - 14.0,
            ),
            vec2(widgets::MIN_TARGET + 4.0, 28.0),
        );
        let mut child = ui.new_child(UiBuilder::new().max_rect(place));
        let label = i18n::translate(
            language,
            keys::palette::COPY_LABEL,
            &[("title", title.as_str().into())],
        );
        copied = IconButton::new(Icon::Copy, &label, "palette-copy")
            .show(&mut child)
            .clicked();
    }
    (response.clicked() && !copied, response.hovered(), copied)
}

fn title_of(item: &Item, language: i18n::Language) -> String {
    match item {
        Item::Note(note) if note.title.trim().is_empty() => {
            i18n::translate(language, keys::notes::UNTITLED, &[])
        }
        Item::Note(note) => note.title.clone(),
        Item::Space { name, .. } => name.clone(),
        Item::Command(command) => i18n::translate(language, command.label(), &[]),
        Item::Chip { chip, .. } => match chip {
            Chip::Scope(_) => chip_label(chip, language),
            Chip::Space { name, .. } => name.clone(),
            Chip::Tag(tag) => tag.clone(),
        },
        Item::Create(query) => i18n::translate(
            language,
            keys::palette::CREATE_NOTE,
            &[("query", query.as_str().into())],
        ),
    }
}

/// The kind's badge or a tile; answers where the words begin.
fn mark(ui: &mut Ui, item: &Item, inner: Rect, language: i18n::Language) -> f32 {
    let p = palette(ui.ctx());
    let tile = Rect::from_min_size(
        pos2(inner.left(), inner.center().y - 13.0),
        vec2(26.0, 26.0),
    );
    let corner = CornerRadius::same(7);
    match item {
        Item::Note(note) => {
            let place =
                Rect::from_min_size(pos2(inner.left(), inner.center().y - 9.0), vec2(60.0, 18.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(place));
            let badge =
                crate::notes::ui::kind_badge(&mut child, note.kind, note.language, language);
            badge.rect.right().max(inner.left() + 26.0) + 10.0
        }
        Item::Create(_) => inner.left(),
        Item::Space { .. }
        | Item::Chip {
            chip: Chip::Space { .. },
            ..
        } => {
            ui.painter().rect_filled(tile, corner, p.bg_3);
            ui.painter().circle_filled(tile.center(), 4.0, p.amber_fill);
            tile.right() + 10.0
        }
        Item::Command(_) | Item::Chip { .. } => {
            let (glyph, hue) = match item {
                Item::Chip {
                    chip: Chip::Tag(_), ..
                } => ("#", p.pink),
                Item::Chip { .. } => ("@", p.teal),
                _ => (">", p.text_1),
            };
            ui.painter().rect_filled(tile, corner, p.bg_3);
            let galley = widgets::text(ui, glyph, 13.0, 600.0, FontFamily::Monospace, hue);
            ui.painter()
                .galley(tile.center() - galley.size() / 2.0, galley, hue);
            tile.right() + 10.0
        }
    }
}

/// The title, the term lit in it, and a note's first line under it.
fn main_text(ui: &Ui, state: &AppState, item: &Item, title: &str, rect: Rect) {
    let p = palette(ui.ctx());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    // The offer to create says what was typed; it is no match.
    let term = if matches!(item, Item::Create(_)) {
        String::new()
    } else {
        term_of(state)
    };
    let job = lit(title, &term, rect.width(), p.amber_text);
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let sub = match item {
        Item::Note(note) => first_line(note),
        Item::Chip { chip, count } => chip_line(chip, *count, state.language),
        _ => String::new(),
    };
    let ink = if matches!(
        item,
        Item::Chip {
            chip: Chip::Scope(_),
            ..
        }
    ) {
        p.teal
    } else {
        p.text_0
    };
    if sub.is_empty() {
        painter.galley(
            pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
        return;
    }
    let mono = matches!(item, Item::Note(note) if note.kind == NoteKind::Snippet);
    let (size, family) = if mono {
        (11.0, FontFamily::Monospace)
    } else {
        (11.5, FontFamily::Proportional)
    };
    let mut line = LayoutJob::simple_singleline(sub, FontId::new(size, family), p.text_2);
    line.wrap.max_width = rect.width();
    line.wrap.max_rows = 1;
    let line = ui.fonts_mut(|fonts| fonts.layout_job(line));
    let total = galley.size().y + 2.0 + line.size().y;
    let top = rect.center().y - total / 2.0;
    let below = top + galley.size().y + 2.0;
    painter.galley(pos2(rect.left(), top), galley, ink);
    painter.galley(pos2(rect.left(), below), line, p.text_2);
}

fn term_of(state: &AppState) -> String {
    match state.palette.mode() {
        Mode::Search(term) | Mode::Commands(term) => term,
        Mode::Scopes(_) | Mode::Tags(_) => String::new(),
    }
}

/// What was typed, lit in the title, when folding keeps the title's length.
fn lit(title: &str, term: &str, width: f32, hit: Color32) -> LayoutJob {
    let format =
        |colour: Color32| TextFormat::simple(FontId::new(13.0, FontFamily::Proportional), colour);
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    let folded = api::notes::fold(title);
    let at = (!term.is_empty() && folded.chars().count() == title.chars().count())
        .then(|| folded.find(&api::notes::fold(term)))
        .flatten();
    // Painted in the row's ink.
    let normal = Color32::PLACEHOLDER;
    let Some(at) = at else {
        job.append(title, 0.0, format(normal));
        return job;
    };
    // The byte offset in the folded title is a character offset in the title: same length.
    let start = folded[..at].chars().count();
    let length = term.chars().count();
    let chars: Vec<char> = title.chars().collect();
    let part = |from: usize, to: usize| {
        chars[from.min(chars.len())..to.min(chars.len())]
            .iter()
            .collect::<String>()
    };
    job.append(&part(0, start), 0.0, format(normal));
    job.append(&part(start, start + length), 0.0, format(hit));
    job.append(&part(start + length, chars.len()), 0.0, format(normal));
    job
}

/// The first line that says something: a list's first item, a note's first line.
fn first_line(note: &engine::notes::model::DisplayNote) -> String {
    if note.kind == NoteKind::Checklist {
        return note
            .items
            .first()
            .map(|item| item.text.clone())
            .unwrap_or_default();
    }
    note.content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned()
}

fn chip_line(chip: &Chip, count: Option<u32>, language: i18n::Language) -> String {
    match chip {
        Chip::Scope(Scope::Notes) => {
            i18n::translate(language, keys::palette::scope_lines::NOTES, &[])
        }
        Chip::Scope(Scope::Spaces) => {
            i18n::translate(language, keys::palette::scope_lines::SPACES, &[])
        }
        Chip::Space { .. } => i18n::translate(language, keys::palette::scope_lines::SPACE, &[]),
        Chip::Tag(_) => i18n::translate(
            language,
            keys::palette::TAG_COUNT,
            &[("count", count.unwrap_or(0).into())],
        ),
    }
}

/// What Enter does when the row is highlighted, otherwise where it lives; answers its width.
fn side(ui: &mut Ui, state: &AppState, item: &Item, inner: Rect, on: bool) -> f32 {
    let p = palette(ui.ctx());
    let language = state.language;
    if let (false, Item::Command(command)) = (on, item) {
        let caps = command.keys();
        if caps.is_empty() {
            return 0.0;
        }
        let width: f32 = caps.iter().map(|word| cap_width(ui, word) + 3.0).sum();
        let place = Rect::from_min_size(
            pos2(inner.right() - width, inner.center().y - 9.0),
            vec2(width, 18.0),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(place)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 3.0;
        for word in caps {
            cap(&mut child, word);
        }
        return width;
    }
    let words = if on {
        let key = match item {
            Item::Note(_) => keys::palette::act::NOTE,
            Item::Space { .. } => keys::palette::act::SPACE,
            Item::Command(_) => keys::palette::act::COMMAND,
            Item::Chip { .. } => keys::palette::act::CHIP,
            Item::Create(_) => keys::palette::act::CREATE,
        };
        i18n::translate(language, key, &[])
    } else {
        match item {
            Item::Note(note) => {
                let space = state
                    .spaces
                    .spaces()
                    .iter()
                    .find(|space| space.id == note.space_id)
                    .map(|space| space.name.clone())
                    .unwrap_or_default();
                let tags: Vec<String> = note
                    .tags
                    .iter()
                    .take(2)
                    .map(|tag| format!("#{tag}"))
                    .collect();
                [vec![space], tags].concat().join("  ")
            }
            _ => String::new(),
        }
    };
    if words.trim().is_empty() {
        return 0.0;
    }
    let ink = if on { p.text_1 } else { p.text_2 };
    let width = widgets::arrowed(ui, None, &words, 11.5, ink).min(inner.width() * 0.4);
    let at = pos2(inner.right() - width, inner.center().y);
    widgets::arrowed(ui, Some(at), &words, 11.5, ink);
    width
}

fn cap_width(ui: &Ui, word: &str) -> f32 {
    widgets::arrowed(ui, None, word, 10.5, Color32::PLACEHOLDER) + 12.0
}

/// A key cap; its glyph painted when no face has it (↑ ↓ ↵ ⌫).
fn cap(ui: &mut Ui, word: &str) {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(cap_width(ui, word), 18.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, word));
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        p.bg_3,
        Stroke::new(1.0, p.line),
        egui::StrokeKind::Inside,
    );
    widgets::arrowed(
        ui,
        Some(pos2(rect.left() + 6.0, rect.center().y)),
        word,
        10.5,
        p.text_1,
    );
}

/// What `@`, `#` and `>` do, under the empty field's recent notes.
fn prefixes(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    ui.add_space(6.0);
    let response = ui
        .horizontal_wrapped(|ui| {
            ui.add_space(10.0);
            ui.spacing_mut().item_spacing.x = 14.0;
            for (code, key) in [
                ("@", keys::palette::prefixes::AT),
                ("#", keys::palette::prefixes::HASH),
                (">", keys::palette::prefixes::COMMAND),
            ] {
                let mut job = LayoutJob::default();
                job.append(
                    code,
                    0.0,
                    TextFormat::simple(FontId::new(11.5, FontFamily::Monospace), p.text_0),
                );
                job.append(
                    &format!(" {}", i18n::translate(language, key, &[])),
                    0.0,
                    TextFormat::simple(FontId::new(11.5, FontFamily::Proportional), p.text_2),
                );
                ui.label(job);
            }
        })
        .response;
    widgets::address(ui, &response, "palette-prefixes");
}

/// The keys of the row highlighted, under the list.
fn hints(ui: &mut Ui, state: &AppState, current: Option<&Item>) {
    let p = palette(ui.ctx());
    let language = state.language;
    let mut hints: Vec<(&[&str], i18n::Key)> = vec![(&["↑", "↓"], keys::palette::keys::NAVIGATE)];
    match current {
        Some(Item::Note(_)) => {
            hints.push((&["↵"], keys::palette::keys::OPEN));
            hints.push((&["Ctrl", "C"], keys::palette::keys::COPY));
        }
        Some(Item::Chip { .. }) => hints.push((&["Tab", "↵"], keys::palette::keys::CHOOSE)),
        Some(Item::Command(_)) => hints.push((&["↵"], keys::palette::keys::RUN)),
        Some(Item::Create(_)) => hints.push((&["↵"], keys::palette::keys::CREATE)),
        Some(Item::Space { .. }) => hints.push((&["↵"], keys::palette::keys::OPEN)),
        None => {}
    }
    if !state.palette.chips().is_empty() && state.palette.text().is_empty() {
        hints.push((&["⌫"], keys::palette::keys::UNCHIP));
    }
    hints.push((&["Esc"], keys::palette::keys::CLOSE));
    let top = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), top, Stroke::new(1.0, p.line_soft));
    ui.add_space(4.0);
    let response = ui
        .horizontal_wrapped(|ui| {
            ui.add_space(10.0);
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (caps, key) in hints {
                for word in caps {
                    cap(ui, word);
                }
                let words = i18n::translate(language, key, &[]);
                let galley =
                    widgets::text(ui, &words, 11.0, 400.0, FontFamily::Proportional, p.text_2);
                let (rect, _) =
                    ui.allocate_exact_size(galley.size() + vec2(10.0, 0.0), Sense::hover());
                ui.painter().galley(rect.min, galley, p.text_2);
            }
        })
        .response;
    widgets::address(ui, &response, "palette-hint");
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::model::NotePatch;

    use crate::notes::bench::{Bench, area, settle, until};
    use crate::testing;

    fn press(harness: &mut Harness<'_, Bench>, modifiers: Modifiers, key: Key) {
        harness.key_press_modifiers(modifiers, key);
        settle(harness);
    }

    /// Two notes, one tagged, and the palette open from its key.
    fn opened() -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        let reset = testing::note(&backend, "Reset the dev database").id.clone();
        testing::note(&backend, "Unrelated note");
        let patch = NotePatch {
            tags: Some(vec!["ops".into()]),
            ..NotePatch::default()
        };
        engine::db::run_here(backend.engine(), move |engine| {
            engine::notes::update_note(engine, reset, patch)
        })
        .unwrap();
        let mut harness = area(backend, profile);
        press(&mut harness, Modifiers::COMMAND | Modifiers::ALT, Key::P);
        assert!(harness.state().state.palette.is_open());
        harness
    }

    fn type_in(harness: &mut Harness<'_, Bench>, words: &str) {
        harness
            .get_by(|node| node.author_id() == Some("palette-input"))
            .type_text(words);
        for _ in 0..20 {
            harness.step();
        }
        settle(harness);
    }

    fn options(harness: &Harness<'_, Bench>) -> Vec<String> {
        harness
            .query_all_by(|node| node.author_id() == Some("palette-option"))
            .map(|node| node.accesskit_node().label().unwrap_or_default())
            .collect()
    }

    #[test]
    fn it_narrows_to_what_is_typed_and_enter_opens_the_note() {
        let mut harness = opened();
        type_in(&mut harness, "reset");
        until(&mut harness, "one note found", |harness| {
            options(harness) == ["Reset the dev database"]
        });

        press(&mut harness, Modifiers::NONE, Key::Enter);

        let state = &harness.state().state;
        assert!(!state.palette.is_open());
        assert_eq!(
            state.editor.opened().map(|open| open.title.as_str()),
            Some("Reset the dev database")
        );
    }

    #[test]
    fn a_query_matching_nothing_offers_a_note_named_after_it() {
        let mut harness = opened();
        type_in(&mut harness, "nothing matches this");
        until(&mut harness, "the offer to create", |harness| {
            harness
                .query_by(|node| node.author_id() == Some("palette-create"))
                .is_some()
        });
        assert!(options(&harness).is_empty());
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("palette-empty"))
                .is_some()
        );

        press(&mut harness, Modifiers::NONE, Key::Enter);
        until(&mut harness, "the note is written", |harness| {
            harness
                .state()
                .state
                .editor
                .opened()
                .is_some_and(|open| open.title == "nothing matches this")
        });
    }

    #[test]
    fn a_tag_becomes_a_chip_and_backspace_takes_it_off() {
        let mut harness = opened();
        type_in(&mut harness, "#op");
        press(&mut harness, Modifiers::NONE, Key::Tab);
        assert_eq!(
            harness.state().state.palette.chips(),
            [crate::state::palette::Chip::Tag("ops".into())]
        );
        until(&mut harness, "the tagged note alone", |harness| {
            options(harness) == ["Reset the dev database"]
        });

        press(&mut harness, Modifiers::NONE, Key::Backspace);
        assert!(harness.state().state.palette.chips().is_empty());
    }

    #[test]
    fn a_command_names_a_folder_in_the_space_shown() {
        let mut harness = opened();
        let spaces = crate::notes::bench::spaces_of_backend(&harness.state().backend);
        let space = spaces[0].id.clone();
        harness
            .state_mut()
            .state
            .spaces
            .choose_space(Some(space.clone()));
        type_in(&mut harness, "> folder");
        press(&mut harness, Modifiers::NONE, Key::Enter);
        harness
            .get_by(|node| node.author_id() == Some("folder-name-input"))
            .type_text("Runbooks");
        settle(&mut harness);
        press(&mut harness, Modifiers::NONE, Key::Enter);

        until(&mut harness, "the folder is made", |harness| {
            harness
                .state()
                .state
                .spaces
                .folders()
                .iter()
                .any(|folder| folder.name == "Runbooks" && folder.space_id == space)
        });
    }

    #[test]
    fn escape_closes_it() {
        let mut harness = opened();
        press(&mut harness, Modifiers::NONE, Key::Escape);
        assert!(!harness.state().state.palette.is_open());
    }
}
