use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, FixedOffset, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use unicode_normalization::char::{decompose_canonical, is_combining_mark};

use super::language::Language;
use super::model::{self, DisplayNote, Note, NoteLifecycle};
use crate::count::saturating_u32;
use crate::folders::model::NoteFolder;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotesQuery {
    /// `None` = every space: a choice, not an absence of one.
    pub space_id: Option<String>,
    /// `None` = every folder, filed or not. Narrowing to "unfiled" is not offered: the
    /// absence of a chip already reads, and a filter for it would be a fourth way to say
    /// the same thing.
    #[serde(default)]
    #[specta(optional)]
    pub folder_id: Option<String>,
    pub search: String,
    pub filter: NoteFilter,
    /// A note passes if it carries at least one of these tags.
    pub tags: Vec<String>,
    pub languages: Vec<Language>,
    pub now: DateTime<Utc>,
    /// ⚠️ `Date#getTimezoneOffset()`, whose sign is the opposite of the offset
    /// (−120 for UTC+2). Sections reason in local days.
    pub tz_offset_minutes: i32,
    /// Their own section when the view is chronological, the head of the list when flat.
    pub pinned_first: bool,
}

/// `Untriaged` = notes with a deadline, those whose fate is not decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NoteFilter {
    All,
    Pinned,
    Untriaged,
}

/// What the search, the quick filter and the two rails ask of a note, normalised once.
///
/// ⚠️ The date view narrows on these in SQL and the board dims on them in Rust. This is the
/// rule both are held to, so the two views cannot disagree on which notes match.
#[derive(Debug, Clone)]
pub struct Criteria {
    needle: String,
    filter: NoteFilter,
    tags: Vec<String>,
    languages: Vec<Language>,
}

impl Criteria {
    pub fn new(search: &str, filter: NoteFilter, tags: &[String], languages: &[Language]) -> Self {
        Self {
            needle: fold(search.trim()),
            filter,
            tags: model::normalize_tags(tags),
            languages: languages.to_vec(),
        }
    }

    /// The search, trimmed and folded; empty when nothing is searched.
    pub fn needle(&self) -> &str {
        &self.needle
    }

    /// Whether the search or a rail narrows the notes. The quick filter is not counted: the
    /// date view keeps its sections under it.
    pub fn narrows(&self) -> bool {
        !self.needle.is_empty() || !self.tags.is_empty() || !self.languages.is_empty()
    }

    pub fn passes(&self, note: &Note) -> bool {
        let filter = match self.filter {
            NoteFilter::All => true,
            NoteFilter::Pinned => note.pinned,
            NoteFilter::Untriaged => matches!(note.lifecycle, NoteLifecycle::Expires { .. }),
        };

        // ASCII only, like the `COLLATE NOCASE` the date view filters with in SQL.
        let tags = self.tags.is_empty()
            || self.tags.iter().any(|wanted| {
                note.tags
                    .iter()
                    .any(|carried| carried.eq_ignore_ascii_case(wanted))
            });

        let languages = self.languages.is_empty() || self.languages.contains(&note.language);
        let search = self.needle.is_empty() || matches_search(note, &self.needle);

        filter && tags && languages && search
    }
}

