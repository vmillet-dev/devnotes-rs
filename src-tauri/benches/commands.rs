//! What the IPC surface costs, by class of command.
//!
//! Below the command boundary, not through Tauri: a command is four lines, so
//! `store::*` plus `view::*` plus the serde round trip captures nearly all of the cost.
//! `tauri::test::mock_app` would drag the whole app lifecycle in and buy only the IPC
//! transport, which this codebase does not control.
//!
//! ```
//! cargo bench -- --save-baseline main
//! cargo bench -- --baseline main
//! ```

mod corpus;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use devnotes_lib::attachments::{self, model::Attachment, sealed};
use devnotes_lib::libraries;
use devnotes_lib::notes::model::NotePatch;
use devnotes_lib::notes::store;
use devnotes_lib::notes::view::{NoteFilter, NotesQuery};
use devnotes_lib::transfer::{bundle, file};
use devnotes_lib::vault::key::{Cost, Vault};

use corpus::{NOTES, build, now, run_query};

fn query(search: &str) -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search: search.to_string(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: now(),
        tz_offset_minutes: -120,
        pinned_first: true,
    }
}

/// The one that matters most: it runs on every keystroke, behind the 150 ms debounce.
fn whole_corpus_read(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("whole-corpus read");
    // A pass over 8000 notes is long enough that criterion's hundred samples would make
    // this group most of the run.
    group.sample_size(20);

    group.bench_function("query_notes, unfiltered", |b| {
        b.iter(|| black_box(run_query(&mut corpus, &query(""))));
    });

    // What `query_notes` holds the lock for: everything before `view::build`.
    group.bench_function("query_notes, the locked part", |b| {
        b.iter(|| {
            let fetched = store::fetch(&mut corpus.connection, &query("")).expect("a view");
            let decorations = store::decorations(&mut corpus.connection).expect("the decorations");
            black_box((fetched, decorations))
        });
    });

    // Matching runs on the fetched rows, not in SQL — the needle decides the cost.
    group.bench_function("query_notes, search matching nothing", |b| {
        b.iter(|| black_box(run_query(&mut corpus, &query("zzz-no-such-needle"))));
    });

    // Unaccented on purpose: the fold has to strip the accents off every byte of the
    // corpus before it can answer.
    group.bench_function("query_notes, search folding accents", |b| {
        b.iter(|| black_box(run_query(&mut corpus, &query("deploiement"))));
    });

    // Narrowed in SQL to the 160 pinned notes: what a filtered query pays for its side tables.
    group.bench_function("query_notes, pinned only", |b| {
        let pinned = NotesQuery {
            filter: NoteFilter::Pinned,
            ..query("")
        };
        b.iter(|| black_box(run_query(&mut corpus, &pinned)));
    });

    group.finish();
}

/// The editor's round trip, once per field committed.
fn single_write(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("single write");

    group.bench_function("update_note", |b| {
        let id = corpus.note_ids[NOTES / 2].clone();
        let mut at = 0u32;
        b.iter(|| {
            at += 1;
            let patch = NotePatch {
                title: Some(format!("Retitled {at}")),
                ..NotePatch::default()
            };
            black_box(store::update(&mut corpus.connection, &id, &patch, now()).expect("a write"));
        });
    });

    group.finish();
}

/// Two writes that paid for work they never used: a body save read and opened the whole
/// kept history to compare with one entry, and a pin toggle sealed the title, the body and
/// the source to write a boolean.
fn body_writes(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("body writes");
    let id = corpus.note_ids[NOTES / 3].clone();
    let body = store::by_ids(&mut corpus.connection, std::slice::from_ref(&id))
        .expect("the note")
        .remove(0)
        .content;

    // A full history, so every save below also rotates the oldest body out.
    for kept in 0..=devnotes_lib::notes::revision::KEEP {
        let patch = NotePatch {
            content: Some(format!("{body}\n-- {kept}")),
            ..NotePatch::default()
        };
        store::update(&mut corpus.connection, &id, &patch, now()).expect("a write");
    }

    group.bench_function("update_note, body, full history", |b| {
        let mut at = 0u32;
        b.iter(|| {
            at += 1;
            let patch = NotePatch {
                content: Some(format!("{body}\n-- saved {at}")),
                ..NotePatch::default()
            };
            black_box(store::update(&mut corpus.connection, &id, &patch, now()).expect("a write"));
        });
    });

    group.bench_function("update_note, pin toggled", |b| {
        let mut pinned = false;
        b.iter(|| {
            pinned = !pinned;
            let patch = NotePatch {
                pinned: Some(pinned),
                ..NotePatch::default()
            };
            black_box(store::update(&mut corpus.connection, &id, &patch, now()).expect("a write"));
        });
    });

    group.finish();
}

/// One lock, N rows. The selection bar's actions.
fn bulk(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("bulk over 100 notes");
    let ids: Vec<String> = corpus.note_ids.iter().take(100).cloned().collect();

    group.bench_function("tag_notes", |b| {
        b.iter(|| {
            black_box(
                store::tag_many(&mut corpus.connection, &ids, &["bulk".to_string()], now())
                    .expect("a batch"),
            );
        });
    });

    group.bench_function("move_notes", |b| {
        let mut at = 0usize;
        b.iter(|| {
            at += 1;
            let target = &corpus.space_ids[at % corpus.space_ids.len()];
            black_box(
                store::move_many(&mut corpus.connection, &ids, target, now()).expect("a batch"),
            );
        });
    });

    group.bench_function("delete_notes then restore_notes", |b| {
        b.iter(|| {
            store::trash::trash_many(&mut corpus.connection, &ids, now()).expect("a trashing");
            black_box(
                store::trash::restore_many(&mut corpus.connection, &ids).expect("a restoration"),
            );
        });
    });

    group.finish();
}

