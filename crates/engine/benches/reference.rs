//! The two reference libraries of the migration plan (§ 6.1), generated on disk, never in
//! memory: an in-memory database has no pager, no page cache and no I/O.
//!
//! - **Realistic** (~60 MB of text): 30,000 notes — 20,000 snippets of ~1.5 kB, 5,000 Notes of
//!   ~4 kB, 3,000 lists of 20 items, 1,500 Calcul sheets of 30 lines, 500 Dessins of ~20 kB —,
//!   4 spaces, 300 folders on three levels, 600 tags, 2,000 attachments; 2,000 HTTP requests in
//!   40 collections and 250 folders, 30 environments, 20,000 history entries, 500 cookies.
//! - **Stress** (~400 MB): 30,000 notes of ~13 kB, and the same HTTP.
//!
//! A Dessin carries its scene alone: its two pictures are not what a search or the index reads.

#![allow(dead_code)]

use std::fmt::Write;
use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, Utc};
use engine::db::{self, Library};
use engine::http::cookies::{Change, JarCookie};
use engine::http::model::{HttpMethod, HttpRequestDraft, RequestDocument, RequestKind};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{Note, NoteLifecycle};
use engine::notes::priority::Priority;
use engine::vault::key::Cost;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Size {
    Realistic,
    Stress,
}

pub(crate) const NOTES: usize = 30_000;
const SPACES: usize = 4;
const FOLDERS: usize = 300;
const TAGS: usize = 600;
const ATTACHMENTS: usize = 2_000;
const COLLECTIONS: usize = 40;
const HTTP_FOLDERS: usize = 250;
const REQUESTS: usize = 2_000;
const ENVIRONMENTS: usize = 30;
const HISTORY: usize = 20_000;
const COOKIES: usize = 500;

const LANGUAGES: [Language; 8] = [
    Language::Sql,
    Language::Ts,
    Language::Rs,
    Language::Py,
    Language::Json,
    Language::Go,
    Language::Js,
    Language::Txt,
];

/// The library, its directory erased with it.
pub(crate) struct Reference {
    pub(crate) library: Option<Library>,
    pub(crate) note_ids: Vec<String>,
    pub(crate) space_ids: Vec<String>,
    /// ⚠️ Last: fields drop in order, and Windows refuses to delete under an open connection.
    _directory: Directory,
}

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn now() -> DateTime<Utc> {
    db::iso8601::parse("2026-07-25T09:00:00.000Z").expect("a valid instant")
}

/// Accented, with `{{fields}}`: `fold` has a fast path for ASCII alone.
fn line(seed: usize) -> &'static str {
    match seed % 6 {
        0 => "let étape = migration::run(&mut connection)?; // déploiement",
        1 => "kubectl rollout restart deployment/{{service}} --namespace prod",
        2 => "SELECT calls, mean_exec_time FROM pg_stat_statements LIMIT 20;",
        3 => "const keysOf = <T extends object>(value: T) => Object.keys(value);",
        4 => "curl -H \"Authorization: Bearer {{token}}\" https://api.exemple.fr/v2/états",
        _ => "Vérifier la réplication avant la bascule, puis prévenir l'astreinte.",
    }
}

