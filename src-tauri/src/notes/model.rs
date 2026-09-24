//! ⚠️ `rename_all` and `tag = "kind"` are load-bearing: without them serde emits
//! `space_id` and `{"Expires":{…}}`, which the front end cannot read back.

use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::checklist::{self, ChecklistItem, NoteKind};
use super::language::{self, Language};
use super::placeholder::{self, Placeholder};
use super::view::SearchHit;
use crate::folders::model::NoteFolder;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub space_id: String,
    /// ⚠️ The folder travels with an export; where its zone sits on the board does not,
    /// and must never join this model — `transfer::Bundle` deserializes `Note` itself.
    #[serde(default)]
    #[specta(optional)]
    pub folder_id: Option<String>,
    pub title: String,
    pub language: Language,
    pub content: String,
    /// Breadcrumb, e.g. "API Gateway / Auth". Can be empty.
    pub source: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lifecycle: NoteLifecycle,
    /// ⚠️ `default`: `transfer::Bundle` deserializes `Note` itself, and a required key
    /// would make every export file written before todo lists unreadable.
    #[serde(default)]
    pub kind: NoteKind,
    /// A checklist has these instead of `content`.
    #[serde(default)]
    pub items: Vec<ChecklistItem>,
    /// Written by `set_placeholder_values` and by nothing else: filling a field is not
    /// editing the note, so it leaves `updated_at` alone.
    #[serde(default)]
    pub placeholder_values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NoteLifecycle {
    Permanent,
    Expires { at: DateTime<Utc> },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteDraft {
    pub space_id: String,
    /// Set by the two places that file a new note: the board and the inside of a folder.
    #[serde(default)]
    #[specta(optional)]
    pub folder_id: Option<String>,
    pub title: String,
    pub language: Language,
    pub content: String,
    pub source: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub lifecycle: NoteLifecycle,
    #[serde(default)]
    pub kind: NoteKind,
    #[serde(default)]
    pub items: Vec<ChecklistItem>,
}

/// One seeded note, and which of the seeded folders it lands in.
///
/// ⚠️ An **index** into the folders the same command creates, not an id: they do not exist
/// until the transaction that writes them is under way. `None` stays loose, which the
/// first launch shows on purpose — "no folder" is a legitimate state.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SampleNote {
    #[serde(default)]
    #[specta(optional)]
    pub folder: Option<u32>,
    pub draft: NoteDraft,
}

/// ⚠️ A field set to `None` stays unchanged, and `#[specta(optional)]` makes the key
/// omissible on the TypeScript side — without it the front sends `null` for what it does
/// not touch, overwriting it.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotePatch {
    #[specta(optional)]
    pub space_id: Option<String>,
    #[specta(optional)]
    pub title: Option<String>,
    #[specta(optional)]
    pub language: Option<Language>,
    #[specta(optional)]
    pub content: Option<String>,
    #[specta(optional)]
    pub source: Option<String>,
    #[specta(optional)]
    pub tags: Option<Vec<String>>,
    #[specta(optional)]
    pub pinned: Option<bool>,
    #[specta(optional)]
    pub lifecycle: Option<NoteLifecycle>,
    #[specta(optional)]
    pub kind: Option<NoteKind>,
    /// Replaces the whole list, like `tags`: a position is all the identity an item has.
    #[specta(optional)]
    pub items: Option<Vec<ChecklistItem>>,
}

impl NoteDraft {
    /// A checklist is exempt from language detection — it has no body to read.
    pub fn into_note(self, id: String, now: DateTime<Utc>) -> Note {
        let language = if self.kind == NoteKind::Checklist {
            Language::default()
        } else {
            language::for_draft(&self)
        };
        let tags = normalize_tags(&self.tags);
        let items = checklist::normalize_items(&self.items);

        Note {
            id,
            space_id: self.space_id,
            folder_id: self.folder_id,
            title: self.title,
            language,
            content: self.content,
            source: self.source,
            tags,
            pinned: self.pinned,
            created_at: now,
            updated_at: now,
            lifecycle: self.lifecycle,
            kind: self.kind,
            items,
            placeholder_values: BTreeMap::new(),
        }
    }
}

