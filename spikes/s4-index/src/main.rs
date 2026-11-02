//! Spike S4: the in-memory notes index on the realistic and the stress corpora.
//!
//! `cargo run --release -p s4-index -- <realistic|stress> <db path>` generates the corpus once
//! (sealed with AES-256-GCM like `vault::key`, base64 in TEXT columns), then measures the index
//! build, its resident memory and typical queries, with the folded text kept plain and then lz4.

use std::collections::HashMap;
use std::time::Instant;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use rusqlite::{Connection, params};
use unicode_normalization::char::{decompose_canonical, is_combining_mark};

const KEY: [u8; 32] = [7; 32];

fn cipher() -> Aes256Gcm {
    Aes256Gcm::new_from_slice(&KEY).expect("a 32-byte key")
}

fn seal(cipher: &Aes256Gcm, rng: &mut impl Rng, text: &str) -> String {
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce);
    let mut joined = nonce.to_vec();
    joined.extend(cipher.encrypt(Nonce::from_slice(&nonce), text.as_bytes()).expect("sealed"));
    BASE64.encode(joined)
}

fn open(cipher: &Aes256Gcm, sealed: &str) -> String {
    let raw = BASE64.decode(sealed).expect("base64");
    let (nonce, body) = raw.split_at(12);
    String::from_utf8(cipher.decrypt(Nonce::from_slice(nonce), body).expect("opened")).expect("utf8")
}

/// `notes::view::fold`, copied.
fn fold(text: &str) -> String {
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

// --- Corpus -------------------------------------------------------------------------------

const LINES: &[&str] = &[
    "    let étape = migration::run(&mut connection)?; // déploiement\n",
    "    kubectl rollout restart deployment/api --namespace prod\n",
    "    SELECT calls, mean_exec_time FROM pg_stat_statements LIMIT 20;\n",
    "    const keysOf = <T extends object>(value: T) => Object.keys(value);\n",
    "curl -H 'Authorization: Bearer {{token}}' https://{{host}}/v1/élèves\n",
    "## Résumé de la réunion — décisions prises\n",
    "- [ ] vérifier les sauvegardes du serveur de préproduction\n",
    "| colonne | valeur | commentaire |\n",
];

fn text_of(rng: &mut impl Rng, bytes: usize) -> String {
    let mut text = String::with_capacity(bytes + 80);
    while text.len() < bytes {
        text.push_str(LINES[rng.gen_range(0..LINES.len())]);
    }
    text
}

struct Spec {
    kind: &'static str,
    count: usize,
    bytes: usize,
}

fn specs(stress: bool) -> Vec<Spec> {
    if stress {
        return vec![Spec { kind: "snippet", count: 30_000, bytes: 13_000 }];
    }
    vec![
        Spec { kind: "snippet", count: 20_000, bytes: 1_500 },
        Spec { kind: "markdown", count: 5_000, bytes: 4_000 },
        Spec { kind: "checklist", count: 3_000, bytes: 0 },
        Spec { kind: "calc", count: 1_500, bytes: 900 },
        Spec { kind: "drawing", count: 500, bytes: 20_000 },
    ]
}

fn generate(path: &str, stress: bool) {
    let _ = std::fs::remove_file(path);
    let mut db = Connection::open(path).expect("db");
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE notes (id TEXT PRIMARY KEY, space_id TEXT, folder_id TEXT, kind TEXT,
           language TEXT, priority TEXT, title TEXT, content TEXT, source TEXT,
           updated_at TEXT, deleted_at TEXT);
         CREATE INDEX notes_updated_at ON notes (updated_at DESC);
         CREATE TABLE note_tags (note_id TEXT, tag TEXT);
         CREATE TABLE checklist_items (note_id TEXT, position INTEGER, text TEXT);",
    )
    .expect("schema");
    let cipher = cipher();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let tx = db.transaction().expect("tx");
    let mut seed = 0usize;
    for spec in specs(stress) {
        for _ in 0..spec.count {
            seed += 1;
            let id = format!("{seed:032x}");
            let content = text_of(&mut rng, spec.bytes);
            tx.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, 'rs', 'none', ?5, ?6, ?7, ?8, NULL)",
                params![
                    id,
                    format!("space-{}", seed % 4),
                    format!("folder-{}", seed % 300),
                    spec.kind,
                    seal(&cipher, &mut rng, &format!("Note {seed} — étape de déploiement")),
                    seal(&cipher, &mut rng, &content),
                    seal(&cipher, &mut rng, "Runbook"),
                    format!("2026-07-{:02}T09:00:00.000Z", 1 + seed % 28),
                ],
            )
            .expect("note");
            for tag in [seed % 600, (seed * 7) % 600] {
                tx.execute("INSERT INTO note_tags VALUES (?1, ?2)", params![id, format!("tag-{tag}")])
                    .expect("tag");
            }
            if spec.kind == "checklist" {
                for position in 0..20 {
                    tx.execute(
                        "INSERT INTO checklist_items VALUES (?1, ?2, ?3)",
                        params![id, position, seal(&cipher, &mut rng, &format!("Étape {position} à vérifier"))],
                    )
                    .expect("item");
                }
            }
        }
    }
    tx.commit().expect("commit");
}

