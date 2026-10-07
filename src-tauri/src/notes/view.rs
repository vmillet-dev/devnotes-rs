use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, FixedOffset, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use unicode_normalization::char::{decompose_canonical, is_combining_mark};

use super::kind::NoteKind;
use super::language::Language;
use super::model::{self, DisplayNote, Note, NoteLifecycle};
use super::priority::Priority;
use crate::count::saturating_u32;
use crate::folders::model::NoteFolder;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotesQuery {
    /// `None` = every space: a choice, not an absence of one.
    pub space_id: Option<String>,
    /// `None` = every folder, filed or not. No "unfiled" filter: the absence of a chip already
    /// says it.
    #[serde(default)]
    #[specta(optional)]
    pub folder_id: Option<String>,
    pub search: String,
    pub filter: NoteFilter,
    /// A note passes if it carries at least one of these tags.
    pub tags: Vec<String>,
    pub languages: Vec<Language>,
    pub kinds: Vec<NoteKind>,
    pub priorities: Vec<Priority>,
    /// Left out by the palette: the modification date, newest first.
    #[serde(default)]
    #[specta(optional)]
    pub order: NoteOrder,
    #[serde(default)]
    #[specta(optional)]
    pub grouping: Grouping,
    pub now: DateTime<Utc>,
    /// `Date#getTimezoneOffset()`, whose sign is the opposite of the offset (−120 for
    /// UTC+2). Sections reason in local days.
    pub tz_offset_minutes: i32,
    /// Their own section when the view is chronological, the head of the list when flat.
    pub pinned_first: bool,
}

/// What the date view is ordered by. The board keeps the places its cards were given.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteOrder {
    pub key: SortKey,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    #[default]
    Modified,
    Created,
    Priority,
    /// The kind, then the language.
    Format,
    Title,
}

/// How the date view gathers its cards. A search or a facet still makes one flat list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Grouping {
    #[default]
    Date,
    Priority,
    Format,
    None,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    #[default]
    Descending,
    Ascending,
}

/// `Untriaged` = notes with a deadline, those whose fate is not decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NoteFilter {
    All,
    Pinned,
    Untriaged,
}

/// What the search, the quick filter and the four rails ask of a note, normalised once: the
/// date view narrows on it and the board dims on it, so the two cannot disagree on a match.
#[derive(Debug, Clone)]
pub struct Criteria {
    needle: String,
    filter: NoteFilter,
    tags: Vec<String>,
    languages: Vec<Language>,
    kinds: Vec<NoteKind>,
    priorities: Vec<Priority>,
}

impl Criteria {
    pub fn new(
        search: &str,
        filter: NoteFilter,
        tags: &[String],
        languages: &[Language],
        kinds: &[NoteKind],
        priorities: &[Priority],
    ) -> Self {
        Self {
            needle: fold(search.trim()),
            filter,
            tags: model::normalize_tags(tags),
            languages: languages.to_vec(),
            kinds: kinds.to_vec(),
            priorities: priorities.to_vec(),
        }
    }

    /// The search, trimmed and folded; empty when nothing is searched.
    pub fn needle(&self) -> &str {
        &self.needle
    }

    /// Whether the search or a rail narrows the notes. Not the quick filter, under which the
    /// date view keeps its sections.
    pub fn narrows(&self) -> bool {
        !self.needle.is_empty()
            || !self.tags.is_empty()
            || !self.languages.is_empty()
            || !self.kinds.is_empty()
            || !self.priorities.is_empty()
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

        let languages = self.languages.is_empty()
            || (note.kind.has_language() && self.languages.contains(&note.language));
        let kinds = self.kinds.is_empty() || self.kinds.contains(&note.kind);
        let priorities = self.priorities.is_empty() || self.priorities.contains(&note.priority);
        let search = self.needle.is_empty() || matches_search(note, &self.needle);

        filter && tags && languages && kinds && priorities && search
    }
}

