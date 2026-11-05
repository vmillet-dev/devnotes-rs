//! The toolbar over a Note (the reference's `TOOLS`): headings, marks and the link, lists and
//! the quote, the code block and the table; while the caret is in a code block, its language;
//! in a table, its tools. After a tool the text takes the keys back.

use egui::{Key, Sense, Ui, UiBuilder, vec2};
use engine::notes::language::Language as Code;
use i18n::{Language, keys};
use richtext::edit;
use richtext::inlines::Mark;
use richtext::{Block, ListKind, Marks, table, tree};

use super::{LinkForm, RichEditor, Step};
use crate::theme::icons::Icon;
use crate::widgets::{self, Button, ChoiceMenu, IconButton, Kind, TextField};

#[derive(Clone, Copy)]
pub(super) enum Tool {
    Heading(u8),
    Mark(fn() -> Mark),
    Link,
    List(ListKind),
    Quote,
    CodeBlock,
    Table,
    RowAfter,
    ColumnAfter,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
}

/// What a tool shows: an icon, or a few characters of text.
#[derive(Clone, Copy)]
enum Face {
    Icon(Icon),
    Text(&'static str),
}

/// A tool, its address, its face and its name.
type Entry = (Tool, &'static str, Face, i18n::Key);

const GROUPS: [&[Entry]; 4] = {
    use keys::editor::rich as words;
    [
        &[
            (
                Tool::Heading(1),
                "rich-h1",
                Face::Text("H1"),
                words::HEADING1,
            ),
            (
                Tool::Heading(2),
                "rich-h2",
                Face::Text("H2"),
                words::HEADING2,
            ),
            (
                Tool::Heading(3),
                "rich-h3",
                Face::Text("H3"),
                words::HEADING3,
            ),
        ],
        &[
            (
                Tool::Mark(|| Mark::Bold),
                "rich-bold",
                Face::Icon(Icon::Bold),
                words::BOLD,
            ),
            (
                Tool::Mark(|| Mark::Italic),
                "rich-italic",
                Face::Icon(Icon::Italic),
                words::ITALIC,
            ),
            (
                Tool::Mark(|| Mark::Strike),
                "rich-strike",
                Face::Icon(Icon::Strike),
                words::STRIKE,
            ),
            (
                Tool::Mark(|| Mark::Code),
                "rich-code",
                Face::Icon(Icon::Code),
                words::CODE,
            ),
            (Tool::Link, "rich-link", Face::Icon(Icon::Link), words::LINK),
        ],
        &[
            (
                Tool::List(ListKind::Bullet),
                "rich-bullet",
                Face::Icon(Icon::ListBullet),
                words::BULLET_LIST,
            ),
            (
                Tool::List(ListKind::Ordered { start: 1 }),
                "rich-ordered",
                Face::Icon(Icon::ListOrdered),
                words::ORDERED_LIST,
            ),
            (
                Tool::List(ListKind::Task),
                "rich-tasks",
                Face::Icon(Icon::ListTasks),
                words::TASK_LIST,
            ),
            (
                Tool::Quote,
                "rich-quote",
                Face::Icon(Icon::Quote),
                words::QUOTE,
            ),
        ],
        &[
            (
                Tool::CodeBlock,
                "rich-codeBlock",
                Face::Text("{ }"),
                words::CODE_BLOCK,
            ),
            (
                Tool::Table,
                "rich-table",
                Face::Icon(Icon::Table),
                words::TABLE,
            ),
        ],
    ]
};

/// Shown while the caret is in a table.
const TABLE_TOOLS: [(Tool, &str, i18n::Key); 5] = {
    use keys::editor::rich as words;
    [
        (Tool::RowAfter, "rich-rowAfter", words::ADD_ROW),
        (Tool::ColumnAfter, "rich-columnAfter", words::ADD_COLUMN),
        (Tool::DeleteRow, "rich-deleteRow", words::DELETE_ROW),
        (
            Tool::DeleteColumn,
            "rich-deleteColumn",
            words::DELETE_COLUMN,
        ),
        (Tool::DeleteTable, "rich-deleteTable", words::DELETE_TABLE),
    ]
};

pub(super) fn show(ui: &mut Ui, view: &mut RichEditor, language: Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    let mut inner = ui.new_child(
        UiBuilder::new()
            .max_rect(ui.max_rect().shrink2(vec2(24.0, 6.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner.spacing_mut().item_spacing.x = 2.0;
    let group = widgets::region(&mut inner, ui.max_rect(), "rich-toolbar", Sense::hover());
    let label = tr(keys::editor::rich::TOOLBAR);
    group.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Toolbar, true, &label));
    for (at, tools) in GROUPS.iter().enumerate() {
        if at > 0 {
            inner.add_space(10.0);
        }
        for (tool, author_id, face, key) in *tools {
            let name = tr(*key);
            if button(&mut inner, *face, &name, author_id, active(view, *tool)) {
                run(view, *tool);
                view.refocus = view.link.is_none();
            }
        }
    }
    if let Some(fence) = code_language(view) {
        inner.add_space(10.0);
        language_menu(
            &mut inner,
            view,
            &fence,
            &tr(keys::editor::rich::CODE_LANGUAGE),
        );
    }
    if table::cell_of(&view.document, &view.selection.head.path).is_some() {
        inner.add_space(10.0);
        for (tool, author_id, key) in TABLE_TOOLS {
            let clicked = Button::new(&tr(key), author_id)
                .kind(Kind::Quiet)
                .show(&mut inner)
                .clicked();
            if clicked {
                run(view, tool);
                view.refocus = true;
            }
        }
    }
}

fn button(ui: &mut Ui, face: Face, name: &str, author_id: &str, on: bool) -> bool {
    match face {
        Face::Icon(icon) => IconButton::new(icon, name, author_id)
            .pressed(on)
            .show(ui)
            .clicked(),
        Face::Text(text) => {
            let response = Button::new(text, author_id)
                .kind(Kind::Quiet)
                .armed(on)
                .show(ui);
            let name = name.to_owned();
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(name));
            response.clicked()
        }
    }
}

/// The fence of the code block the caret is in.
fn code_language(view: &RichEditor) -> Option<String> {
    match tree::block(&view.document, &view.selection.head.path) {
        Some(Block::CodeBlock { language, .. }) => Some(language.clone()),
        _ => None,
    }
}

/// The code block's language, among the snippets' own; plain text writes a bare fence.
fn language_menu(ui: &mut Ui, view: &mut RichEditor, fence: &str, label: &str) {
    let options: Vec<(Code, &str)> = Code::ALL
        .into_iter()
        .map(|code| (code, crate::notes::language::label(code)))
        .collect();
    let mut chosen = fence.parse::<Code>().unwrap_or(Code::Txt);
    if ChoiceMenu::new(label, "rich-code-language", &options)
        .show(ui, &mut chosen)
        .changed()
    {
        let fence = if chosen == Code::Txt {
            String::new()
        } else {
            chosen.as_str().to_owned()
        };
        view.record(Step::Other);
        edit::set_language(&mut view.document, &view.selection, fence);
        view.refocus = true;
    }
}

/// Whether the tool's mark or block is where the caret is.
fn active(view: &RichEditor, tool: Tool) -> bool {
    let head = &view.selection.head;
    match tool {
        Tool::Heading(level) => matches!(
            tree::block(&view.document, &head.path),
            Some(Block::Heading { level: at, .. }) if *at == level
        ),
        Tool::Mark(mark) => {
            let marks = view.stored.clone().unwrap_or_else(|| view.marks_here());
            match mark() {
                Mark::Bold => marks.bold,
                Mark::Italic => marks.italic,
                Mark::Strike => marks.strike,
                Mark::Code => marks.code,
                Mark::Link(_) => false,
            }
        }
        Tool::Link => view.marks_here().link.is_some(),
        Tool::List(kind) => tree::innermost_item(&view.document, &head.path)
            .and_then(|(list, _, _)| tree::block(&view.document, &list).cloned())
            .is_some_and(|block| {
                matches!(block, Block::List { kind: at, .. }
                    if std::mem::discriminant(&at) == std::mem::discriminant(&kind))
            }),
        Tool::Quote => matches!(
            head.path
                .first()
                .and_then(|top| view.document.blocks.get(*top)),
            Some(Block::Quote(_))
        ),
        Tool::CodeBlock => code_language(view).is_some(),
        _ => false,
    }
}

pub(super) fn run(view: &mut RichEditor, tool: Tool) {
    match tool {
        Tool::Heading(level) => {
            let on = active(view, tool);
            view.record(Step::Other);
            edit::set_heading(&mut view.document, &view.selection, (!on).then_some(level));
        }
        Tool::Mark(mark) => view.toggle(&mark()),
        Tool::Link => {
            let text = edit::selected_text(&view.document, &view.selection);
            view.link = Some(LinkForm {
                text,
                address: view.marks_here().link.unwrap_or_default(),
            });
        }
        Tool::List(kind) => view.change(Step::Other, |document, selection| {
            edit::toggle_list(document, selection, kind)
        }),
        Tool::Quote => view.change(Step::Other, edit::toggle_quote),
        Tool::CodeBlock => view.change(Step::Other, edit::toggle_code_block),
        Tool::Table => view.change(Step::Other, table::insert),
        Tool::RowAfter => view.change(Step::Other, table::add_row_after),
        Tool::ColumnAfter => view.change(Step::Other, table::add_column_after),
        Tool::DeleteRow => view.change(Step::Other, table::delete_row),
        Tool::DeleteColumn => view.change(Step::Other, table::delete_column),
        Tool::DeleteTable => view.change(Step::Other, table::delete),
    }
}

/// The text and the address of a link; applied over the selection, or written at the caret.
pub(super) fn link_form(ui: &mut Ui, view: &mut RichEditor, language: Language) {
    use keys::editor::rich as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let Some(form) = &mut view.link else {
        return;
    };
    let mut inner = ui.new_child(
        UiBuilder::new()
            .max_rect(ui.max_rect().shrink2(vec2(24.0, 4.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner.spacing_mut().item_spacing.x = 8.0;
    let text_name = tr(words::LINK_TEXT);
    let address_name = tr(words::LINK_ADDRESS);
    inner.scope(|ui| {
        ui.set_width(200.0);
        TextField::new(&mut form.text, &text_name, "rich-link-text")
            .compact()
            .show(ui);
    });
    let address = TextField::new(&mut form.address, &address_name, "rich-link-address")
        .compact()
        .hint("https://")
        .show(&mut inner);
    let entered = address.lost_focus() && inner.input(|input| input.key_pressed(Key::Enter));
    let applied = Button::new(&tr(words::LINK_APPLY), "rich-link-apply")
        .kind(Kind::Filled)
        .show(&mut inner)
        .clicked();
    if !(applied || entered) {
        return;
    }
    let Some(LinkForm { text, address }) = view.link.take() else {
        return;
    };
    view.refocus = true;
    let href = Some(address.trim().to_owned()).filter(|href| !href.is_empty());
    if view.selection.is_empty() && !text.is_empty() {
        let marks = Marks {
            link: href,
            ..Marks::default()
        };
        view.change(Step::Other, |document, selection| {
            edit::insert_text(document, selection, &text, &marks)
        });
    } else {
        view.record(Step::Other);
        edit::set_mark(
            &mut view.document,
            &view.selection,
            &Mark::Link(href.clone()),
            href.is_some(),
        );
    }
}