// --- Index --------------------------------------------------------------------------------

struct Entry {
    id: u32,
    space: u8,
    folder: u16,
    kind: u8,
    updated: i64,
    tags: Vec<u16>,
    title: String,
    folded: Folded,
    body_lz4: Vec<u8>,
}

enum Folded {
    Plain(String),
    Lz4(Vec<u8>),
}

impl Folded {
    fn contains(&self, finder: &memchr::memmem::Finder) -> bool {
        match self {
            Self::Plain(text) => finder.find(text.as_bytes()).is_some(),
            Self::Lz4(packed) => {
                let text = lz4_flex::decompress_size_prepended(packed).expect("lz4");
                finder.find(&text).is_some()
            }
        }
    }
    fn heap(&self) -> usize {
        match self {
            Self::Plain(text) => text.capacity(),
            Self::Lz4(packed) => packed.capacity(),
        }
    }
}

struct Row {
    id: u32,
    space: u8,
    folder: u16,
    kind: u8,
    updated: i64,
    title: String,
    content: String,
    source: String,
    items: Vec<String>,
}

const SLICE: usize = 2_000;

fn build(db: &Connection, compress_folded: bool) -> (Vec<Entry>, f64) {
    let cipher = cipher();
    let started = Instant::now();
    let mut tags: HashMap<String, Vec<u16>> = HashMap::new();
    let mut statement = db.prepare("SELECT note_id, tag FROM note_tags").expect("tags");
    let mut rows = statement.query([]).expect("rows");
    while let Some(row) = rows.next().expect("row") {
        let tag: String = row.get(1).expect("tag");
        let tag: u16 = tag[4..].parse().expect("n");
        tags.entry(row.get(0).expect("id")).or_default().push(tag);
    }
    let mut items: HashMap<String, Vec<String>> = HashMap::new();
    let mut statement = db.prepare("SELECT note_id, text FROM checklist_items ORDER BY note_id, position").expect("items");
    let mut rows = statement.query([]).expect("rows");
    while let Some(row) = rows.next().expect("row") {
        items.entry(row.get(0).expect("id")).or_default().push(row.get(1).expect("text"));
    }

    let mut statement = db
        .prepare("SELECT id, space_id, folder_id, kind, title, content, source, updated_at FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC")
        .expect("notes");
    let mut rows = statement.query([]).expect("rows");
    let mut index = Vec::new();
    let mut first_slice = None;
    let mut sealed: Vec<(String, Row)> = Vec::with_capacity(SLICE);
    loop {
        let row = rows.next().expect("row");
        if let Some(row) = row {
            let id: String = row.get(0).expect("id");
            let space: String = row.get(1).expect("space");
            let folder: String = row.get(2).expect("folder");
            let kind: String = row.get(3).expect("kind");
            sealed.push((
                id.clone(),
                Row {
                    id: u32::from_str_radix(&id[24..], 16).expect("id"),
                    space: space[6..].parse().expect("space"),
                    folder: folder[7..].parse().expect("folder"),
                    kind: ["snippet", "markdown", "checklist", "calc", "drawing"].iter().position(|k| *k == kind).expect("kind") as u8,
                    updated: row.get::<_, String>(7).expect("at")[8..10].parse().expect("day"),
                    title: row.get(4).expect("title"),
                    content: row.get(5).expect("content"),
                    source: row.get(6).expect("source"),
                    items: items.remove(&id).unwrap_or_default(),
                },
            ));
            if sealed.len() < SLICE {
                continue;
            }
        }
        if sealed.is_empty() {
            break;
        }
        // One slice: SQLite read above is sequential, opening and folding is parallel.
        let slice: Vec<Entry> = sealed
            .par_drain(..)
            .map(|(key, row)| {
                let title = open(&cipher, &row.title);
                let body = open(&cipher, &row.content);
                let source = open(&cipher, &row.source);
                let items: Vec<String> = row.items.iter().map(|item| open(&cipher, item)).collect();
                let entry_tags = tags.get(&key).cloned().unwrap_or_default();
                let mut searchable = String::with_capacity(title.len() + body.len() + 64);
                searchable.push_str(&title);
                searchable.push('\n');
                searchable.push_str(&body);
                for item in &items {
                    searchable.push('\n');
                    searchable.push_str(item);
                }
                searchable.push('\n');
                searchable.push_str(&source);
                let mut folded = fold(&searchable);
                folded.shrink_to_fit();
                let folded = if compress_folded {
                    Folded::Lz4(lz4_flex::compress_prepend_size(folded.as_bytes()))
                } else {
                    Folded::Plain(folded)
                };
                Entry {
                    id: row.id,
                    space: row.space,
                    folder: row.folder,
                    kind: row.kind,
                    updated: row.updated,
                    tags: entry_tags,
                    title,
                    folded,
                    body_lz4: lz4_flex::compress_prepend_size(body.as_bytes()),
                }
            })
            .collect();
        index.extend(slice);
        if first_slice.is_none() {
            first_slice = Some(started.elapsed().as_secs_f64() * 1e3);
        }
    }
    (index, first_slice.unwrap_or(0.0))
}