/// The facet and panel queries, each one a pass over a side table.
fn aggregation(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("corpus-wide aggregation");

    group.bench_function("list_tags", |b| {
        b.iter(|| black_box(store::tag_usage(&mut corpus.connection).expect("the tags")));
    });

    group.bench_function("list_trash, nothing trashed", |b| {
        b.iter(|| {
            black_box(store::trash::list_trashed(&mut corpus.connection).expect("the trash"));
        });
    });

    group.bench_function("list_global_placeholders", |b| {
        b.iter(|| {
            black_box(
                store::global_placeholder_values(&mut corpus.connection).expect("the globals"),
            );
        });
    });

    group.finish();
}

/// `retag` touches every matching row, in one transaction.
fn corpus_rewrite(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("corpus-wide rewrite");

    group.bench_function("rename_tags", |b| {
        let mut at = 0u32;
        b.iter(|| {
            at += 1;
            let (from, into) = if at.is_multiple_of(2) {
                ("ops", "operations")
            } else {
                ("operations", "ops")
            };
            black_box(
                store::retag(&mut corpus.connection, &[from.to_string()], into).expect("a retag"),
            );
        });
    });

    group.finish();
}

/// The bundle both ways, against a real file.
fn disk(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("disk");
    // The bundle is ~100 MB, so these two want their own sample size.
    group.sample_size(10);

    let path = std::env::temp_dir().join("devnotes-bench-export.devnotes");
    let target = path.to_string_lossy().to_string();
    // The corpus seeds no attachment, so the archive carries the bundle alone. What the
    // directory is does not matter; that it exists does.
    let attachments = std::env::temp_dir();

    group.bench_function("export_notes", |b| {
        b.iter(|| {
            let notes = store::all(&mut corpus.connection, None).expect("the corpus");
            let packed = bundle::collect(&mut corpus.connection, notes).expect("a bundle");
            black_box(
                file::write(
                    &target,
                    &packed,
                    &attachments,
                    corpus.connection.vault(),
                    None,
                )
                .expect("a written file"),
            );
        });
    });

    group.bench_function("import_notes, every id already there", |b| {
        b.iter(|| {
            let (incoming, mut payload) = file::read(&target, None).expect("a readable file");
            black_box(
                bundle::merge(&mut corpus.connection, incoming, &mut payload, &attachments)
                    .expect("a merge"),
            );
        });
    });

    group.finish();
    let _ = std::fs::remove_file(&path);
}

/// One thumbnail, as the editor asks for it — once per attachment on the note it opens.
///
/// The second function is not a command: it is the registry lookup an open library spares
/// every read, measured beside what it is spared from.
fn attachment(c: &mut Criterion) {
    let mut corpus = build();
    let mut group = c.benchmark_group("attachment");

    let directory = attachments::files::directory(&corpus.connection);
    std::fs::create_dir_all(&directory).expect("a writable temporary directory");
    let screenshot = Attachment {
        id: "a-screenshot".to_string(),
        note_id: corpus.note_ids[0].clone(),
        file_name: "capture.png".to_string(),
        mime_type: "image/png".to_string(),
        byte_size: 256 * 1024,
        created_at: now(),
    };
    let (db, vault) = corpus.connection.split();
    sealed::write_sealed(
        vault,
        &directory.join(screenshot.stored_name()),
        &vec![7u8; 256 * 1024],
    )
    .expect("a sealed file");
    attachments::store::create(db, vault, &screenshot).expect("a record");

    group.bench_function("read_attachment, 256 kB", |b| {
        b.iter(|| {
            black_box(
                attachments::files::read_plain(&mut corpus.connection, "a-screenshot")
                    .expect("the bytes"),
            )
        });
    });

    let profile =
        std::env::temp_dir().join(format!("devnotes-bench-profile-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&profile).expect("a writable temporary directory");
    libraries::registry::open_directory_in(&profile).expect("a registry");

    group.bench_function("the registry lookup it no longer pays", |b| {
        b.iter(|| {
            let library = libraries::registry::open_directory_in(&profile).expect("a directory");
            std::fs::create_dir_all(library.join("attachments")).expect("a directory");
            black_box(library)
        });
    });

    group.finish();
    let _ = std::fs::remove_dir_all(&profile);
}

/// What `Cost::default` costs is what an attacker pays per guess against a copied library.
/// Here because criterion builds in release: a debug run overstates Argon2 twentyfold. Seeds
/// no corpus, so it runs alone: `cargo bench -- unlock`.
fn unlock(c: &mut Criterion) {
    let mut group = c.benchmark_group("unlock");
    group.sample_size(10);

    group.bench_function("Vault::derive at the shipped cost", |b| {
        b.iter(|| {
            black_box(
                Vault::derive(
                    black_box("a passphrase"),
                    b"0123456789abcdef",
                    Cost::default(),
                )
                .expect("a key"),
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    whole_corpus_read,
    single_write,
    body_writes,
    bulk,
    aggregation,
    corpus_rewrite,
    disk,
    attachment,
    unlock
);
criterion_main!(benches);