impl From<&NotesQuery> for Criteria {
    fn from(query: &NotesQuery) -> Self {
        Self::new(
            &query.search,
            query.filter,
            &query.tags,
            &query.languages,
            &query.kinds,
            &query.priorities,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Facets {
    pub tags: Vec<String>,
    pub languages: Vec<Language>,
    pub kinds: Vec<FacetCount<NoteKind>>,
    pub priorities: Vec<FacetCount<Priority>>,
}

/// How many of the space's notes carry a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FacetCount<T> {
    pub value: T,
    pub count: u32,
}

/// Every value in its declared order, zeros included: a chip keeps its place when its count
/// falls to nothing. `rows` is the column grouped and counted; a value this build does not
/// know counts nowhere.
fn counted<T: Copy>(
    every: &[T],
    stored: impl Fn(T) -> &'static str,
    rows: &[(String, i64)],
) -> Vec<FacetCount<T>> {
    every
        .iter()
        .map(|&value| FacetCount {
            value,
            count: rows
                .iter()
                .find(|(name, _)| name == stored(value))
                .map_or(0, |(_, count)| saturating_u32(*count)),
        })
        .collect()
}

pub fn count_kinds(rows: &[(String, i64)]) -> Vec<FacetCount<NoteKind>> {
    counted(&NoteKind::ALL, NoteKind::as_str, rows)
}

pub fn count_priorities(rows: &[(String, i64)]) -> Vec<FacetCount<Priority>> {
    counted(&Priority::ALL, Priority::as_str, rows)
}

/// Sections gathered by date, priority or format, or all in one; or one flat list of matches,
/// which offers the create card only as a place to create in rather than a list of matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// Split on the creation date when the list is sorted by it, on the last edit otherwise:
    /// the sections and the order cannot disagree.
    Dated {
        by_creation: bool,
    },
    ByPriority,
    ByFormat,
    Whole,
    Flat {
        create_ghost: bool,
    },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotesView {
    pub sections: Vec<NoteSection>,
    /// The space's, not the current filter's: facets drawn from filtered notes would empty
    /// the rail on the first selection.
    pub available_tags: Vec<String>,
    pub available_languages: Vec<Language>,
    pub kind_counts: Vec<FacetCount<NoteKind>>,
    pub priority_counts: Vec<FacetCount<Priority>>,
    pub is_filtering: bool,
    /// `u32` and not `usize`: Specta refuses what JSON cannot carry exactly.
    pub matched: u32,
}

/// No `Title` variant: a note found by its own title needs no excerpt, which would repeat the
/// biggest thing on the card. That case is `SearchMatch::Title`.
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
    /// Unique among the sections, where the key is not: every format section is `format`.
    pub id: String,
    /// What a priority or a format section gathers, which the front names.
    pub group: Option<SectionGroup>,
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
    Priority,
    Format,
    All,
}

impl NoteSectionKey {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Today => "today",
            Self::Week => "week",
            Self::Older => "older",
            Self::Results => "results",
            Self::Priority => "priority",
            Self::Format => "format",
            Self::All => "all",
        }
    }
}

/// Declared in the order the format sections run: snippets by language, then the other kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Type)]
#[serde(tag = "group", rename_all = "camelCase")]
pub enum SectionGroup {
    Language {
        language: Language,
    },
    /// A Note or a todo list, whose kind is its format.
    Kind {
        kind: NoteKind,
    },
    Priority {
        priority: Priority,
    },
}

impl SectionGroup {
    fn format_of(note: &Note) -> Self {
        if note.kind.has_language() {
            Self::Language {
                language: note.language,
            }
        } else {
            Self::Kind { kind: note.kind }
        }
    }

    fn key(self) -> NoteSectionKey {
        match self {
            Self::Priority { .. } => NoteSectionKey::Priority,
            Self::Language { .. } | Self::Kind { .. } => NoteSectionKey::Format,
        }
    }

    fn id(self) -> String {
        match self {
            Self::Language { language } => format!("format-{language}"),
            Self::Kind { kind } => format!("format-{kind}"),
            Self::Priority { priority } => format!("priority-{priority}"),
        }
    }
}

impl NotesQuery {
    /// The same normalisation as on write, or a typed `#urgent` misses `urgent`.
    pub(crate) fn selected_tags(&self) -> Vec<String> {
        model::normalize_tags(&self.tags)
    }

    /// Not inside an opened folder, whose breadcrumb already names it.
    pub fn shows_folder_chips(&self) -> bool {
        self.folder_id.is_none()
    }
}

impl NotesView {
    /// Every note of every section, for the passes that decorate them after [`build`].
    pub fn notes_mut(&mut self) -> impl Iterator<Item = &mut DisplayNote> {
        self.sections
            .iter_mut()
            .flat_map(|section| section.notes.iter_mut())
    }
}