impl NotePatch {
    /// ⚠️ Does not check that `space_id` exists — only persistence can, and it does so
    /// before calling.
    pub fn apply(&self, note: &mut Note, now: DateTime<Utc>) {
        // Skipped once the note is — or becomes — a checklist: no body to read.
        let becomes_checklist = self.kind.unwrap_or(note.kind) == NoteKind::Checklist;
        let detected = if becomes_checklist {
            None
        } else {
            language::after_patch(note, self)
        };

        if let Some(space_id) = &self.space_id {
            // ⚠️ A folder belongs to one space, so leaving the note in it would show a
            // chip the space switcher can never reach.
            if *space_id != note.space_id {
                note.folder_id = None;
            }
            note.space_id.clone_from(space_id);
        }
        if let Some(title) = &self.title {
            note.title.clone_from(title);
        }
        if let Some(language) = self.language {
            note.language = language;
        }
        if let Some(content) = &self.content {
            note.content.clone_from(content);
        }
        if let Some(language) = detected {
            note.language = language;
        }
        if let Some(source) = &self.source {
            note.source.clone_from(source);
        }
        if let Some(pinned) = self.pinned {
            note.pinned = pinned;
        }
        if let Some(tags) = &self.tags {
            note.tags = normalize_tags(tags);
        }
        if let Some(lifecycle) = &self.lifecycle {
            note.lifecycle = lifecycle.clone();
        }
        if let Some(kind) = self.kind {
            note.kind = kind;
        }
        if let Some(items) = &self.items {
            note.items = checklist::normalize_items(items);
        }

        note.updated_at = now;
    }
}

const EXPIRING_SOON: TimeDelta = TimeDelta::days(3);

/// The decision, not the rendering: the dated variants carry a date and not a label,
/// so "4 min ago" ages on screen without a round trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NoteFooter {
    Source { value: String },
    Expiry { at: DateTime<Utc> },
    Age { at: DateTime<Utc> },
}

/// `flatten`: the front end has a single note type.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplayNote {
    #[serde(flatten)]
    pub note: Note,
    pub footer: NoteFooter,
    pub expiring_soon: bool,
    /// The list is derived from the text; the values are persisted.
    pub placeholders: Vec<Placeholder>,
    /// Filled in afterwards by whoever holds a connection: `decorate` reads no database.
    pub attachment_count: u32,
    /// Resolved in a pass of its own, like [`Self::attachment_count`]: the front end
    /// never joins a `folder_id` against a list it happens to hold.
    pub folder: Option<NoteFolder>,
    /// ⚠️ What copying yields when that is not the content. Decided here so
    /// `checklist::to_markdown` stays the only place the `- [x] ` syntax exists.
    pub copy_text: Option<String>,
    /// Filled in afterwards by `view::build`; `None` outside a search, and for a note
    /// found by its own title.
    pub search_hit: Option<SearchHit>,
    /// `content` holds only its first lines: a list sends previews, `get_note` the body.
    pub truncated: bool,
}

impl std::ops::Deref for DisplayNote {
    type Target = Note;

    fn deref(&self) -> &Self::Target {
        &self.note
    }
}

/// What a card shows of a body, and so all a list sends of one.
pub const PREVIEW_LINES: usize = 5;

/// ⚠️ Lines alone are no bound: a minified file is one line of a megabyte.
pub const PREVIEW_CHARS: usize = 1_000;

impl DisplayNote {
    /// ⚠️ After everything that reads the body — the fields, the search excerpt — and only
    /// on what goes into a list: a note cut here must never reach the editor.
    pub fn cut_to_preview(&mut self) {
        if let Some(preview) = preview_of(&self.note.content) {
            self.note.content = preview;
            self.truncated = true;
        }
    }
}

/// `None` when the body already fits, which is most snippets.
fn preview_of(body: &str) -> Option<String> {
    let lines_end = body
        .match_indices('\n')
        .nth(PREVIEW_LINES - 1)
        .map_or(body.len(), |(at, _)| at);
    let end = body[..lines_end]
        .char_indices()
        .nth(PREVIEW_CHARS)
        .map_or(lines_end, |(at, _)| at);

    (end < body.len()).then(|| body[..end].to_string())
}

impl Note {
    /// Brought to the rules every draft and patch applies. An import is the one path that
    /// inserts a whole note without going through either.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.tags = normalize_tags(&self.tags);
        self.items = checklist::normalize_items(&self.items);
        self.placeholder_values =
            placeholder::normalize_values(std::mem::take(&mut self.placeholder_values));

        self
    }
}

pub fn decorate(note: Note, now: DateTime<Utc>) -> DisplayNote {
    DisplayNote {
        footer: footer_of(&note),
        expiring_soon: expires_soon(&note, now),
        placeholders: placeholder::parse(&note.content, &note.placeholder_values),
        attachment_count: 0,
        folder: None,
        search_hit: None,
        copy_text: match note.kind {
            NoteKind::Checklist => Some(checklist::to_markdown(&note.items)),
            NoteKind::Snippet => None,
        },
        truncated: false,
        note,
    }
}