fn lines(seed: usize, count: usize) -> String {
    (0..count)
        .map(|n| line(seed + n))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown(seed: usize) -> String {
    let mut text = format!("# Procédure {seed}\n\n");
    for section in 0..4 {
        let _ = write!(
            text,
            "## Étape {section}\n\n{}\n\n",
            lines(seed + section, 4)
        );
        text.push_str("- vérifier\n- **déployer**\n- surveiller\n\n");
        text.push_str("| hôte | rôle |\n| --- | --- |\n| db-1 | primaire |\n\n");
        let _ = write!(text, "```sql\n{}\n```\n\n", lines(seed, 3));
    }
    if seed.is_multiple_of(20) {
        text.push_str("```mermaid\nflowchart LR\n  api --> db\n```\n");
    }
    text
}

fn calc(seed: usize) -> String {
    (0..30)
        .map(|n| format!("ligne {n} = {} * {}", seed % 97 + n, n + 2))
        .collect::<Vec<_>>()
        .join("\n")
}

fn scene(seed: usize) -> String {
    let objects: Vec<String> = (0..60)
        .map(|n| {
            format!(
                r#"{{"type":"textbox","left":{},"top":{},"width":180,"text":"Étiquette {seed}-{n} de la vue"}}"#,
                n * 7,
                n * 11
            )
        })
        .collect();
    format!(r#"{{"version":"7.4.0","objects":[{}]}}"#, objects.join(","))
}

/// What the `n`th note is, by the share of each kind in the realistic corpus.
fn kind_of(n: usize, size: Size) -> NoteKind {
    if matches!(size, Size::Stress) {
        return NoteKind::Snippet;
    }
    match n % 60 {
        0..40 => NoteKind::Snippet,
        40..50 => NoteKind::Note,
        50..56 => NoteKind::Checklist,
        56..59 => NoteKind::Calc,
        _ => NoteKind::Drawing,
    }
}

fn note(n: usize, size: Size, space_id: &str, folder_id: Option<&str>) -> Note {
    let kind = kind_of(n, size);
    let content = match (size, kind) {
        (Size::Stress, _) => lines(n, 200),
        (_, NoteKind::Snippet) => lines(n, 22),
        (_, NoteKind::Note) => markdown(n),
        (_, NoteKind::Calc) => calc(n),
        (_, NoteKind::Drawing) => scene(n),
        (_, NoteKind::Checklist) => String::new(),
    };
    let items = if kind == NoteKind::Checklist {
        (0..20)
            .map(|item| ChecklistItem {
                text: format!("Tâche {item} : {}", line(n + item)),
                done: item % 3 == 0,
            })
            .collect()
    } else {
        Vec::new()
    };
    let at = now() - TimeDelta::minutes(i64::try_from(n).unwrap());
    Note {
        id: uuid(n),
        space_id: space_id.to_string(),
        folder_id: folder_id.map(str::to_string),
        title: format!("Note {n} — étape de déploiement"),
        language: if kind == NoteKind::Snippet {
            LANGUAGES[n % LANGUAGES.len()]
        } else {
            Language::Txt
        },
        content,
        source: format!("API Gateway / Auth / {}", n % 50),
        tags: vec![
            format!("tag-{}", n % TAGS),
            format!("tag-{}", (n * 7) % TAGS),
        ],
        pinned: n.is_multiple_of(97),
        created_at: at,
        updated_at: at,
        lifecycle: NoteLifecycle::Permanent,
        kind,
        priority: Priority::ALL[n % Priority::ALL.len()],
        items,
        placeholder_values: std::collections::BTreeMap::new(),
    }
}

fn uuid(n: usize) -> String {
    format!("00000000-0000-7000-8000-{n:012}")
}

pub(crate) fn build(size: Size) -> Reference {
    let directory =
        std::env::temp_dir().join(format!("devnotes-reference-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let vault = engine::vault::file::create(&directory, "a passphrase", Cost::FOR_TESTS).unwrap();
    let mut library = db::open(&directory.join("devnotes.sqlite3"), vault).unwrap();
    let (space_ids, note_ids) = seed(&mut library, size);
    std::fs::create_dir_all(directory.join("attachments")).unwrap();

    Reference {
        library: Some(library),
        note_ids,
        space_ids,
        _directory: Directory(directory),
    }
}

/// Everything but the attachments, into a library already open: the spaces, then the notes.
pub(crate) fn seed(library: &mut Library, size: Size) -> (Vec<String>, Vec<String>) {
    let space_ids: Vec<String> = (0..SPACES)
        .map(|n| {
            engine::spaces::store::create(library, &format!("Espace {n}"))
                .unwrap()
                .id
        })
        .collect();
    let folder_ids = folders(library, &space_ids);
    let note_ids = notes(library, size, &space_ids, &folder_ids);
    http(library);
    (space_ids, note_ids)
}

/// Three levels: 60 at the top of the spaces, two under each of them, then one under those.
fn folders(library: &mut Library, space_ids: &[String]) -> Vec<(String, String)> {
    let mut made: Vec<(String, String)> = Vec::new();
    let mut parents: Vec<Option<(String, String)>> = vec![None; 60];
    for level in 0..3 {
        let mut next = Vec::new();
        for (n, parent) in parents.iter().enumerate() {
            if made.len() >= FOLDERS {
                break;
            }
            let space = parent
                .as_ref()
                .map_or_else(|| space_ids[n % SPACES].clone(), |(_, space)| space.clone());
            let id = engine::folders::store::create(
                library,
                &space,
                parent.as_ref().map(|(id, _)| id.as_str()),
                &format!("Dossier {level}-{n}"),
                now(),
            )
            .unwrap()
            .id;
            made.push((id.clone(), space.clone()));
            next.push(Some((id.clone(), space.clone())));
            if level < 2 {
                next.push(Some((id, space)));
            }
        }
        parents = next;
    }
    made
}

fn notes(
    library: &mut Library,
    size: Size,
    space_ids: &[String],
    folders: &[(String, String)],
) -> Vec<String> {
    (0..NOTES)
        .map(|n| {
            let note = if n % 3 == 0 {
                let (folder, space) = &folders[n % folders.len()];
                note(n, size, space, Some(folder))
            } else {
                note(n, size, &space_ids[n % SPACES], None)
            };
            engine::notes::store::insert_imported(library, &note).unwrap();
            note.id
        })
        .collect()
}

/// Through the service function, once the library is in an engine: a small screenshot each.
pub(crate) fn attach(engine: &engine::Engine, note_ids: &[String]) {
    let pixels = [90_u8, 120, 200, 255].repeat(32 * 32);
    for n in 0..ATTACHMENTS {
        let note = note_ids[(n * 13) % note_ids.len()].clone();
        engine::db::run_here(engine, |e| {
            engine::attachments::attach_image(e, note, format!("capture-{n}"), (32, 32), &pixels)
        })
        .unwrap();
    }
}

fn http(library: &mut Library) {
    let collections: Vec<String> = (0..COLLECTIONS)
        .map(|n| {
            engine::http::store::create_collection(library, &format!("API {n}"), now())
                .unwrap()
                .id
        })
        .collect();
    let folders: Vec<(String, String)> = (0..HTTP_FOLDERS)
        .map(|n| {
            let collection = &collections[n % COLLECTIONS];
            let id = engine::http::store::create_folder(
                library,
                collection,
                None,
                &format!("Ressource {n}"),
                now(),
            )
            .unwrap()
            .id;
            (collection.clone(), id)
        })
        .collect();
    let mut request_ids = Vec::new();
    for n in 0..REQUESTS {
        let (collection, folder) = &folders[n % HTTP_FOLDERS];
        let draft = HttpRequestDraft {
            collection_id: collection.clone(),
            folder_id: (n % 4 != 0).then(|| folder.clone()),
            name: format!("Lister les états {n}"),
            kind: RequestKind::Http,
            method: HttpMethod::Get,
            document: RequestDocument {
                url: format!("https://api.exemple.fr/v2/ressources/{n}?page={{{{page}}}}"),
                ..RequestDocument::default()
            },
        };
        let name = draft.name.clone();
        request_ids.push(
            engine::http::store::create_request(library, &draft, &name, now())
                .unwrap()
                .id,
        );
    }
    for n in 0..ENVIRONMENTS {
        engine::http::environments::store::create(library, &format!("Environnement {n}"), now())
            .unwrap();
    }
    for n in 0..HISTORY {
        let summary = serde_json::from_value(serde_json::json!({
            "name": format!("Lister les états {}", n % REQUESTS),
            "url": format!("https://api.exemple.fr/v2/ressources/{}", n % REQUESTS),
            "millis": 120, "size": 2048, "failure": null, "status": 200,
        }))
        .unwrap();
        let record = serde_json::from_value(serde_json::json!({
            "exchange": {
                "method": "GET",
                "url": format!("https://api.exemple.fr/v2/ressources/{}", n % REQUESTS),
                "headers": [], "body": { "kind": "none" }
            },
            "response": null,
        }))
        .unwrap();
        // Today's: past its retention, the sweep of the next launch would empty it.
        let at = Utc::now() - TimeDelta::minutes(i64::try_from(n).unwrap());
        engine::http::history::store::record(
            library,
            Some(&request_ids[n % REQUESTS]),
            &summary,
            &record,
            at,
        )
        .unwrap();
    }
    let cookies: Vec<Change> = (0..COOKIES)
        .map(|n| {
            Change::Set(JarCookie {
                name: format!("session-{n}"),
                value: "a1b2c3d4e5".to_string(),
                domain: format!("api{}.exemple.fr", n % 25),
                host_only: false,
                path: "/".to_string(),
                expires: None,
                secure: true,
                http_only: true,
                same_site: Some("Lax".to_string()),
                created: None,
            })
        })
        .collect();
    engine::http::cookies::store::apply(library, &cookies, now()).unwrap();
}
