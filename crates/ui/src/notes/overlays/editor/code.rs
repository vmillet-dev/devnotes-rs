//! A snippet's code (the reference's `code-editor`): `EditorView` over the body, coloured by its
//! language's grammar, each `{{field}}` marked where the core finds it, the find-and-replace bar
//! over it. Tab, the comment token and the indent unit follow the language.

use egui::{Color32, Ui};
use engine::notes::language::Language;
use i18n::{Language as Locale, keys};
use text::highlight::Token;

use super::search_bar::{self, SearchBar};
use crate::api;
use crate::notes::language;
use crate::services::highlight::{self, Colouring};
use crate::theme::palette;
use crate::widgets::editor::{Decoration, Editor, EditorView, Mark};

/// The code being edited, and what is drawn over it.
pub(super) struct CodeView {
    pub(super) editor: Editor,
    language: Language,
    colouring: Option<Colouring>,
    fields: Vec<Decoration>,
    pub(super) search: SearchBar,
    /// Lit from beside: what the explorer has selected.
    pub(super) lit: std::ops::Range<usize>,
    /// Marked in the gutter: what a format changed, until the next keystroke.
    pub(super) marked: Vec<usize>,
}

impl CodeView {
    pub(super) fn of(text: &str, language: Language) -> Self {
        Self {
            editor: Editor::new(text),
            language,
            colouring: None,
            fields: fields(text),
            search: SearchBar::default(),
            lit: 0..0,
            marked: Vec::new(),
        }
    }

    pub(super) fn text(&self) -> String {
        self.editor.text()
    }

    /// The text changed from `line` on.
    pub(super) fn edited(&mut self, line: usize) {
        self.marked.clear();
        if let Some(colouring) = &mut self.colouring {
            colouring.edited(line);
        }
        self.fields = fields(&self.editor.text());
    }
}

/// The fields the core finds, decorated.
fn fields(text: &str) -> Vec<Decoration> {
    api::notes::field_ranges(text)
        .into_iter()
        .map(|range| Decoration {
            range,
            mark: Mark::Field,
        })
        .collect()
}

/// What a frame of the code did.
pub(super) struct Shown {
    pub(super) changed: bool,
    pub(super) left: bool,
}

pub(super) fn show(ui: &mut Ui, code: &mut CodeView, language: Language, locale: Locale) -> Shown {
    if code.language != language || (code.colouring.is_none() && text::highlight::is_ready()) {
        code.language = language;
        code.colouring =
            language::syntax(language).and_then(|syntax| Colouring::new(ui.ctx(), syntax));
    }
    let mut changed = false;
    if let Some(replacing) = search_bar::shortcut(ui) {
        code.search.open(&code.editor, replacing);
    }
    if code.search.open
        && let Some(line) = search_bar::show(ui, &mut code.search, &mut code.editor, locale)
    {
        code.edited(line);
        changed = true;
    }
    let mut decorations = code.fields.clone();
    if code.search.open {
        let selected = code.editor.buffer.selection().range();
        decorations.extend(
            code.editor
                .buffer
                .find(&code.search.query, text::COUNT_CAP)
                .into_iter()
                .map(|range| Decoration {
                    mark: if range == selected {
                        Mark::CurrentMatch
                    } else {
                        Mark::Match
                    },
                    range,
                }),
        );
    }
    let p = palette(ui.ctx());
    let colouring = code.colouring.as_mut();
    let rope = code.editor.buffer.rope().clone();
    let window = colouring.map(|colouring| {
        colouring.window(&rope, code.editor.visible_lines());
        &*colouring
    });
    let colours = |line: usize, _: &str| -> Vec<(std::ops::Range<usize>, Color32)> {
        window
            .and_then(|colouring| colouring.line(line))
            .map(|tokens| {
                tokens
                    .iter()
                    .map(|(range, token)| (range.clone(), tint(p, *token)))
                    .collect()
            })
            .unwrap_or_default()
    };
    let label = i18n::translate(locale, keys::editor::CONTENT_LABEL, &[]);
    let hint = i18n::translate(locale, keys::editor::EMPTY_BODY, &[]);
    let output = EditorView::new(&mut code.editor, &label, "editor-body")
        .hint(&hint)
        .language(locale)
        .indent_unit(language::indent_unit(language))
        .comment(language::line_comment(language))
        .decorations(&decorations)
        .lit_lines(code.lit.clone())
        .marked_lines(&code.marked)
        .colours(&colours)
        .show(ui);
    if let Some(line) = output.changed {
        code.edited(line);
        changed = true;
    }
    Shown {
        changed,
        left: output.response.lost_focus(),
    }
}

fn tint(p: &crate::theme::tokens::Palette, token: Token) -> Color32 {
    highlight::colour(p, token).0
}

#[cfg(test)]
mod tests;