pub fn decorate_now(note: Note) -> DisplayNote {
    decorate(note, Utc::now())
}

/// ⚠️ They override the default written in the text but do not touch what was typed on
/// the note: copying one into `value` would freeze the variable the day it changes.
pub fn apply_global_defaults(note: &mut DisplayNote, globals: &BTreeMap<String, String>) {
    for placeholder in &mut note.placeholders {
        if let Some(value) = globals.get(&placeholder.name) {
            placeholder.default_value.clone_from(value);
        }
    }
}

fn footer_of(note: &Note) -> NoteFooter {
    if let NoteLifecycle::Expires { at } = note.lifecycle {
        return NoteFooter::Expiry { at };
    }

    if note.pinned
        && let Some(root) = note
            .source
            .split(" / ")
            .next()
            .filter(|root| !root.is_empty())
    {
        return NoteFooter::Source {
            value: root.to_string(),
        };
    }

    NoteFooter::Age {
        at: note.updated_at,
    }
}

fn expires_soon(note: &Note, now: DateTime<Utc>) -> bool {
    let NoteLifecycle::Expires { at } = note.lifecycle else {
        return false;
    };

    // A duration, not a number of whole days: at 3 days and 1 hour, rounding down would
    // switch the note to alert a day early.
    at.signed_duration_since(now) <= EXPIRING_SOON
}

/// Where a note sat before a batch moved it — the only thing that can put it back.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotePlacement {
    pub note_id: String,
    pub space_id: String,
}

/// One tag on one note. A batch tagging answers pair by pair rather than with a count,
/// so undoing it cannot strip a tag the note already carried.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteTag {
    pub note_id: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TagUsage {
    pub tag: String,
    pub note_count: u32,
}

/// Written across the whole corpus, so its failure is an error: staying silent would
/// rename onto nothing.
pub fn validated_tag(raw: &str) -> Result<String, crate::error::ValidationError> {
    normalize_tag(raw)
        .map(str::to_string)
        .ok_or_else(|| crate::error::ValidationError::new("tag", "a tag must have a readable name"))
}

/// A rename's two halves, brought to the rules every tag write applies: the sources
/// normalised, the target refused when nothing readable is left of it.
pub fn retagging(
    sources: &[String],
    into: &str,
) -> Result<(Vec<String>, String), crate::error::ValidationError> {
    Ok((normalize_tags(sources), validated_tag(into)?))
}

pub fn normalize_tag(tag: &str) -> Option<&str> {
    let cleaned = tag.trim().trim_start_matches('#').trim();

    (!cleaned.is_empty()).then_some(cleaned)
}

