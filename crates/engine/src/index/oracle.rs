//! The index against the path it replaced: random writes through the service functions, and
//! after each one, the same queries answered by both — SQL with every body opened, and the
//! index — compared whole.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::TimeDelta;

use crate::db::open_in_memory;
use crate::folders::model::FolderDraft;
use crate::notes::checklist::ChecklistItem;
use crate::notes::fixtures::{NOW, at};
use crate::notes::kind::NoteKind;
use crate::notes::language::Language;
use crate::notes::model::{NoteDraft, NoteLifecycle, NotePatch};
use crate::notes::priority::Priority;
use crate::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery, SortDirection, SortKey};
use crate::spaces::model::SpaceDraft;
use crate::{Engine, Paths, RecordingSink, folders, notes, spaces};

/// xorshift64: the same seed, the same run, so a failure replays.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % sides as u64).unwrap()
    }

    fn pick<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.roll(from.len())]
    }
}

const TAGS: [&str; 5] = ["auth", "db", "psql", "prod", "écran"];
const WORDS: [&str; 6] = ["psql", "Écrit", "prod", "connexion", "zone", "ÉTAPE"];
const NEEDLES: [&str; 6] = ["", "psql", "ecrit", "auth", "etape", "absent"];

struct World {
    _runtime: tokio::runtime::Runtime,
    engine: Arc<Engine>,
    spaces: Vec<String>,
    folders: Vec<String>,
    dice: Dice,
}

impl World {
    fn new(seed: u64) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let paths = Paths {
            profile: std::env::temp_dir(),
            resources: std::env::temp_dir(),
        };
        let engine = Arc::new(Engine::new(
            paths,
            Arc::new(RecordingSink::default()),
            runtime.handle().clone(),
        ));
        *engine.db.lock().unwrap() = Some(open_in_memory().unwrap());
        let mut world = Self {
            _runtime: runtime,
            engine,
            spaces: Vec::new(),
            folders: Vec::new(),
            dice: Dice(seed),
        };
        for name in ["Perso", "Travail"] {
            let draft = SpaceDraft {
                name: name.to_string(),
            };
            let id = spaces::create_space(&world.engine, draft).unwrap().id;
            world.spaces.push(id);
        }
        for (name, space) in [("SQL", 0), ("Prod", 0), ("Infra", 1)] {
            world.folder(name, space);
        }
        for _ in 0..12 {
            world.create();
        }
        world
    }

    fn folder(&mut self, name: &str, space: usize) {
        let draft = FolderDraft {
            space_id: self.spaces[space].clone(),
            name: name.to_string(),
            parent_id: None,
        };
        let id = folders::create_folder(&self.engine, draft).unwrap().id;
        self.folders.push(id);
    }

    fn text(&mut self) -> String {
        format!("{} {}", self.dice.pick(&WORDS), self.dice.pick(&WORDS))
    }

    fn create(&mut self) {
        let kind = *self.dice.pick(&NoteKind::ALL[..4]);
        let draft = NoteDraft {
            space_id: self.dice.pick(&self.spaces).clone(),
            folder_id: None,
            title: self.text(),
            language: *self
                .dice
                .pick(&[Language::Txt, Language::Sql, Language::Py]),
            content: if kind == NoteKind::Checklist {
                String::new()
            } else {
                format!("{}\n{}\n{{{{host}}}}", self.text(), self.text())
            },
            source: self.text(),
            tags: vec![(*self.dice.pick(&TAGS)).to_string()],
            pinned: self.dice.roll(3) == 0,
            lifecycle: if self.dice.roll(4) == 0 {
                NoteLifecycle::Expires {
                    at: at(NOW) + TimeDelta::days(3),
                }
            } else {
                NoteLifecycle::Permanent
            },
            kind,
            priority: *self.dice.pick(&Priority::ALL),
            items: if kind == NoteKind::Checklist {
                vec![ChecklistItem {
                    text: self.text(),
                    done: false,
                }]
            } else {
                Vec::new()
            },
        };
        notes::create_note(&self.engine, draft).unwrap();
    }

    /// The live notes, as the database holds them.
    fn ids(&self) -> Vec<String> {
        let mut library = crate::db::lock(&self.engine.db).unwrap();
        crate::notes::store::all(&mut library, None)
            .unwrap()
            .into_iter()
            .map(|note| note.id)
            .collect()
    }

    /// One write, drawn among those the index must follow.
    fn write(&mut self) {
        let ids = self.ids();
        if ids.is_empty() {
            self.create();
            return;
        }
        let id = self.dice.pick(&ids).clone();
        let one = vec![id.clone()];
        let engine = Arc::clone(&self.engine);
        let tag = (*self.dice.pick(&TAGS)).to_string();
        match self.dice.roll(12) {
            0 => self.create(),
            1 => {
                let patch = NotePatch {
                    title: Some(self.text()),
                    content: Some(self.text()),
                    ..NotePatch::default()
                };
                let _ = notes::update_note(&engine, id, patch);
            }
            2 => drop(notes::tag_notes(&engine, one, vec![tag])),
            3 => drop(notes::remove_tag(&engine, one, tag)),
            4 => drop(notes::set_pinned(&engine, one, self.dice.roll(2) == 0)),
            5 => drop(notes::set_priority(
                &engine,
                one,
                *self.dice.pick(&Priority::ALL),
            )),
            6 => drop(notes::delete_notes(&engine, one)),
            7 => {
                let space = self.dice.pick(&self.spaces).clone();
                drop(notes::move_notes(&engine, one, space));
            }
            8 => {
                let folder = self.dice.pick(&self.folders).clone();
                drop(folders::file_notes(&engine, one, Some(folder)));
            }
            9 => {
                let values = BTreeMap::from([("host".to_string(), self.text())]);
                drop(notes::set_placeholder_values(&engine, id, values));
            }
            10 => {
                let values = BTreeMap::from([("host".to_string(), self.text())]);
                drop(notes::set_global_placeholders(&engine, values));
            }
            _ => drop(notes::rename_tags(
                &engine,
                vec![tag],
                (*self.dice.pick(&TAGS)).to_string(),
            )),
        }
    }

    fn query(&mut self) -> NotesQuery {
        let space_id = match self.dice.roll(3) {
            0 => None,
            n => Some(self.spaces[n - 1].clone()),
        };
        let folder_id = (self.dice.roll(5) == 0).then(|| self.dice.pick(&self.folders).clone());
        NotesQuery {
            space_id,
            folder_id,
            search: (*self.dice.pick(&NEEDLES)).to_string(),
            filter: *self
                .dice
                .pick(&[NoteFilter::All, NoteFilter::Pinned, NoteFilter::Untriaged]),
            tags: if self.dice.roll(4) == 0 {
                vec![(*self.dice.pick(&TAGS)).to_string()]
            } else {
                Vec::new()
            },
            languages: if self.dice.roll(6) == 0 {
                vec![Language::Sql]
            } else {
                Vec::new()
            },
            kinds: if self.dice.roll(6) == 0 {
                vec![NoteKind::Checklist]
            } else {
                Vec::new()
            },
            priorities: if self.dice.roll(6) == 0 {
                vec![Priority::High]
            } else {
                Vec::new()
            },
            order: NoteOrder {
                key: *self.dice.pick(&[
                    SortKey::Modified,
                    SortKey::Created,
                    SortKey::Priority,
                    SortKey::Format,
                    SortKey::Title,
                ]),
                direction: *self
                    .dice
                    .pick(&[SortDirection::Descending, SortDirection::Ascending]),
            },
            grouping: *self.dice.pick(&[
                Grouping::Date,
                Grouping::Priority,
                Grouping::Format,
                Grouping::None,
            ]),
            ranked: self.dice.roll(2) == 0,
            now: at(NOW) + TimeDelta::hours(1),
            tz_offset_minutes: -120,
            pinned_first: self.dice.roll(2) == 0,
        }
    }
}