impl From<&NotesQuery> for Criteria {
    fn from(query: &NotesQuery) -> Self {
        Self::new(&query.search, query.filter, &query.tags, &query.languages)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Facets {
    pub tags: Vec<String>,
    pub languages: Vec<Language>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotesView {
    pub sections: Vec<NoteSection>,
    /// Attached to the space, not the current filter: facets drawn from already
    /// filtered notes would empty the rail on the first selection.
    pub available_tags: Vec<String>,
    pub available_languages: Vec<Language>,
    pub is_filtering: bool,
    /// `u32` and not `usize`: Specta refuses a type JSON cannot render losslessly.
    pub matched: u32,
}

/// ⚠️ No `Title` variant, deliberately: a note found by its own title needs no
/// explanation, and an excerpt would repeat the biggest thing on the card. That case is
/// [`SearchMatch::Title`], which carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SearchField {
    Tag,
    Body,
    Item,
}

/// What made a note match, and where.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub field: SearchField,
    /// The matching line, not the whole body: a card has room for one.
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteSection {
    pub key: NoteSectionKey,
    pub has_expiring_notes: bool,
    pub notes: Vec<DisplayNote>,
    pub show_create_ghost: bool,
}

/// A translation key on the front-end side: no readable label crosses the bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NoteSectionKey {
    Pinned,
    Today,
    Week,
    Older,
    Results,
}

/// Separate from [`build`], which reads no database.
pub fn apply_attachment_counts<S: std::hash::BuildHasher>(
    view: &mut NotesView,
    counts: &HashMap<String, u32, S>,
) {
    for section in &mut view.sections {
        for note in &mut section.notes {
            note.attachment_count = counts.get(&note.id).copied().unwrap_or(0);
        }
    }
}

/// Separate from [`build`] for the same reason as [`apply_attachment_counts`].
pub fn apply_folders<S: std::hash::BuildHasher>(
    view: &mut NotesView,
    folders: &HashMap<String, NoteFolder, S>,
) {
    for section in &mut view.sections {
        for note in &mut section.notes {
            note.folder = note
                .folder_id
                .as_ref()
                .and_then(|id| folders.get(id))
                .cloned();
        }
    }
}

/// Separate from [`build`] for the same reason as [`apply_attachment_counts`].
pub fn apply_global_defaults(view: &mut NotesView, globals: &BTreeMap<String, String>) {
    if globals.is_empty() {
        return;
    }

    for section in &mut view.sections {
        for note in &mut section.notes {
            model::apply_global_defaults(note, globals);
        }
    }
}

pub fn build(mut notes: Vec<Note>, facets: Facets, request: &NotesQuery) -> NotesView {
    let criteria = Criteria::from(request);
    let needle = criteria.needle();
    // Collected while filtering: finding the same match twice is paid for twice.
    let mut hits: HashMap<String, SearchHit> = HashMap::new();
    if !needle.is_empty() {
        notes.retain(|note| match find_match(note, needle) {
            None => false,
            Some(SearchMatch::Title) => true,
            Some(SearchMatch::Elsewhere(hit)) => {
                hits.insert(note.id.clone(), hit);
                true
            }
        });
    }

    // A quick filter restricts a view that stays chronological; a search, a facet or an
    // opened folder switches to a flat list.
    //
    // ⚠️ The folder belongs here and `build_sections` still knows nothing about it: the
    // inside of a folder is already sorted by the fact of being there, so dating it again
    // would be classifying twice. The date view's own sections are untouched — nothing
    // sends a `folder_id` unless a folder has actually been opened.
    let inside_folder = request.folder_id.is_some();
    let is_filtering = criteria.narrows() || inside_folder;

    // ⚠️ The inside of a folder is a place to create in — a note made there arrives filed
    // — where a result list is a list of what already matched. So the ghost rides on the
    // flat view only when the folder is the *whole* reason it is flat.
    let flat = is_filtering.then_some(!criteria.narrows());
    let matched = saturating_u32(notes.len());

    let offset = offset_from_minutes(request.tz_offset_minutes);

    let mut view = NotesView {
        sections: build_sections(notes, flat, request.pinned_first, request.now, offset),
        available_tags: facets.tags,
        available_languages: facets.languages,
        is_filtering,
        matched,
    };

    // A pass of its own, like the attachment counter: the notes become `DisplayNote`s
    // inside `build_sections`, and threading a second value through would cost every
    // section-splitting test an argument.
    apply_search_hits(&mut view, &mut hits);
    view
}

fn apply_search_hits(view: &mut NotesView, hits: &mut HashMap<String, SearchHit>) {
    if hits.is_empty() {
        return;
    }

    for section in &mut view.sections {
        for note in &mut section.notes {
            note.search_hit = hits.remove(&note.id);
        }
    }
}

/// `needle` is expected to have been through [`fold`] and trimmed.
///
/// ⚠️ Folded in Rust and not in SQL: without ICU, SQLite's `LOWER()` only handles ASCII,
/// so `Étape` would not match `étape`. The items count as much as the content — a todo
/// list has no body to be found by.
/// ⚠️ The one place a needle meets a note. The board reuses it rather than growing a
/// second, subtly different match — it dims what does not match instead of dropping it.
pub fn matches_search(note: &Note, needle: &str) -> bool {
    find_match(note, needle).is_some()
}

fn find_match(note: &Note, needle: &str) -> Option<SearchMatch> {
    if contains_folded(&note.title, needle) {
        return Some(SearchMatch::Title);
    }

    if let Some(tag) = note.tags.iter().find(|tag| contains_folded(tag, needle)) {
        return Some(SearchMatch::elsewhere(SearchField::Tag, tag));
    }

    if let Some(line) = note
        .content
        .lines()
        .find(|line| contains_folded(line, needle))
    {
        return Some(SearchMatch::elsewhere(SearchField::Body, line.trim()));
    }

    note.items
        .iter()
        .find(|item| contains_folded(&item.text, needle))
        .map(|item| SearchMatch::elsewhere(SearchField::Item, &item.text))
}

/// A note either fails to match, matches on something the card already shows, or
/// matches on something it does not and can quote.
enum SearchMatch {
    Title,
    Elsewhere(SearchHit),
}

impl SearchMatch {
    fn elsewhere(field: SearchField, text: &str) -> Self {
        Self::Elsewhere(SearchHit {
            field,
            excerpt: clip(text),
        })
    }
}

/// A card shows one line, and a body is free to hold a minified payload on one of them.
const EXCERPT_CHARS: usize = 160;

/// ⚠️ Characters and not bytes: `s[..160]` panics in the middle of a `é`.
fn clip(text: &str) -> String {
    let mut clipped: String = text.chars().take(EXCERPT_CHARS).collect();
    if text.chars().nth(EXCERPT_CHARS).is_some() {
        clipped.push('…');
    }
    clipped
}

/// Lowercase and accent-free, so `etape` finds `Étape`. Both sides go through here,
/// which makes the match symmetric. Only what a canonical decomposition separates is
/// folded: `ø` and `ß` are letters of their own and stay.
///
/// ⚠️ Decomposed one character at a time, not through the `nfd()` iterator over the whole
/// string — measured 5× slower for the same answer, its lookahead buffering earning
/// nothing when every mark is dropped anyway. The ASCII branches are the common case,
/// not a micro-optimisation: code is ASCII end to end, prose between its accents.
pub(crate) fn fold(text: &str) -> String {
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }

    let mut folded = String::with_capacity(text.len());

    for character in text.chars() {
        if character.is_ascii() {
            folded.push(character.to_ascii_lowercase());
        } else {
            decompose_canonical(character, |part| {
                if !is_combining_mark(part) {
                    folded.extend(part.to_lowercase());
                }
            });
        }
    }

    folded
}

/// ⚠️ Do not hand-roll a fold-as-you-compare scan to save the copy: `str::contains` runs
/// Two-Way (O(n+m)) where a window scan is O(n·m), and searching is precisely the case
/// where most notes do not match. Measured at nearly twice the cost.
fn contains_folded(haystack: &str, needle: &str) -> bool {
    fold(haystack).contains(needle)
}

const A_WEEK: TimeDelta = TimeDelta::days(7);

const MAX_TZ_OFFSET_MINUTES: u32 = 14 * 60;

/// ⚠️ The sign flips: JavaScript counts the minutes to add to local time to get UTC
/// (−120 for UTC+2), where chrono expects the offset east. The bound is checked before
/// the multiplication, which would otherwise overflow.
fn offset_from_minutes(tz_offset_minutes: i32) -> FixedOffset {
    let utc = FixedOffset::east_opt(0).expect("UTC is a valid offset");

    if tz_offset_minutes.unsigned_abs() > MAX_TZ_OFFSET_MINUTES {
        return utc;
    }

    FixedOffset::east_opt(-tz_offset_minutes * 60).unwrap_or(utc)
}

fn is_same_local_day(a: &DateTime<FixedOffset>, b: &DateTime<FixedOffset>) -> bool {
    a.year() == b.year() && a.month() == b.month() && a.day() == b.day()
}

fn is_within(date: &DateTime<FixedOffset>, now: &DateTime<FixedOffset>, window: TimeDelta) -> bool {
    let elapsed = now.signed_duration_since(*date);
    elapsed >= TimeDelta::zero() && elapsed <= window
}

fn section(
    key: NoteSectionKey,
    notes: Vec<Note>,
    show_create_ghost: bool,
    now: DateTime<Utc>,
) -> NoteSection {
    let notes: Vec<DisplayNote> = notes
        .into_iter()
        .map(|note| model::decorate(note, now))
        .collect();

    NoteSection {
        has_expiring_notes: notes.iter().any(|note| note.expiring_soon),
        key,
        notes,
        show_create_ghost,
    }
}

/// The partition is stable: at equal pinning, SQL decides.
fn results(
    mut notes: Vec<Note>,
    pinned_first: bool,
    create_ghost: bool,
    now: DateTime<Utc>,
) -> Vec<NoteSection> {
    if pinned_first {
        notes.sort_by_key(|note| !note.pinned);
    }

    vec![section(NoteSectionKey::Results, notes, create_ghost, now)]
}

