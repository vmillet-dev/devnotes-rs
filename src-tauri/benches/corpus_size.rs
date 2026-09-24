//! What a query costs as the corpus grows, and how much of it crosses the bridge.
//!
//! Kept out of `commands` so that bench's corpus stays one size and its numbers comparable.
//!
//! Slow by construction: it seeds three corpora of ~13 kB notes, the largest of them
//! 20 000. Run it on its own.
//!
//! ```
//! cargo bench --bench corpus_size
//! ```

mod corpus;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use devnotes_lib::notes::store;
use devnotes_lib::notes::view::{NoteFilter, NotesQuery};

use corpus::{Corpus, build_of, now, run_query};

/// The sizes the ticket names. 20 000 notes of ~13 kB is ~260 MB of body — well past
/// what a person writes, which is the point: the shape of the curve is what decides.
const SIZES: [usize; 3] = [1_000, 5_000, 20_000];

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

/// Everything the query does before the view: the SQL, the rows, and opening every
/// sealed value. No `view::build`, no serde.
fn read_only(corpus: &mut Corpus, request: &NotesQuery) -> usize {
    let (notes, _) = store::fetch(&mut corpus.connection, request).expect("a view");

    notes.len()
}

fn query_by_corpus_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("query by corpus size");
    // A pass over 20 000 notes is long enough that a hundred samples would take an hour.
    group.sample_size(10);

    for size in SIZES {
        let mut corpus = build_of(size);

        for (name, request) in [("open", query("")), ("search", query("étape"))] {
            // Printed rather than measured: the payload is a size, not a duration, and it
            // is half of what the ticket is about.
            let payload = run_query(&mut corpus, &request).len();
            println!("{size} notes, {name}: {payload} bytes on the wire");

            group.bench_with_input(BenchmarkId::new(name, size), &size, |b, _| {
                b.iter(|| black_box(run_query(&mut corpus, &request)));
            });

            // The same query stopped before the view is built and serialised: the gap
            // between the two is what a lighter wire shape could take away, and what a
            // LIMIT could not.
            group.bench_with_input(
                BenchmarkId::new(format!("{name}, read only"), size),
                &size,
                |b, _| {
                    b.iter(|| black_box(read_only(&mut corpus, &request)));
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, query_by_corpus_size);
criterion_main!(benches);