/// What a card carries beside its note, read from other tables, and so apart from [`build`],
/// which reads no database. The canvas, the board and a single note all go through `apply`.
#[derive(Debug, Default)]
pub struct Decorations {
    pub attachment_counts: HashMap<String, u32>,
    /// Empty where no chip is wanted: on the board, and inside an opened folder.
    pub folders: HashMap<String, NoteFolder>,
    pub globals: BTreeMap<String, String>,
}

impl Decorations {
    pub fn apply<'a>(&self, notes: impl IntoIterator<Item = &'a mut DisplayNote>) {
        for note in notes {
            note.attachment_count = self.attachment_counts.get(&note.id).copied().unwrap_or(0);
            note.folder = note
                .folder_id
                .as_ref()
                .and_then(|id| self.folders.get(id))
                .cloned();
            model::apply_global_defaults(note, &self.globals);
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

    // A quick filter keeps the view chronological; a search, a facet or an opened folder
    // switches to a flat list. `build_sections` knows nothing of folders: the inside of one is
    // already sorted by being there.
    order_notes(&mut notes, request.order);

    let inside_folder = request.folder_id.is_some();
    let is_filtering = criteria.narrows() || inside_folder;

    // The ghost card rides on a flat view only when the folder is its whole reason: a place to
    // create in, not a list of matches.
    let layout = if is_filtering {
        Layout::Flat {
            create_ghost: !criteria.narrows(),
        }
    } else {
        match request.grouping {
            Grouping::Date => Layout::Dated {
                by_creation: request.order.key == SortKey::Created,
            },
            Grouping::Priority => Layout::ByPriority,
            Grouping::Format => Layout::ByFormat,
            Grouping::None => Layout::Whole,
        }
    };
    let matched = saturating_u32(notes.len());

    let offset = offset_from_minutes(request.tz_offset_minutes);

    let mut view = NotesView {
        sections: build_sections(notes, layout, request.pinned_first, request.now, offset),
        available_tags: facets.tags,
        available_languages: facets.languages,
        kind_counts: facets.kinds,
        priority_counts: facets.priorities,
        is_filtering,
        matched,
    };

    // A pass of its own, like the attachment counter: threading a second value through
    // `build_sections` would cost every section test an argument.
    apply_search_hits(&mut view, &mut hits);
    if request.order.key == SortKey::Created {
        view.notes_mut().for_each(DisplayNote::dated_by_creation);
    }
    view.notes_mut().for_each(DisplayNote::cut_to_preview);
    view
}

/// Ties fall on the modification date, newest first, then the id: a card never swaps places
/// with its neighbour between two refreshes, whichever way the list runs.
fn order_notes(notes: &mut [Note], order: NoteOrder) {
    // Titles are sealed, so SQL cannot order them; folded once, not per comparison, so that
    // case and accents do not scatter them.
    let titles: HashMap<String, String> = if order.key == SortKey::Title {
        notes
            .iter()
            .map(|note| (note.id.clone(), fold(&note.title)))
            .collect()
    } else {
        HashMap::new()
    };

    notes.sort_by(|a, b| {
        let primary = match order.key {
            SortKey::Modified => a.updated_at.cmp(&b.updated_at),
            SortKey::Created => a.created_at.cmp(&b.created_at),
            SortKey::Priority => a.priority.cmp(&b.priority),
            SortKey::Format => a
                .kind
                .cmp(&b.kind)
                .then_with(|| a.language.as_str().cmp(b.language.as_str())),
            SortKey::Title => titles.get(&a.id).cmp(&titles.get(&b.id)),
        };
        let directed = match order.direction {
            SortDirection::Descending => primary.reverse(),
            SortDirection::Ascending => primary,
        };

        directed.then_with(|| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        })
    });
}

fn apply_search_hits(view: &mut NotesView, hits: &mut HashMap<String, SearchHit>) {
    if hits.is_empty() {
        return;
    }

    for note in view.notes_mut() {
        note.search_hit = hits.remove(&note.id);
    }
}