/// `flat` is `None` for the chronological sections and `Some(ghost)` for the single
/// `Results` list, `ghost` saying whether it is somewhere a note can be created.
fn build_sections(
    notes: Vec<Note>,
    flat: Option<bool>,
    pinned_first: bool,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Vec<NoteSection> {
    let local_now = now.with_timezone(&offset);
    if let Some(create_ghost) = flat {
        return results(notes, pinned_first, create_ghost, now);
    }

    let mut pinned = Vec::new();
    let mut today = Vec::new();
    let mut this_week = Vec::new();
    let mut older = Vec::new();

    for note in notes {
        // Without the hoist the "pinned" section disappears rather than being empty.
        if pinned_first && note.pinned {
            pinned.push(note);
            continue;
        }

        let created = note.created_at.with_timezone(&offset);
        if is_same_local_day(&created, &local_now) {
            today.push(note);
        } else if is_within(&created, &local_now, A_WEEK) {
            this_week.push(note);
        } else {
            older.push(note);
        }
    }

    let mut sections = Vec::new();

    if !pinned.is_empty() {
        sections.push(section(NoteSectionKey::Pinned, pinned, false, now));
    }
    if !today.is_empty() {
        sections.push(section(NoteSectionKey::Today, today, false, now));
    }

    // Always present: it hosts the "paste or create" ghost card.
    sections.push(section(NoteSectionKey::Week, this_week, true, now));

    if !older.is_empty() {
        sections.push(section(NoteSectionKey::Older, older, false, now));
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::checklist::{ChecklistItem, NoteKind};
    use crate::notes::fixtures::{NOW, at, note as sample};

    fn request() -> NotesQuery {
        NotesQuery {
            space_id: None,
            folder_id: None,
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            now: at(NOW),
            tz_offset_minutes: 0,
            pinned_first: true,
        }
    }

    fn note(id: &str, title: &str) -> Note {
        Note {
            id: id.to_string(),
            title: title.to_string(),
            created_at: at("2026-07-25T08:00:00.000Z"),
            ..sample()
        }
    }

    fn keys(view: &NotesView) -> Vec<NoteSectionKey> {
        view.sections.iter().map(|section| section.key).collect()
    }

    #[test]
    fn an_empty_search_keeps_every_note_and_reports_no_filtering() {
        let view = build(
            vec![note("a", "Un"), note("b", "Deux")],
            Facets::default(),
            &request(),
        );

        assert_eq!(view.matched, 2);
        assert!(!view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Today, NoteSectionKey::Week]);
    }

    #[test]
    fn a_search_narrows_the_notes_and_collapses_the_sections() {
        let notes = vec![note("a", "Déploiement"), note("b", "Autre chose")];

        let view = build(
            notes,
            Facets::default(),
            &NotesQuery {
                search: "  DEPLOI  ".to_string(),
                ..request()
            },
        );

        assert_eq!(view.matched, 1);
        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
    }

    #[test]
    fn a_selected_tag_counts_as_filtering_even_with_no_search() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                tags: vec!["urgent".to_string()],
                ..request()
            },
        );

        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
    }

    /// ⚠️ The inside of a folder is already sorted by the fact of being there, so dating
    /// it again would be classifying twice. `build_sections` still knows nothing about a
    /// folder — this is the only place the two meet.
    #[test]
    fn an_opened_folder_is_a_flat_grid_rather_than_dated_sections() {
        let view = build(
            vec![note("a", "Un"), note("b", "Deux")],
            Facets::default(),
            &NotesQuery {
                folder_id: Some("f-1".to_string()),
                ..request()
            },
        );

        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
        assert_eq!(view.matched, 2);
    }

    /// ⚠️ The inside of a folder is a place to create in — a note made there arrives
    /// filed — where the flat view a search produces is a list of what already matched.
    #[test]
    fn an_opened_folder_keeps_the_slot_a_note_is_created_from() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                folder_id: Some("f-1".to_string()),
                ..request()
            },
        );

        assert_eq!(keys(&view), [NoteSectionKey::Results]);
        assert!(view.sections[0].show_create_ghost);
    }

    #[test]
    fn a_search_inside_a_folder_offers_nothing_to_create() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                folder_id: Some("f-1".to_string()),
                search: "Un".to_string(),
                ..request()
            },
        );

        assert_eq!(keys(&view), [NoteSectionKey::Results]);
        assert!(!view.sections[0].show_create_ghost);
    }

    /// A facet narrows a list; it does not name a place.
    #[test]
    fn a_selected_facet_offers_nothing_to_create_either() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                folder_id: Some("f-1".to_string()),
                languages: vec![Language::Json],
                ..request()
            },
        );

        assert!(!view.sections[0].show_create_ghost);
    }

    /// The date view is untouched: nothing sends a folder unless one has been opened.
    #[test]
    fn no_folder_leaves_the_sections_exactly_as_they_were() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery { ..request() },
        );

        assert!(!view.is_filtering);
        assert_ne!(keys(&view), [NoteSectionKey::Results]);
    }

    #[test]
    fn a_tag_that_normalizes_to_nothing_does_not_count_as_filtering() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                tags: vec![" # ".to_string()],
                ..request()
            },
        );

        assert!(!view.is_filtering);
    }

    #[test]
    fn a_fruitless_search_reports_filtering_with_zero_matches() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                search: "no-such-thing".to_string(),
                ..request()
            },
        );

        assert_eq!(view.matched, 0);
        assert!(view.is_filtering);
    }

    #[test]
    fn the_rail_facets_are_passed_through_untouched() {
        let view = build(
            vec![note("a", "Un")],
            Facets {
                tags: vec!["api".to_string(), "auth".to_string()],
                languages: vec![Language::Json, Language::Txt],
            },
            &NotesQuery {
                search: "no-such-thing".to_string(),
                ..request()
            },
        );

        assert_eq!(view.available_tags, ["api", "auth"]);
        assert_eq!(view.available_languages, [Language::Json, Language::Txt]);
    }

    #[test]
    fn a_selected_language_counts_as_filtering_like_a_selected_tag() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                languages: vec![Language::Json],
                ..request()
            },
        );

        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
    }

    mod search {
        use super::*;

        #[test]
        fn the_title_the_tags_and_the_content_are_all_searched() {
            let note = Note {
                title: "Déploiement".to_string(),
                content: "kubectl apply".to_string(),
                tags: vec!["ops".to_string()],
                ..sample()
            };

            assert!(matches_search(&note, &fold("déploi")));
            assert!(matches_search(&note, &fold("kubectl")));
            assert!(matches_search(&note, &fold("ops")));
            assert!(!matches_search(&note, &fold("terraform")));
        }

        #[test]
        fn search_case_folding_reaches_beyond_ascii() {
            let note = Note {
                title: "Étape suivante".to_string(),
                ..sample()
            };

            assert!(matches_search(&note, &fold("étape")));
            assert!(matches_search(&note, &fold("ÉTAPE")));
        }

        /// Nobody reaches for the accent key to search.
        #[test]
        fn an_unaccented_needle_finds_accented_text() {
            let note = Note {
                title: "Étape de migration".to_string(),
                content: "Prévenir l'équipe".to_string(),
                tags: vec!["déploiement".to_string()],
                ..sample()
            };

            assert!(matches_search(&note, &fold("etape")));
            assert!(matches_search(&note, &fold("equipe")));
            assert!(matches_search(&note, &fold("deploiement")));
        }

        /// Both sides go through `fold`, so it works the other way round too.
        #[test]
        fn an_accented_needle_finds_unaccented_text() {
            let note = Note {
                title: "Etape de migration".to_string(),
                ..sample()
            };

            assert!(matches_search(&note, &fold("Étape")));
        }

        /// Only what a canonical decomposition separates is folded away.
        #[test]
        fn a_letter_of_its_own_is_not_folded_into_another() {
            let note = Note {
                title: "Størrelse".to_string(),
                ..sample()
            };

            assert!(matches_search(&note, &fold("størrelse")));
            assert!(!matches_search(&note, &fold("storrelse")));
        }

        #[test]
        fn folding_leaves_an_ascii_needle_alone() {
            assert_eq!(fold("Kubectl APPLY"), "kubectl apply");
        }

        #[test]
        fn a_checklist_is_found_by_the_text_of_its_items() {
            let note = Note {
                title: "Sprint".to_string(),
                content: String::new(),
                kind: NoteKind::Checklist,
                items: vec![
                    ChecklistItem {
                        text: "Review the migration".to_string(),
                        done: false,
                    },
                    ChecklistItem {
                        text: "Prévenir l'équipe".to_string(),
                        done: true,
                    },
                ],
                ..sample()
            };

            assert!(matches_search(&note, &fold("migration")));
            assert!(matches_search(&note, &fold("equipe")));
            assert!(!matches_search(&note, &fold("terraform")));
        }
    }

    mod hits {
        use super::*;

        fn hit_for(note: Note, search: &str) -> Option<SearchHit> {
            let view = build(
                vec![note],
                Facets::default(),
                &NotesQuery {
                    search: search.to_string(),
                    ..request()
                },
            );

            view.sections
                .into_iter()
                .flat_map(|section| section.notes)
                .next()
                .and_then(|note| note.search_hit)
        }

        #[test]
        fn quotes_the_line_that_matched_and_not_the_first_one() {
            let note = Note {
                content: "first\nsecond\n  kubectl rollout restart\nfourth".to_string(),
                ..sample()
            };

            let hit = hit_for(note, "rollout").expect("a body match is worth quoting");
            assert_eq!(hit.field, SearchField::Body);
            // Trimmed: a card shows one line and it should start with the code.
            assert_eq!(hit.excerpt, "kubectl rollout restart");
        }

        #[test]
        fn says_nothing_when_the_title_is_what_matched() {
            let note = Note {
                title: "Rollout".to_string(),
                content: "kubectl apply".to_string(),
                ..sample()
            };

            // Quoting the title back would repeat the biggest thing on the card.
            assert!(hit_for(note, "rollout").is_none());
        }

        #[test]
        fn quotes_the_tag_and_the_item_the_card_does_not_show() {
            let tagged = Note {
                tags: vec!["urgent".to_string()],
                ..sample()
            };
            let hit = hit_for(tagged, "urgen").expect("a tag match is worth quoting");
            assert_eq!(hit.field, SearchField::Tag);
            assert_eq!(hit.excerpt, "urgent");

            let listed = Note {
                items: vec![
                    ChecklistItem {
                        text: "Bump the version".to_string(),
                        done: false,
                    },
                    ChecklistItem {
                        text: "Push the tag".to_string(),
                        done: false,
                    },
                ],
                ..sample()
            };
            let hit = hit_for(listed, "push").expect("an item match is worth quoting");
            assert_eq!(hit.field, SearchField::Item);
            assert_eq!(hit.excerpt, "Push the tag");
        }

        #[test]
        fn clips_a_line_long_enough_to_be_a_payload() {
            let note = Note {
                content: format!("{}needle", "x".repeat(400)),
                ..sample()
            };

            let hit = hit_for(note, "needle").expect("it still matches");
            // 160 characters and the ellipsis that says there were more.
            assert_eq!(hit.excerpt.chars().count(), EXCERPT_CHARS + 1);
            assert!(hit.excerpt.ends_with('…'));
        }

        #[test]
        fn clips_on_characters_rather_than_bytes() {
            let note = Note {
                content: format!("{}needle", "é".repeat(400)),
                ..sample()
            };

            // A byte slice would have panicked in the middle of one of these.
            let hit = hit_for(note, "needle").expect("an accented needle still matches");
            assert_eq!(hit.excerpt.chars().count(), EXCERPT_CHARS + 1);
        }

        #[test]
        fn carries_nothing_when_nothing_is_searched() {
            let note = Note {
                content: "kubectl apply".to_string(),
                ..sample()
            };

            assert!(hit_for(note, "   ").is_none());
        }
    }

    mod sections {
        use super::*;
        use crate::notes::model::NoteLifecycle;

        fn utc() -> FixedOffset {
            FixedOffset::east_opt(0).unwrap()
        }

        fn now_at(_offset: FixedOffset) -> DateTime<Utc> {
            at(NOW)
        }

        fn note(id: &str, created_at: &str) -> Note {
            Note {
                id: id.to_string(),
                created_at: at(created_at),
                updated_at: at(created_at),
                ..sample()
            }
        }

        fn keys(sections: &[NoteSection]) -> Vec<NoteSectionKey> {
            sections.iter().map(|section| section.key).collect()
        }

        fn ids_in(sections: &[NoteSection], key: NoteSectionKey) -> Vec<String> {
            sections
                .iter()
                .filter(|section| section.key == key)
                .flat_map(|section| section.notes.iter().map(|note| note.id.clone()))
                .collect()
        }

        #[test]
        fn a_real_offset_keeps_its_sign_inverted() {
            assert_eq!(offset_from_minutes(-120).local_minus_utc(), 2 * 3600);
            assert_eq!(offset_from_minutes(300).local_minus_utc(), -5 * 3600);
            assert_eq!(offset_from_minutes(0).local_minus_utc(), 0);
        }

        #[test]
        fn an_absurd_offset_falls_back_to_utc_without_overflowing() {
            // ⚠️ Negating i32::MIN or multiplying i32::MAX by 60 panics in debug while
            // the connection mutex is held, poisoning it for the rest of the process.
            for absurd in [i32::MIN, i32::MAX, -100_000, 100_000, 841, -841] {
                assert_eq!(offset_from_minutes(absurd).local_minus_utc(), 0);
            }
        }

        #[test]
        fn every_unpinned_note_lands_in_exactly_one_section() {
            let offset = utc();
            let notes = vec![
                note("today", "2026-07-25T08:00:00.000Z"),
                note("week", "2026-07-21T08:00:00.000Z"),
                note("older", "2020-01-01T08:00:00.000Z"),
            ];

            let sections = build_sections(notes, None, true, now_at(offset), offset);

            let placed: Vec<String> = sections
                .iter()
                .flat_map(|section| section.notes.iter().map(|note| note.id.clone()))
                .collect();
            assert_eq!(placed.len(), 3);
            assert_eq!(ids_in(&sections, NoteSectionKey::Today), ["today"]);
            assert_eq!(ids_in(&sections, NoteSectionKey::Week), ["week"]);
            assert_eq!(ids_in(&sections, NoteSectionKey::Older), ["older"]);
        }

        #[test]
        fn the_week_section_is_present_even_when_empty() {
            let offset = utc();

            let sections = build_sections(Vec::new(), None, true, now_at(offset), offset);

            assert_eq!(keys(&sections), [NoteSectionKey::Week]);
            assert!(sections[0].show_create_ghost);
        }

        #[test]
        fn only_the_week_section_carries_the_create_ghost() {
            let offset = utc();
            let notes = vec![
                note("today", "2026-07-25T08:00:00.000Z"),
                note("older", "2020-01-01T08:00:00.000Z"),
            ];

            let sections = build_sections(notes, None, true, now_at(offset), offset);

            let with_ghost: Vec<NoteSectionKey> = sections
                .iter()
                .filter(|section| section.show_create_ghost)
                .map(|section| section.key)
                .collect();
            assert_eq!(with_ghost, [NoteSectionKey::Week]);
        }

        #[test]
        fn pinned_notes_leave_the_chronological_sections() {
            let offset = utc();
            let mut pinned = note("pinned", "2026-07-25T08:00:00.000Z");
            pinned.pinned = true;

            let sections = build_sections(vec![pinned], None, true, now_at(offset), offset);

            assert_eq!(ids_in(&sections, NoteSectionKey::Pinned), ["pinned"]);
            assert!(ids_in(&sections, NoteSectionKey::Today).is_empty());
        }

        #[test]
        fn without_the_hoist_a_pinned_note_follows_its_date_like_any_other() {
            let offset = utc();
            let mut pinned = note("pinned", "2026-07-25T08:00:00.000Z");
            pinned.pinned = true;

            let sections = build_sections(vec![pinned], None, false, now_at(offset), offset);

            assert!(!keys(&sections).contains(&NoteSectionKey::Pinned));
            assert_eq!(ids_in(&sections, NoteSectionKey::Today), ["pinned"]);
        }

        #[test]
        fn the_hoist_reaches_the_flat_list_too() {
            let offset = utc();
            let mut pinned = note("pinned", "2019-05-05T08:00:00.000Z");
            pinned.pinned = true;
            let notes = vec![note("recent", "2026-07-25T08:00:00.000Z"), pinned];

            let hoisted = build_sections(notes.clone(), Some(false), true, now_at(offset), offset);
            let untouched = build_sections(notes, Some(false), false, now_at(offset), offset);

            assert_eq!(
                ids_in(&hoisted, NoteSectionKey::Results),
                ["pinned", "recent"]
            );
            assert_eq!(
                ids_in(&untouched, NoteSectionKey::Results),
                ["recent", "pinned"]
            );
        }

        #[test]
        fn empty_sections_other_than_week_are_omitted() {
            let offset = utc();

            let sections = build_sections(
                vec![note("today", "2026-07-25T08:00:00.000Z")],
                None,
                true,
                now_at(offset),
                offset,
            );

            assert_eq!(
                keys(&sections),
                [NoteSectionKey::Today, NoteSectionKey::Week]
            );
        }

        #[test]
        fn filtering_collapses_everything_into_a_single_flat_section() {
            let offset = utc();
            let mut pinned = note("pinned", "2026-07-25T08:00:00.000Z");
            pinned.pinned = true;
            let notes = vec![pinned, note("ancient", "2019-05-05T08:00:00.000Z")];

            let sections = build_sections(notes, Some(false), true, now_at(offset), offset);

            assert_eq!(keys(&sections), [NoteSectionKey::Results]);
            assert_eq!(sections[0].notes.len(), 2);
            assert!(!sections[0].show_create_ghost);
        }

        #[test]
        fn a_section_reports_whether_any_of_its_notes_is_due_soon() {
            let offset = utc();
            let mut expiring = note("expiring", "2026-07-25T08:00:00.000Z");
            expiring.lifecycle = NoteLifecycle::Expires {
                at: at("2026-07-26T00:00:00.000Z"),
            };

            let sections = build_sections(
                vec![expiring, note("plain", "2026-07-25T08:00:00.000Z")],
                None,
                true,
                now_at(offset),
                offset,
            );

            let today = sections
                .iter()
                .find(|s| s.key == NoteSectionKey::Today)
                .unwrap();
            assert!(today.has_expiring_notes);
            let week = sections
                .iter()
                .find(|s| s.key == NoteSectionKey::Week)
                .unwrap();
            assert!(!week.has_expiring_notes);
        }

        #[test]
        fn a_distant_deadline_does_not_light_up_the_section_hint() {
            let offset = utc();
            let mut expiring = note("expiring", "2026-07-25T08:00:00.000Z");
            expiring.lifecycle = NoteLifecycle::Expires {
                at: at("2027-01-01T00:00:00.000Z"),
            };

            let sections = build_sections(vec![expiring], None, true, now_at(offset), offset);

            let today = sections
                .iter()
                .find(|s| s.key == NoteSectionKey::Today)
                .unwrap();
            assert!(!today.has_expiring_notes);
        }

        #[test]
        fn the_day_boundary_follows_the_local_timezone_not_utc() {
            let paris = offset_from_minutes(-120);
            let now = at("2026-07-25T21:30:00.000Z");

            let sections = build_sections(
                vec![note("local-today", "2026-07-25T20:00:00.000Z")],
                None,
                true,
                now,
                paris,
            );

            assert_eq!(ids_in(&sections, NoteSectionKey::Today), ["local-today"]);
        }

        #[test]
        fn a_note_created_just_after_local_midnight_is_not_yesterday() {
            let paris = offset_from_minutes(-120);
            let now = at("2026-07-26T08:00:00.000Z");

            let sections = build_sections(
                vec![note("after-midnight", "2026-07-25T22:10:00.000Z")],
                None,
                true,
                now,
                paris,
            );

            assert_eq!(ids_in(&sections, NoteSectionKey::Today), ["after-midnight"]);
        }

        #[test]
        fn a_note_created_in_the_future_is_not_swallowed() {
            let offset = utc();

            let sections = build_sections(
                vec![note("future", "2030-01-01T00:00:00.000Z")],
                None,
                true,
                now_at(offset),
                offset,
            );

            assert_eq!(ids_in(&sections, NoteSectionKey::Older), ["future"]);
        }

        #[test]
        fn the_order_received_is_preserved_inside_a_section() {
            let offset = utc();
            let notes = vec![
                note("first", "2026-07-25T08:00:00.000Z"),
                note("second", "2026-07-25T07:00:00.000Z"),
            ];

            let sections = build_sections(notes, None, true, now_at(offset), offset);

            assert_eq!(
                ids_in(&sections, NoteSectionKey::Today),
                ["first", "second"]
            );
        }
    }

    mod criteria {
        use super::*;

        fn tagged(id: &str, tags: &[&str]) -> Note {
            Note {
                id: id.to_string(),
                tags: tags.iter().map(ToString::to_string).collect(),
                ..sample()
            }
        }

        fn wanting(tags: &[&str]) -> Criteria {
            let tags: Vec<String> = tags.iter().map(ToString::to_string).collect();
            Criteria::new("", NoteFilter::All, &tags, &[])
        }

        /// `note_tags.tag` is `COLLATE NOCASE`, which folds ASCII and nothing else.
        #[test]
        fn a_tag_is_matched_the_way_the_column_compares_it() {
            let criteria = wanting(&["étape"]);

            assert!(criteria.passes(&tagged("a", &["étape"])));
            assert!(!criteria.passes(&tagged("c", &["Étape"])));
            assert!(wanting(&["urgent"]).passes(&tagged("d", &["URGENT"])));
        }

        #[test]
        fn a_tag_asked_for_with_its_hash_finds_the_stored_one() {
            assert!(wanting(&["#urgent"]).passes(&tagged("a", &["urgent"])));
        }

        #[test]
        fn a_tag_that_normalises_to_nothing_narrows_nothing() {
            let criteria = wanting(&[" # "]);

            assert!(!criteria.narrows());
            assert!(criteria.passes(&tagged("a", &[])));
        }

        #[test]
        fn the_quick_filter_decides_a_match_without_counting_as_narrowing() {
            let criteria = Criteria::new("", NoteFilter::Pinned, &[], &[]);

            assert!(!criteria.narrows());
            assert!(!criteria.passes(&sample()));
            assert!(criteria.passes(&Note {
                pinned: true,
                ..sample()
            }));
        }

        #[test]
        fn the_search_is_trimmed_and_folded_before_it_is_compared() {
            let criteria = Criteria::new("  ÉTAPE ", NoteFilter::All, &[], &[]);

            assert_eq!(criteria.needle(), "etape");
            assert!(criteria.passes(&Note {
                title: "Étape suivante".to_string(),
                ..sample()
            }));
        }
    }
}
