//! A note changing kind: what its text becomes, decided here and nowhere else.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::checklist::{self, ChecklistItem};
use super::kind::NoteKind;
use super::language::{self, Language};
use super::model::{DisplayNote, Note};

/// What a note holds that a change of kind rewrites: the answer a conversion gives, so its undo
/// puts back exactly that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteBody {
    pub kind: NoteKind,
    pub language: Language,
    pub content: String,
    pub items: Vec<ChecklistItem>,
}

impl NoteBody {
    pub fn of(note: &Note) -> Self {
        Self {
            kind: note.kind,
            language: note.language,
            content: note.content.clone(),
            items: note.items.clone(),
        }
    }
}

/// The note's body once it is of `into`. A snippet and a Note keep the text as it is; a todo
/// list gives its items as a task list to a Note and as lines to a snippet, and takes one item
/// per line that is not blank. Only a snippet reads its text for a language.
pub fn convert(note: &Note, into: NoteKind) -> NoteBody {
    let content = match (note.kind, into) {
        (_, NoteKind::Checklist) => String::new(),
        (NoteKind::Checklist, NoteKind::Note) => checklist::to_markdown(&note.items),
        (NoteKind::Checklist, _) => note
            .items
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => note.content.clone(),
    };
    let items = match (note.kind, into) {
        (NoteKind::Checklist, NoteKind::Checklist) => note.items.clone(),
        (_, NoteKind::Checklist) => items_of(&note.content),
        _ => Vec::new(),
    };
    let language = if into.has_language() {
        language::from_content(&content)
    } else {
        Language::default()
    };

    NoteBody {
        kind: into,
        language,
        content,
        items,
    }
}

/// One item per line that is not blank: a list's marker dropped, a task's box read as done.
pub fn items_of(text: &str) -> Vec<ChecklistItem> {
    let items: Vec<ChecklistItem> = text
        .lines()
        .map(|line| {
            let line = without_marker(line.trim());
            let (done, text) = without_box(line);
            ChecklistItem {
                text: text.to_string(),
                done,
            }
        })
        .collect();

    checklist::normalize_items(&items)
}

fn without_marker(line: &str) -> &str {
    if let Some(rest) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        return rest.trim_start();
    }

    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0
        && let Some(rest) = line[digits..]
            .strip_prefix(". ")
            .or_else(|| line[digits..].strip_prefix(") "))
    {
        return rest.trim_start();
    }

    line
}

fn without_box(line: &str) -> (bool, &str) {
    for (mark, done) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
        if let Some(rest) = line.strip_prefix(mark) {
            return (done, rest.trim_start());
        }
    }

    (false, line)
}

/// What a conversion answers: the note as it now is, and the body it had, which its undo puts
/// back.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteConversion {
    pub note: DisplayNote,
    pub previous: NoteBody,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;

    use super::*;
    use crate::notes::model::NoteLifecycle;
    use crate::notes::priority::Priority;

    fn note_of(kind: NoteKind, content: &str) -> Note {
        Note {
            id: "note".to_string(),
            space_id: "space".to_string(),
            folder_id: None,
            title: String::new(),
            language: Language::default(),
            content: content.to_string(),
            source: String::new(),
            tags: Vec::new(),
            pinned: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            lifecycle: NoteLifecycle::Permanent,
            kind,
            priority: Priority::None,
            items: Vec::new(),
            placeholder_values: BTreeMap::new(),
        }
    }

    fn item(text: &str, done: bool) -> ChecklistItem {
        ChecklistItem {
            text: text.to_string(),
            done,
        }
    }

    fn checklist(items: &[ChecklistItem]) -> Note {
        let mut note = note_of(NoteKind::Checklist, "");
        note.items = items.to_vec();
        note
    }

    #[test]
    fn a_snippet_and_a_note_keep_their_text_as_it_is() {
        let code = "fn main() {\n    println!(\"hi\");\n}";

        let as_note = convert(&note_of(NoteKind::Snippet, code), NoteKind::Note);
        let as_snippet = convert(&note_of(NoteKind::Note, code), NoteKind::Snippet);

        assert_eq!(as_note.content, code);
        assert_eq!(as_note.language, Language::default());
        assert_eq!(as_snippet.content, code);
        assert_eq!(as_snippet.language, language::from_content(code));
    }

    #[test]
    fn a_todo_list_becomes_a_task_list_in_a_note_and_lines_in_a_snippet() {
        let list = checklist(&[item("Freeze", true), item("Deploy", false)]);

        let as_note = convert(&list, NoteKind::Note);
        let as_snippet = convert(&list, NoteKind::Snippet);

        assert_eq!(as_note.content, "- [x] Freeze\n- [ ] Deploy");
        assert!(as_note.items.is_empty());
        assert_eq!(as_snippet.content, "Freeze\nDeploy");
    }

    #[test]
    fn a_text_becomes_one_item_per_line_its_markers_dropped_and_its_boxes_read() {
        let text = "Release\n\n- Freeze the merges\n* [x] Back the database up\n+ [ ] Ship\n12. Watch\n3) Tell\n  [X] Close   ";

        let list = convert(&note_of(NoteKind::Note, text), NoteKind::Checklist);

        assert_eq!(list.content, "");
        assert_eq!(list.language, Language::default());
        assert_eq!(
            list.items,
            [
                item("Release", false),
                item("Freeze the merges", false),
                item("Back the database up", true),
                item("Ship", false),
                item("Watch", false),
                item("Tell", false),
                item("Close", true),
            ]
        );
    }

    #[test]
    fn a_number_alone_or_a_word_after_a_dash_is_text_not_a_marker() {
        assert_eq!(
            items_of("2026 budget\n-dash\n1.5 litres"),
            [
                item("2026 budget", false),
                item("-dash", false),
                item("1.5 litres", false)
            ]
        );
    }

    #[test]
    fn a_task_list_goes_round_a_note_and_back_unchanged() {
        let items = [item("One", true), item("Two", false)];

        let as_note = convert(&checklist(&items), NoteKind::Note);
        let back = convert(
            &note_of(NoteKind::Note, &as_note.content),
            NoteKind::Checklist,
        );

        assert_eq!(back.items, items);
    }
}
