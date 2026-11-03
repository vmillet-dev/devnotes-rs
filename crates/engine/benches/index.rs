//! The in-memory index on the two reference corpora (`reference.rs`, plan § 6.1), against the
//! budgets of § 6.2: built ≤ 1.5 s / ≤ 4 s, search p95 ≤ 50 / 120 ms, `list_tags` ≤ 5 / 10 ms,
//! a write ≤ 5 ms, a batch of 100 ≤ 30 ms.
//!
//! ```
//! cargo bench -p engine --bench index
//! ```

mod reference;

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use engine::db::run_here;
use engine::notes::model::NotePatch;
use engine::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};
use engine::notes::{list_tags, query_notes, tag_notes, untag_notes, update_note};
use engine::{Engine, Paths, RecordingSink};

use reference::{Reference, Size, build, now};

/// An engine over a reference library, and the runtime its queue answers on.
struct Bench {
    engine: Arc<Engine>,
    runtime: tokio::runtime::Runtime,
    corpus: Reference,
}

fn bench(size: Size) -> Bench {
    let mut corpus = build(size);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let paths = Paths {
        profile: std::env::temp_dir(),
        resources: std::env::temp_dir(),
    };
    let engine = Arc::new(Engine::new(
        paths,
        Arc::new(RecordingSink::default()),
        runtime.handle().clone(),
    ));
    *engine.db().lock().unwrap() = corpus.library.take();
    reference::attach(&engine, &corpus.note_ids);
    Bench {
        engine,
        runtime,
        corpus,
    }
}

impl Bench {
    fn fill(&self) {
        self.runtime
            .block_on(engine::index::fill(Arc::clone(&self.engine)));
    }
}

fn query(search: &str) -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search: search.to_string(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        kinds: Vec::new(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: Grouping::default(),
        ranked: false,
        now: now(),
        tz_offset_minutes: -120,
        pinned_first: true,
    }
}

fn label(size: Size) -> &'static str {
    match size {
        Size::Realistic => "realistic",
        Size::Stress => "stress",
    }
}

fn index_build(c: &mut Criterion) {
    for size in [Size::Realistic, Size::Stress] {
        let bench = bench(size);
        let mut group = c.benchmark_group(format!("index_build/{}", label(size)));
        group.sample_size(10);
        group.bench_function("fill, every slice", |b| {
            b.iter(|| {
                bench.engine.wipe_index();
                bench.fill();
            });
        });
        group.finish();
    }
}

fn index_query(c: &mut Criterion) {
    for size in [Size::Realistic, Size::Stress] {
        let bench = bench(size);
        bench.fill();
        let engine = &bench.engine;
        let mut group = c.benchmark_group(format!("index_query/{}", label(size)));
        group.sample_size(20);
        for (name, search) in [
            ("unfiltered", ""),
            ("search matching nothing", "zzzzzz"),
            ("search folding accents", "etape"),
        ] {
            group.bench_function(name, |b| {
                b.iter(|| black_box(run_here(engine, |e| query_notes(e, query(search)).unwrap())));
            });
        }
        let space = bench.corpus.space_ids[0].clone();
        group.bench_function("space, tag and pinned", |b| {
            b.iter(|| {
                let mut asked = query("");
                asked.space_id = Some(space.clone());
                asked.tags = vec!["tag-7".to_string()];
                asked.filter = NoteFilter::Pinned;
                black_box(run_here(engine, |e| query_notes(e, asked).unwrap()))
            });
        });
        group.bench_function("list_tags", |b| {
            b.iter(|| black_box(run_here(engine, |e| list_tags(e).unwrap())));
        });
        group.finish();
    }
}

fn index_refresh(c: &mut Criterion) {
    for size in [Size::Realistic, Size::Stress] {
        let bench = bench(size);
        bench.fill();
        let engine = &bench.engine;
        let id = bench.corpus.note_ids[1].clone();
        let batch: Vec<String> = bench.corpus.note_ids[..100].to_vec();
        let mut group = c.benchmark_group(format!("index_refresh/{}", label(size)));
        group.bench_function("update_note, title", |b| {
            let mut n = 0_u32;
            b.iter(|| {
                n += 1;
                let patch = NotePatch {
                    title: Some(format!("Retitrée {n}")),
                    ..NotePatch::default()
                };
                black_box(run_here(engine, |e| {
                    update_note(e, id.clone(), patch).unwrap()
                }))
            });
        });
        group.bench_function("tag_notes then untag_notes, 100", |b| {
            b.iter(|| {
                run_here(engine, |e| {
                    let tagged = tag_notes(e, batch.clone(), vec!["banc".to_string()]).unwrap();
                    untag_notes(e, tagged).unwrap()
                })
            });
        });
        group.finish();
    }
}

criterion_group!(benches, index_build, index_query, index_refresh);
criterion_main!(benches);