fn rss_mb() -> (f64, f64) {
    let status = std::fs::read_to_string("/proc/self/status").expect("status");
    let field = |name: &str| -> f64 {
        status
            .lines()
            .find(|line| line.starts_with(name))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<f64>().ok())
            .unwrap_or(0.0)
            / 1024.0
    };
    (field("VmRSS:"), field("VmHWM:"))
}

fn percentile(mut samples: Vec<f64>, p: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() as f64 - 1.0) * p).round() as usize]
}

fn time<T>(runs: usize, mut work: impl FnMut() -> T) -> (f64, f64) {
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        std::hint::black_box(work());
        samples.push(started.elapsed().as_secs_f64() * 1e3);
    }
    (percentile(samples.clone(), 0.5), percentile(samples, 0.95))
}

fn queries(index: &[Entry]) {
    let report = |name: &str, (p50, p95): (f64, f64)| println!("  {name:<36} p50 {p50:7.2} ms   p95 {p95:7.2} ms");

    report("empty (all, sorted by updated)", time(30, || {
        let mut ids: Vec<(i64, u32)> = index.iter().map(|e| (e.updated, e.id)).collect();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        ids.len()
    }));
    for needle in ["deploiement", "pg_stat", "zzz-absent", "étape 7"] {
        let folded = fold(needle);
        let finder = memchr::memmem::Finder::new(folded.as_bytes());
        report(&format!("search {needle:?}"), time(30, || {
            index.par_iter().filter(|e| e.folded.contains(&finder)).map(|e| e.id).collect::<Vec<_>>().len()
        }));
    }
    report("filter space+kind+tag", time(30, || {
        index.iter().filter(|e| e.space == 2 && e.kind == 0 && e.tags.contains(&42)).count()
    }));
    report("facets (tag counts, kinds, folders)", time(30, || {
        let mut tag_counts = vec![0u32; 600];
        let mut kinds = [0u32; 5];
        let mut folders = vec![0u32; 300];
        for entry in index {
            for tag in &entry.tags {
                tag_counts[*tag as usize] += 1;
            }
            kinds[entry.kind as usize] += 1;
            folders[entry.folder as usize] += 1;
        }
        tag_counts.iter().sum::<u32>() + kinds[0] + folders[0]
    }));
    let finder = memchr::memmem::Finder::new(b"kubectl");
    report("search + quote 50 bodies (lz4 out)", time(30, || {
        let hits: Vec<&Entry> = index.par_iter().filter(|e| e.folded.contains(&finder)).collect();
        hits.iter().take(50).map(|e| lz4_flex::decompress_size_prepended(&e.body_lz4).expect("lz4").len()).sum::<usize>()
    }));
}

fn main() {
    let mut args = std::env::args().skip(1);
    let corpus = args.next().unwrap_or_else(|| "realistic".into());
    let path = args.next().unwrap_or_else(|| format!("/tmp/s4-{corpus}.sqlite3"));
    let stress = corpus == "stress";
    if !std::path::Path::new(&path).exists() {
        let started = Instant::now();
        generate(&path, stress);
        println!("generated {path} in {:.1} s", started.elapsed().as_secs_f64());
    }
    let mode = args.next().unwrap_or_else(|| "plain".into());
    let db = Connection::open(&path).expect("db");
    let (base, _) = rss_mb();
    let started = Instant::now();
    let (index, first) = build(&db, mode == "lz4");
    let built = started.elapsed().as_secs_f64() * 1e3;
    drop(db);
    let (rss, peak) = rss_mb();
    let plain: usize = index.iter().map(|e| e.title.len()).sum::<usize>();
    let folded: usize = index.iter().map(|e| e.folded.heap()).sum();
    let bodies: usize = index.iter().map(|e| e.body_lz4.capacity()).sum();
    println!("{corpus} / folded {mode}: {} notes", index.len());
    println!("  first slice ({SLICE} notes)            {first:7.1} ms");
    println!("  full build                           {built:7.1} ms");
    println!("  resident {:.0} MB (process before {:.0} MB, peak {:.0} MB)", rss, base, peak);
    println!("  heap: folded {:.0} MB, bodies lz4 {:.0} MB, titles {:.1} MB", folded as f64 / 1e6, bodies as f64 / 1e6, plain as f64 / 1e6);
    queries(&index);
}