/// `needle` is expected folded and trimmed. Folded in Rust, not SQL: without ICU, `LOWER()`
/// only handles ASCII. The items count like the content, a todo list having no body. The
/// board reuses this rather than growing a second, subtly different match.
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

    // The Markdown only adds characters, so a body without the needle is skipped unparsed.
    if contains_folded(&note.content, needle)
        && let Some(line) = note
            .readable_body()
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

/// A card shows one line, and a body may hold a minified payload on one.
const EXCERPT_CHARS: usize = 160;

/// Characters, not bytes: `s[..160]` panics in the middle of a `é`.
fn clip(text: &str) -> String {
    let mut clipped: String = text.chars().take(EXCERPT_CHARS).collect();
    if text.chars().nth(EXCERPT_CHARS).is_some() {
        clipped.push('…');
    }
    clipped
}

/// Lowercase and accent-free, so `etape` finds `Étape`, and symmetric since both sides come
/// through here. Only what a canonical decomposition separates is folded: `ø` and `ß` stay.
/// Decomposed per character rather than through `nfd()` over the string, whose lookahead
/// buffering buys nothing when every mark is dropped; the ASCII branches are the common case.
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

/// Not a fold-as-you-compare scan: `str::contains` runs Two-Way, O(n+m), where a window scan
/// is O(n·m), and most notes do not match.
fn contains_folded(haystack: &str, needle: &str) -> bool {
    fold(haystack).contains(needle)
}

const A_WEEK: TimeDelta = TimeDelta::days(7);

const MAX_TZ_OFFSET_MINUTES: u32 = 14 * 60;

/// ⚠️ The sign flips: JavaScript counts the minutes to add to local time to reach UTC (−120 for
/// UTC+2), chrono the offset east. The bound is checked before the multiplication overflows.
pub(crate) fn offset_from_minutes(tz_offset_minutes: i32) -> FixedOffset {
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
        id: key.as_str().to_string(),
        group: None,
        notes,
        show_create_ghost,
    }
}

fn gathered(
    group: SectionGroup,
    notes: Vec<Note>,
    show_create_ghost: bool,
    now: DateTime<Utc>,
) -> NoteSection {
    NoteSection {
        id: group.id(),
        group: Some(group),
        ..section(group.key(), notes, show_create_ghost, now)
    }
}

/// The partition is stable: at equal pinning, the order the list was sorted in holds.
fn hoisted(mut notes: Vec<Note>, pinned_first: bool) -> Vec<Note> {
    if pinned_first {
        notes.sort_by_key(|note| !note.pinned);
    }
    notes
}

/// From the most pressing down. "None" is always there: it hosts the create card, and it is
/// where a new note lands.
fn by_priority(notes: Vec<Note>, pinned_first: bool, now: DateTime<Utc>) -> Vec<NoteSection> {
    let mut groups: BTreeMap<std::cmp::Reverse<Priority>, Vec<Note>> = BTreeMap::new();
    groups.entry(std::cmp::Reverse(Priority::None)).or_default();
    for note in notes {
        groups
            .entry(std::cmp::Reverse(note.priority))
            .or_default()
            .push(note);
    }

    groups
        .into_iter()
        .map(|(std::cmp::Reverse(priority), notes)| {
            gathered(
                SectionGroup::Priority { priority },
                hoisted(notes, pinned_first),
                priority == Priority::None,
                now,
            )
        })
        .collect()
}

/// The create card rides the last section, or a section of its own when there is none.
fn by_format(notes: Vec<Note>, pinned_first: bool, now: DateTime<Utc>) -> Vec<NoteSection> {
    let mut groups: BTreeMap<SectionGroup, Vec<Note>> = BTreeMap::new();
    for note in notes {
        groups
            .entry(SectionGroup::format_of(&note))
            .or_default()
            .push(note);
    }
    if groups.is_empty() {
        return whole(Vec::new(), pinned_first, now);
    }

    let last = groups.len() - 1;
    groups
        .into_iter()
        .enumerate()
        .map(|(at, (group, notes))| gathered(group, hoisted(notes, pinned_first), at == last, now))
        .collect()
}

fn whole(notes: Vec<Note>, pinned_first: bool, now: DateTime<Utc>) -> Vec<NoteSection> {
    vec![section(
        NoteSectionKey::All,
        hoisted(notes, pinned_first),
        true,
        now,
    )]
}