#[test]
fn every_query_answers_what_the_database_answers_through_random_writes() {
    for seed in [7, 1_234, 98_765] {
        let mut world = World::new(seed);
        for step in 0..60 {
            world.write();
            for _ in 0..4 {
                let query = world.query();
                let from_index = notes::query_notes(&world.engine, query.clone()).unwrap();
                let from_database =
                    notes::query_notes_from_database(&world.engine, &query).unwrap();
                assert_eq!(
                    serde_json::to_value(&from_index).unwrap(),
                    serde_json::to_value(&from_database).unwrap(),
                    "seed {seed}, step {step}, {query:?}"
                );
            }
            let tags = notes::list_tags(&world.engine).unwrap();
            let mut library = crate::db::lock(&world.engine.db).unwrap();
            let usage = crate::notes::store::tag_usage(&mut library).unwrap();
            drop(library);
            assert_eq!(
                tags.iter()
                    .map(|tag| (tag.tag.clone(), i64::from(tag.note_count)))
                    .collect::<Vec<_>>(),
                usage,
                "seed {seed}, step {step}"
            );
        }
    }
}

/// A query sent while the index fills finishes it first: the views do not say they are partial.
#[test]
fn a_query_landing_between_two_slices_sees_every_note() {
    let world = World::new(42);
    {
        let mut library = crate::db::lock(&world.engine.db).unwrap();
        let mut index = world.engine.index().unwrap();
        index.start(&mut library).unwrap();
        assert!(index.next_slice(&mut library).unwrap());
        assert!(!index.is_complete());
    }
    let everything = NotesQuery {
        space_id: None,
        folder_id: None,
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: Grouping::None,
        ranked: false,
        now: at(NOW),
        tz_offset_minutes: 0,
        pinned_first: false,
    };

    let view = notes::query_notes(&world.engine, everything).unwrap();

    assert_eq!(view.matched, 12);
    assert!(world.engine.index().unwrap().is_complete());
}
