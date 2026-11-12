//! The first launch's samples: a space, two folders and six notes, each showing one thing. The
//! texts are translated here, in the language the application opens in; the core writes them
//! in one transaction (`seed_samples`), and decides where they land.

use chrono::{DateTime, Days, Local, NaiveTime, Utc};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{NoteDraft, NoteLifecycle, SampleNote};
use engine::notes::priority::Priority;
use i18n::{Key, keys};

/// In the open library's preferences: written once the samples are seeded, or found unneeded.
pub const SEEDED_KEY: &str = "devnotes.notes.samplesSeeded";

/// ⚠️ Untranslatable: a translation reads `{{name}}` as one of its parameters.
const PSQL_SNIPPET: &str = "psql -h {{host}} -p {{port=5432}} -U {{user}} -d {{database}}";

/// A document to open as a graph: data reads the same in every language.
const API_RESPONSE: &str = r#"{
  "service": "billing",
  "version": "1.4.2",
  "healthy": true,
  "replicas": 3,
  "endpoints": [
    { "path": "/invoices", "method": "GET", "p95Ms": 42 },
    { "path": "/invoices", "method": "POST", "p95Ms": 118 },
    { "path": "/health", "method": "GET", "p95Ms": 3 }
  ],
  "owner": { "team": "payments", "onCall": "@oncall" }
}"#;

/// On one line on purpose: Shift+Alt+F is what lays it out.
const UNFORMATTED: &str = "const retries={max:3,delayMs:250};export async function withRetry<T>(task:()=>Promise<T>,left=retries.max):Promise<T>{try{return await task()}catch(error){if(left<=0)throw error;await new Promise(r=>setTimeout(r,retries.delayMs));return withRetry(task,left-1)}}";

/// Indexes into `Samples::folders`, which have no id until the core writes them; their order
/// is the order the board lays its zones out in.
const SNIPPETS: u32 = 0;
const START_HERE: u32 = 1;

const DEADLINE_DAYS: u64 = 7;

pub struct Samples {
    pub space: String,
    pub folders: Vec<String>,
    pub notes: Vec<SampleNote>,
}

pub fn samples(language: i18n::Language, now: DateTime<Utc>) -> Samples {
    use keys::notes::samples as text;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let checklist = [
        text::checklist::OPEN,
        text::checklist::FIELDS,
        text::checklist::GRAPH,
        text::checklist::SEARCH,
        text::checklist::FORMAT,
        text::checklist::TOOLS,
        text::checklist::PALETTE,
        text::checklist::BOARD,
        text::checklist::KIND,
    ]
    .map(|key| ChecklistItem {
        text: tr(key),
        done: false,
    });
    // The two Notes open the board, the snippets fill a zone; the list is left loose on
    // purpose: « no folder » is a state of its own, and the first launch shows it.
    let notes = vec![
        filed(
            START_HERE,
            NoteDraft {
                content: tr(text::welcome::CONTENT),
                source: tr(text::welcome::SOURCE),
                tags: vec![tr(text::tags::DEVNOTES)],
                pinned: true,
                ..draft(tr(text::welcome::TITLE), NoteKind::Note)
            },
        ),
        filed(
            SNIPPETS,
            NoteDraft {
                language: Language::Sh,
                content: PSQL_SNIPPET.into(),
                source: tr(text::snippet::SOURCE),
                tags: vec![tr(text::tags::DATABASE), tr(text::tags::EXAMPLE)],
                ..draft(tr(text::snippet::TITLE), NoteKind::Snippet)
            },
        ),
        filed(
            SNIPPETS,
            NoteDraft {
                language: Language::Json,
                content: API_RESPONSE.into(),
                source: tr(text::json::SOURCE),
                tags: vec![tr(text::tags::JSON), tr(text::tags::EXAMPLE)],
                ..draft(tr(text::json::TITLE), NoteKind::Snippet)
            },
        ),
        filed(
            START_HERE,
            NoteDraft {
                content: tr(text::release::CONTENT),
                tags: vec![tr(text::tags::DEVNOTES)],
                ..draft(tr(text::release::TITLE), NoteKind::Note)
            },
        ),
        SampleNote {
            folder: None,
            draft: NoteDraft {
                tags: vec![tr(text::tags::DEVNOTES)],
                lifecycle: NoteLifecycle::Expires { at: deadline(now) },
                priority: Priority::High,
                items: checklist.to_vec(),
                ..draft(tr(text::checklist::TITLE), NoteKind::Checklist)
            },
        },
        filed(
            SNIPPETS,
            NoteDraft {
                language: Language::Ts,
                content: UNFORMATTED.into(),
                source: tr(text::format::SOURCE),
                tags: vec![tr(text::tags::TYPESCRIPT), tr(text::tags::EXAMPLE)],
                ..draft(tr(text::format::TITLE), NoteKind::Snippet)
            },
        ),
    ];
    Samples {
        space: tr(text::SPACE),
        folders: vec![tr(text::folders::SNIPPETS), tr(text::folders::START_HERE)],
        notes,
    }
}

/// The space is left empty: it does not exist yet, and the core fills it in.
fn draft(title: String, kind: NoteKind) -> NoteDraft {
    NoteDraft {
        space_id: String::new(),
        folder_id: None,
        title,
        language: Language::Txt,
        content: String::new(),
        source: String::new(),
        tags: Vec::new(),
        pinned: false,
        lifecycle: NoteLifecycle::Permanent,
        kind,
        priority: Priority::None,
        items: Vec::new(),
    }
}

fn filed(folder: u32, draft: NoteDraft) -> SampleNote {
    SampleNote {
        folder: Some(folder),
        draft,
    }
}

/// The end of the local day a week on: midnight would leave a note dated today expired at once.
fn deadline(now: DateTime<Utc>) -> DateTime<Utc> {
    let day = now.with_timezone(&Local).date_naive() + Days::new(DEADLINE_DAYS);
    let end = NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap_or(NaiveTime::MIN);
    day.and_time(end)
        .and_local_timezone(Local)
        .latest()
        .map_or(now, |at| at.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fields_snippet_keeps_its_fields_in_every_language() {
        for language in [i18n::Language::En, i18n::Language::Fr] {
            let samples = samples(language, Utc::now());
            let psql = &samples.notes[1].draft;
            assert_eq!(psql.content, PSQL_SNIPPET);
            assert!(!psql.title.is_empty());
        }
    }

    #[test]
    fn the_samples_are_translated() {
        let en = samples(i18n::Language::En, Utc::now());
        let fr = samples(i18n::Language::Fr, Utc::now());

        assert_ne!(en.space, fr.space);
        assert_ne!(en.notes[0].draft.title, fr.notes[0].draft.title);
        assert!(en.notes[0].draft.title.contains("DevNotes"));
    }

    #[test]
    fn the_list_is_due_at_the_end_of_a_local_day_a_week_on() {
        let now = Utc::now();
        let samples = samples(i18n::Language::En, now);
        let list = &samples.notes[4];
        let NoteLifecycle::Expires { at } = list.draft.lifecycle else {
            panic!("the list has no deadline");
        };

        let local = at.with_timezone(&Local);
        assert_eq!(
            local.date_naive(),
            now.with_timezone(&Local).date_naive() + Days::new(7)
        );
        assert_eq!(
            local.time(),
            NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap()
        );
        assert!(list.folder.is_none());
        assert_eq!(list.draft.items.len(), 9);
    }
}