fn results(
    notes: Vec<Note>,
    pinned_first: bool,
    create_ghost: bool,
    now: DateTime<Utc>,
) -> Vec<NoteSection> {
    vec![section(
        NoteSectionKey::Results,
        hoisted(notes, pinned_first),
        create_ghost,
        now,
    )]
}

fn build_sections(
    notes: Vec<Note>,
    layout: Layout,
    pinned_first: bool,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Vec<NoteSection> {
    let local_now = now.with_timezone(&offset);
    let by_creation = match layout {
        Layout::Flat { create_ghost } => return results(notes, pinned_first, create_ghost, now),
        Layout::ByPriority => return by_priority(notes, pinned_first, now),
        Layout::ByFormat => return by_format(notes, pinned_first, now),
        Layout::Whole => return whole(notes, pinned_first, now),
        Layout::Dated { by_creation } => by_creation,
    };

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

        let dated = if by_creation {
            note.created_at
        } else {
            note.updated_at
        }
        .with_timezone(&offset);
        if is_same_local_day(&dated, &local_now) {
            today.push(note);
        } else if is_within(&dated, &local_now, A_WEEK) {
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
    use crate::notes::checklist::ChecklistItem;
    use crate::notes::fixtures::{NOW, at, note as sample};

    fn request() -> NotesQuery {
        NotesQuery {
            space_id: None,
            folder_id: None,
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            kinds: Vec::new(),
            priorities: Vec::new(),
            order: NoteOrder::default(),
            grouping: Grouping::default(),
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
    fn a_decoration_fills_the_counter_the_chip_and_the_proposed_values() {
        let mut filed = model::decorate(
            Note {
                folder_id: Some("f-1".to_string()),
                content: "ssh {{host}}".to_string(),
                ..note("n-1", "Deploy")
            },
            at(NOW),
        );
        let decorations = Decorations {
            attachment_counts: HashMap::from([("n-1".to_string(), 2)]),
            folders: HashMap::from([(
                "f-1".to_string(),
                NoteFolder {
                    id: "f-1".to_string(),
                    name: "Ops".to_string(),
                    colour: crate::folders::model::FolderColour::nth(0),
                },
            )]),
            globals: BTreeMap::from([("host".to_string(), "db.internal".to_string())]),
        };

        decorations.apply([&mut filed]);

        assert_eq!(filed.attachment_count, 2);
        assert_eq!(
            filed.folder.map(|folder| folder.name),
            Some("Ops".to_string())
        );
        assert_eq!(filed.placeholders[0].default_value, "db.internal");
    }

    #[test]
    fn no_chip_is_drawn_inside_an_opened_folder() {
        let inside = NotesQuery {
            folder_id: Some("f-1".to_string()),
            ..request()
        };

        assert!(request().shows_folder_chips());
        assert!(!inside.shows_folder_chips());
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
                kinds: count_kinds(&[("note".to_string(), 2)]),
                priorities: count_priorities(&[("high".to_string(), 1)]),
            },
            &NotesQuery {
                search: "no-such-thing".to_string(),
                ..request()
            },
        );

        assert_eq!(view.available_tags, ["api", "auth"]);
        assert_eq!(view.available_languages, [Language::Json, Language::Txt]);
        assert_eq!(view.kind_counts[1].count, 2);
        assert_eq!(view.priority_counts[3].count, 1);
    }

    #[test]
    fn every_kind_is_counted_in_its_order_even_at_zero() {
        let counts = count_kinds(&[
            ("checklist".to_string(), 3),
            ("snippet".to_string(), 5),
            ("from-a-newer-build".to_string(), 1),
        ]);

        let pairs: Vec<(NoteKind, u32)> = counts.iter().map(|c| (c.value, c.count)).collect();
        assert_eq!(
            pairs,
            [
                (NoteKind::Snippet, 5),
                (NoteKind::Note, 0),
                (NoteKind::Checklist, 3)
            ]
        );
    }

    mod order {
        use super::*;

        fn titles(notes: &[Note]) -> Vec<&str> {
            notes.iter().map(|note| note.title.as_str()).collect()
        }

        fn ordered(mut notes: Vec<Note>, key: SortKey, direction: SortDirection) -> Vec<Note> {
            order_notes(&mut notes, NoteOrder { key, direction });
            notes
        }

        /// Sealed titles are ordered here, folded: case and accents do not scatter them.
        #[test]
        fn titles_run_a_to_z_whatever_their_case_and_accents() {
            let notes = vec![note("a", "zeta"), note("b", "Étape"), note("c", "alpha")];

            let up = ordered(notes.clone(), SortKey::Title, SortDirection::Ascending);
            let down = ordered(notes, SortKey::Title, SortDirection::Descending);

            assert_eq!(titles(&up), ["alpha", "Étape", "zeta"]);
            assert_eq!(titles(&down), ["zeta", "Étape", "alpha"]);
        }

        #[test]
        fn priorities_run_from_the_most_pressing_down() {
            let notes = [
                Priority::Low,
                Priority::Urgent,
                Priority::None,
                Priority::High,
            ]
            .into_iter()
            .enumerate()
            .map(|(at, priority)| Note {
                priority,
                ..note(&at.to_string(), priority.as_str())
            })
            .collect();

            let down = ordered(notes, SortKey::Priority, SortDirection::Descending);

            assert_eq!(titles(&down), ["urgent", "high", "low", "none"]);
        }

        #[test]
        fn a_format_is_its_kind_then_its_language() {
            let of = |id: &str, kind: NoteKind, language: Language| Note {
                language,
                kind,
                ..note(id, id)
            };
            let notes = vec![
                of("list", NoteKind::Checklist, Language::Txt),
                of("sql", NoteKind::Snippet, Language::Sql),
                of("prose", NoteKind::Note, Language::Txt),
                of("json", NoteKind::Snippet, Language::Json),
            ];

            let up = ordered(notes, SortKey::Format, SortDirection::Ascending);

            assert_eq!(titles(&up), ["json", "sql", "prose", "list"]);
        }

        /// Whichever way the list runs: two refreshes never swap neighbours.
        #[test]
        fn ties_fall_on_the_last_edit_then_the_id() {
            let edited = |id: &str, at_: &str| Note {
                updated_at: at(at_),
                ..note(id, id)
            };
            let notes = vec![
                edited("b", "2026-07-25T09:00:00.000Z"),
                edited("c", "2026-07-25T10:00:00.000Z"),
                edited("a", "2026-07-25T09:00:00.000Z"),
            ];

            let up = ordered(notes.clone(), SortKey::Priority, SortDirection::Ascending);
            let down = ordered(notes, SortKey::Priority, SortDirection::Descending);

            assert_eq!(titles(&up), ["c", "a", "b"]);
            assert_eq!(titles(&down), ["c", "a", "b"]);
        }

        #[test]
        fn a_list_sorted_by_creation_dates_its_cards_by_creation() {
            let view = build(
                vec![note("a", "Un")],
                Facets::default(),
                &NotesQuery {
                    order: NoteOrder {
                        key: SortKey::Created,
                        direction: SortDirection::Descending,
                    },
                    ..request()
                },
            );
            let plain = build(vec![note("a", "Un")], Facets::default(), &request());

            let footer = |view: &NotesView| {
                view.sections
                    .iter()
                    .flat_map(|s| &s.notes)
                    .next()
                    .unwrap()
                    .footer
                    .clone()
            };
            assert!(matches!(footer(&view), model::NoteFooter::Created { .. }));
            assert!(matches!(footer(&plain), model::NoteFooter::Age { .. }));
        }
    }

    #[test]
    fn a_selected_priority_counts_as_filtering_and_passes_only_its_notes() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                priorities: vec![Priority::Urgent, Priority::High],
                ..request()
            },
        );
        let criteria = Criteria::new("", NoteFilter::All, &[], &[], &[], &[Priority::Urgent]);
        let counts = count_priorities(&[("urgent".to_string(), 2)]);

        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
        assert!(criteria.passes(&Note {
            priority: Priority::Urgent,
            ..sample()
        }));
        assert!(!criteria.passes(&sample()));
        assert_eq!(counts.len(), Priority::ALL.len());
        assert_eq!((counts[4].value, counts[4].count), (Priority::Urgent, 2));
    }

    #[test]
    fn a_selected_kind_counts_as_filtering_and_passes_only_its_notes() {
        let view = build(
            vec![note("a", "Un")],
            Facets::default(),
            &NotesQuery {
                kinds: vec![NoteKind::Note],
                ..request()
            },
        );
        let criteria = Criteria::new("", NoteFilter::All, &[], &[], &[NoteKind::Note], &[]);

        assert!(view.is_filtering);
        assert_eq!(keys(&view), [NoteSectionKey::Results]);
        assert!(criteria.passes(&Note {
            kind: NoteKind::Note,
            ..sample()
        }));
        assert!(!criteria.passes(&Note {
            kind: NoteKind::Snippet,
            ..sample()
        }));
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
                language: crate::notes::language::Language::Sh,
                content: "first\nsecond\n  kubectl rollout restart\nfourth".to_string(),
                ..sample()
            };

            let hit = hit_for(note, "rollout").expect("a body match is worth quoting");
            assert_eq!(hit.field, SearchField::Body);
            // Trimmed: a card shows one line and it should start with the code.
            assert_eq!(hit.excerpt, "kubectl rollout restart");
        }

        /// A Note is quoted without its Markdown, found by the words it shows.
        #[test]
        fn quotes_a_note_without_its_markdown() {
            let note = Note {
                kind: NoteKind::Note,
                content: "# Standup\n\n- [ ] ask about the **rollout** window".to_string(),
                ..sample()
            };

            let hit = hit_for(note, "rollout").expect("a body match is worth quoting");
            assert_eq!(hit.excerpt, "☐ ask about the rollout window");
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
            // Negating i32::MIN or multiplying i32::MAX by 60 would panic under the connection
            // mutex and poison it.
            for absurd in [i32::MIN, i32::MAX, -100_000, 100_000, 841, -841] {
                assert_eq!(offset_from_minutes(absurd).local_minus_utc(), 0);
            }
        }

        fn edited(id: &str, at_: &str) -> Note {
            Note {
                created_at: at("2019-01-01T08:00:00.000Z"),
                updated_at: at(at_),
                ..note(id, at_)
            }
        }

        /// Sorted by the last edit, the list is split on it too: the two cannot disagree.
        #[test]
        fn the_dated_sections_split_on_the_date_the_list_is_sorted_by() {
            let offset = utc();
            let notes = vec![edited("touched today", "2026-07-25T08:00:00.000Z")];

            let by_edit = build_sections(
                notes.clone(),
                Layout::Dated { by_creation: false },
                true,
                now_at(offset),
                offset,
            );
            let by_creation = build_sections(
                notes,
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

            assert_eq!(ids_in(&by_edit, NoteSectionKey::Today), ["touched today"]);
            assert_eq!(
                ids_in(&by_creation, NoteSectionKey::Older),
                ["touched today"]
            );
        }

        #[test]
        fn priorities_gather_from_the_most_pressing_down_pinned_first_in_each() {
            let offset = utc();
            let with = |id: &str, priority: Priority, pinned: bool| Note {
                pinned,
                priority,
                ..note(id, "2026-07-25T08:00:00.000Z")
            };
            let notes = vec![
                with("low", Priority::Low, false),
                with("urgent", Priority::Urgent, false),
                with("urgent pinned", Priority::Urgent, true),
            ];

            let sections = build_sections(notes, Layout::ByPriority, true, now_at(offset), offset);

            let ids: Vec<&str> = sections.iter().map(|section| section.id.as_str()).collect();
            assert_eq!(ids, ["priority-urgent", "priority-low", "priority-none"]);
            let urgent: Vec<&str> = sections[0]
                .notes
                .iter()
                .map(|note| note.note.id.as_str())
                .collect();
            assert_eq!(urgent, ["urgent pinned", "urgent"]);
            assert!(sections[2].notes.is_empty() && sections[2].show_create_ghost);
        }

        #[test]
        fn formats_gather_snippets_by_language_then_the_other_kinds() {
            let offset = utc();
            let of = |id: &str, language: Language, kind: NoteKind| Note {
                language,
                kind,
                ..note(id, "2026-07-25T08:00:00.000Z")
            };
            let notes = vec![
                of("list", Language::Txt, NoteKind::Checklist),
                of("sql", Language::Sql, NoteKind::Snippet),
                of("prose", Language::Txt, NoteKind::Note),
                of("json", Language::Json, NoteKind::Snippet),
            ];

            let sections = build_sections(notes, Layout::ByFormat, true, now_at(offset), offset);

            let ids: Vec<&str> = sections.iter().map(|section| section.id.as_str()).collect();
            assert_eq!(
                ids,
                [
                    "format-json",
                    "format-sql",
                    "format-note",
                    "format-checklist"
                ]
            );
            assert!(
                sections
                    .iter()
                    .all(|section| section.key == NoteSectionKey::Format)
            );
            let ghosts: Vec<bool> = sections.iter().map(|s| s.show_create_ghost).collect();
            assert_eq!(ghosts, [false, false, false, true]);
        }

        /// Without a grouping, and without a note, the create card still has a place.
        #[test]
        fn no_grouping_is_one_section_that_always_holds_the_create_card() {
            let offset = utc();

            let empty = build_sections(Vec::new(), Layout::Whole, true, now_at(offset), offset);
            let formats =
                build_sections(Vec::new(), Layout::ByFormat, true, now_at(offset), offset);

            assert_eq!(keys(&empty), [NoteSectionKey::All]);
            assert!(empty[0].show_create_ghost);
            assert_eq!(keys(&formats), [NoteSectionKey::All]);
        }

        #[test]
        fn every_unpinned_note_lands_in_exactly_one_section() {
            let offset = utc();
            let notes = vec![
                note("today", "2026-07-25T08:00:00.000Z"),
                note("week", "2026-07-21T08:00:00.000Z"),
                note("older", "2020-01-01T08:00:00.000Z"),
            ];

            let sections = build_sections(
                notes,
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

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

            let sections = build_sections(
                Vec::new(),
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

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

            let sections = build_sections(
                notes,
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

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

            let sections = build_sections(
                vec![pinned],
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

            assert_eq!(ids_in(&sections, NoteSectionKey::Pinned), ["pinned"]);
            assert!(ids_in(&sections, NoteSectionKey::Today).is_empty());
        }

        #[test]
        fn without_the_hoist_a_pinned_note_follows_its_date_like_any_other() {
            let offset = utc();
            let mut pinned = note("pinned", "2026-07-25T08:00:00.000Z");
            pinned.pinned = true;

            let sections = build_sections(
                vec![pinned],
                Layout::Dated { by_creation: true },
                false,
                now_at(offset),
                offset,
            );

            assert!(!keys(&sections).contains(&NoteSectionKey::Pinned));
            assert_eq!(ids_in(&sections, NoteSectionKey::Today), ["pinned"]);
        }

        #[test]
        fn the_hoist_reaches_the_flat_list_too() {
            let offset = utc();
            let mut pinned = note("pinned", "2019-05-05T08:00:00.000Z");
            pinned.pinned = true;
            let notes = vec![note("recent", "2026-07-25T08:00:00.000Z"), pinned];

            let hoisted = build_sections(
                notes.clone(),
                Layout::Flat {
                    create_ghost: false,
                },
                true,
                now_at(offset),
                offset,
            );
            let untouched = build_sections(
                notes,
                Layout::Flat {
                    create_ghost: false,
                },
                false,
                now_at(offset),
                offset,
            );

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
                Layout::Dated { by_creation: true },
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

            let sections = build_sections(
                notes,
                Layout::Flat {
                    create_ghost: false,
                },
                true,
                now_at(offset),
                offset,
            );

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
                Layout::Dated { by_creation: true },
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

            let sections = build_sections(
                vec![expiring],
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

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
                Layout::Dated { by_creation: true },
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
                Layout::Dated { by_creation: true },
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
                Layout::Dated { by_creation: true },
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

            let sections = build_sections(
                notes,
                Layout::Dated { by_creation: true },
                true,
                now_at(offset),
                offset,
            );

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
            Criteria::new("", NoteFilter::All, &tags, &[], &[], &[])
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
            let criteria = Criteria::new("", NoteFilter::Pinned, &[], &[], &[], &[]);

            assert!(!criteria.narrows());
            assert!(!criteria.passes(&sample()));
            assert!(criteria.passes(&Note {
                pinned: true,
                ..sample()
            }));
        }

        #[test]
        fn the_search_is_trimmed_and_folded_before_it_is_compared() {
            let criteria = Criteria::new("  ÉTAPE ", NoteFilter::All, &[], &[], &[], &[]);

            assert_eq!(criteria.needle(), "etape");
            assert!(criteria.passes(&Note {
                title: "Étape suivante".to_string(),
                ..sample()
            }));
        }
    }
}