/// ⚠️ Both writing and querying go through here, or a typed `#urgent` would not find the
/// stored `urgent`. De-duplication is case-insensitive and keeps the first spelling,
/// like the `COLLATE NOCASE` on the column.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();

    tags.iter()
        .filter_map(|tag| normalize_tag(tag))
        .filter(|cleaned| seen.insert(cleaned.to_lowercase()))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::fixtures::{NOW, at, note as sample};

    #[test]
    fn a_rename_normalises_its_sources_and_refuses_a_blank_target() {
        let (sources, target) =
            retagging(&["#Auth".to_string(), "auth".to_string()], " #identity ").unwrap();

        assert_eq!(sources, ["Auth"]);
        assert_eq!(target, "identity");
        assert_eq!(retagging(&sources, " # ").unwrap_err().field, "tag");
    }

    #[test]
    fn an_imported_note_is_brought_to_the_rules_of_a_write() {
        let imported = Note {
            tags: vec!["#auth".to_string(), "Auth".to_string(), " ".to_string()],
            placeholder_values: BTreeMap::from([
                ("host".to_string(), "db".to_string()),
                ("not a field".to_string(), "x".to_string()),
            ]),
            ..sample()
        }
        .normalized();

        assert_eq!(imported.tags, ["auth"]);
        assert_eq!(imported.placeholder_values.len(), 1);
    }

    fn normalized(tags: &[&str]) -> Vec<String> {
        normalize_tags(&tags.iter().copied().map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn padding_and_blanks_are_dropped() {
        assert_eq!(
            normalized(&["  urgent ", "", "   ", "later"]),
            ["urgent", "later"]
        );
    }

    #[test]
    fn a_duplicate_keeps_its_first_spelling() {
        assert_eq!(normalized(&["Urgent", "urgent", "URGENT"]), ["Urgent"]);
    }

    #[test]
    fn only_leading_hashes_are_stripped() {
        assert_eq!(normalized(&["##c++", "a#b"]), ["c++", "a#b"]);
    }

    #[test]
    fn a_tag_reduced_to_nothing_is_dropped_rather_than_stored_empty() {
        assert!(normalized(&[" # ", "#"]).is_empty());
    }

    #[test]
    fn tag_case_folding_reaches_beyond_ascii() {
        assert_eq!(normalized(&["Étape", "étape"]), ["Étape"]);
    }

    fn now() -> DateTime<Utc> {
        at(NOW)
    }

    fn expiring(at_iso: &str) -> Note {
        Note {
            lifecycle: NoteLifecycle::Expires { at: at(at_iso) },
            ..sample()
        }
    }

    fn draft(language: Language, content: &str) -> NoteDraft {
        NoteDraft {
            space_id: "s-1".to_string(),
            folder_id: None,
            title: "Title".to_string(),
            language,
            content: content.to_string(),
            source: "API Gateway".to_string(),
            tags: vec!["  #Urgent ".to_string(), "urgent".to_string()],
            pinned: true,
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::Snippet,
            items: Vec::new(),
        }
    }

    #[test]
    fn a_draft_becomes_a_note_carrying_the_id_and_the_instant_it_was_given() {
        let note = draft(Language::Md, "some prose").into_note("n-7".to_string(), now());

        assert_eq!(note.id, "n-7");
        assert_eq!(note.created_at, now());
        assert_eq!(note.updated_at, now());
    }

    #[test]
    fn turning_a_draft_into_a_note_detects_the_language_and_normalizes_the_tags() {
        let note = draft(Language::Txt, "{\"a\": 1}").into_note("n-7".to_string(), now());

        assert_eq!(note.language, Language::Json);
        assert_eq!(note.tags, ["Urgent"]);
    }

    #[test]
    fn turning_a_draft_into_a_note_leaves_everything_else_alone() {
        let note = draft(Language::Md, "SELECT 1").into_note("n-7".to_string(), now());

        assert_eq!(note.language, Language::Md);
        assert_eq!(note.content, "SELECT 1");
        assert_eq!(note.space_id, "s-1");
        assert_eq!(note.source, "API Gateway");
        assert!(note.pinned);
    }

    #[test]
    fn a_patch_only_touches_the_fields_it_carries() {
        let mut note = sample();
        let patch = NotePatch {
            title: Some("New".to_string()),
            ..NotePatch::default()
        };

        patch.apply(&mut note, at("2026-07-25T10:00:00.000Z"));

        assert_eq!(note.title, "New");
        assert_eq!(note.content, "Content");
        assert_eq!(note.tags, ["auth"]);
        assert_eq!(note.updated_at, at("2026-07-25T10:00:00.000Z"));
        assert_eq!(note.created_at, now());
    }

    #[test]
    fn a_patch_normalizes_the_tags_it_replaces() {
        let mut note = sample();
        let patch = NotePatch {
            tags: Some(vec![
                "  #Ops ".to_string(),
                "OPS".to_string(),
                " ".to_string(),
            ]),
            ..NotePatch::default()
        };

        patch.apply(&mut note, now());

        assert_eq!(note.tags, ["Ops"]);
    }

    #[test]
    fn a_patch_filling_an_empty_note_detects_its_language() {
        let mut note = Note {
            language: Language::Txt,
            content: String::new(),
            ..sample()
        };
        let patch = NotePatch {
            content: Some("SELECT 1".to_string()),
            ..NotePatch::default()
        };

        patch.apply(&mut note, now());

        assert_eq!(note.language, Language::Sql);
    }

    #[test]
    fn an_ordinary_note_shows_the_age_of_its_last_change() {
        let footer = footer_of(&sample());

        assert_eq!(footer, NoteFooter::Age { at: at(NOW) });
    }

    #[test]
    fn a_checklist_is_not_given_a_guessed_language() {
        let draft = NoteDraft {
            kind: NoteKind::Checklist,
            content: "{ \"a\": 1 }".to_string(),
            language: Language::Txt,
            ..draft(Language::Txt, String::new().as_str())
        };

        let note = draft.into_note("n-2".to_string(), now());

        assert_eq!(note.language, Language::Txt);
    }

    #[test]
    fn turning_a_note_into_a_checklist_does_not_trigger_a_detection() {
        let mut note = Note {
            language: Language::Txt,
            content: String::new(),
            ..sample()
        };
        let patch = NotePatch {
            kind: Some(NoteKind::Checklist),
            content: Some("{ \"a\": 1 }".to_string()),
            ..NotePatch::default()
        };

        patch.apply(&mut note, now());

        assert_eq!(note.language, Language::Txt);
    }

    #[test]
    fn a_patch_replaces_the_whole_item_list_and_normalizes_it() {
        let mut note = Note {
            items: vec![ChecklistItem {
                text: "Old".to_string(),
                done: true,
            }],
            ..sample()
        };
        let patch = NotePatch {
            items: Some(vec![
                ChecklistItem {
                    text: "  Ship it ".to_string(),
                    done: false,
                },
                ChecklistItem {
                    text: "   ".to_string(),
                    done: false,
                },
            ]),
            ..NotePatch::default()
        };

        patch.apply(&mut note, now());

        assert_eq!(
            note.items,
            [ChecklistItem {
                text: "Ship it".to_string(),
                done: false
            }]
        );
    }

    #[test]
    fn a_patch_that_says_nothing_about_the_items_leaves_them_alone() {
        let mut note = Note {
            items: vec![ChecklistItem {
                text: "Ship it".to_string(),
                done: true,
            }],
            ..sample()
        };

        NotePatch {
            title: Some("T".to_string()),
            ..NotePatch::default()
        }
        .apply(&mut note, now());

        assert_eq!(note.items.len(), 1);
    }

    #[test]
    fn a_pinned_note_shows_the_first_segment_of_its_context() {
        let note = Note {
            pinned: true,
            source: "API Gateway / Auth / Tokens".to_string(),
            ..sample()
        };

        assert_eq!(
            footer_of(&note),
            NoteFooter::Source {
                value: "API Gateway".to_string()
            }
        );
    }

    #[test]
    fn a_pinned_note_without_context_falls_back_to_its_age() {
        let note = Note {
            pinned: true,
            source: String::new(),
            ..sample()
        };

        assert!(matches!(footer_of(&note), NoteFooter::Age { .. }));
    }

    #[test]
    fn an_expiring_note_shows_its_deadline_even_when_pinned() {
        let note = Note {
            pinned: true,
            source: "API Gateway".to_string(),
            ..expiring("2026-08-01T00:00:00.000Z")
        };

        assert!(matches!(footer_of(&note), NoteFooter::Expiry { .. }));
    }

    #[test]
    fn a_permanent_note_never_counts_as_expiring_soon() {
        assert!(!expires_soon(&sample(), now()));
    }

    #[test]
    fn the_threshold_is_measured_in_fractions_of_a_day() {
        assert!(!expires_soon(&expiring("2026-07-28T10:00:00.000Z"), now()));
        assert!(expires_soon(&expiring("2026-07-28T08:00:00.000Z"), now()));
    }

    #[test]
    fn an_already_expired_note_counts_as_expiring_soon() {
        assert!(expires_soon(&expiring("2026-07-01T00:00:00.000Z"), now()));
    }

    fn previewed(content: &str) -> DisplayNote {
        let mut note = decorate(
            Note {
                content: content.to_string(),
                ..sample()
            },
            now(),
        );
        note.cut_to_preview();

        note
    }

    #[test]
    fn a_body_that_fits_is_sent_whole_and_not_marked() {
        for body in ["", "one", "1\n2\n3\n4\n5"] {
            let note = previewed(body);

            assert_eq!(note.content, body);
            assert!(!note.truncated);
        }
    }

    #[test]
    fn a_long_body_keeps_the_lines_a_card_shows() {
        let note = previewed("1\n2\n3\n4\n5\n6\n7");

        assert_eq!(note.content, "1\n2\n3\n4\n5");
        assert!(note.truncated);
    }

    #[test]
    fn one_long_line_is_cut_on_a_character_boundary() {
        let note = previewed(&"é".repeat(PREVIEW_CHARS * 3));

        assert_eq!(note.content.chars().count(), PREVIEW_CHARS);
        assert!(note.truncated);
    }

    /// ⚠️ Read before the cut: a list cannot say a snippet has fields past its first lines.
    #[test]
    fn the_fields_are_found_in_the_whole_body() {
        let note = previewed("1\n2\n3\n4\n5\npsql -h {{host}}");

        assert_eq!(note.placeholders[0].name, "host");
    }
}
