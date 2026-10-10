---
name: database-migration
description: Use when changing the SQLite schema, Diesel models or queries in src-tauri (Diesel, append-only migrations, hand-written db/schema.rs). Also when a change affects stored or exported data.
---

# Database migration

1. Never edit an existing folder in `src-tauri/migrations/`. Add `YYYY-MM-DD-0000NN_name/up.sql` (+ `down.sql` if siblings have one), numbered after the last.
2. Prefer additive changes (nullable or defaulted column, new table). Existing libraries must open and keep every row; test the upgrade from the previous schema, not only a fresh DB.
3. Update `db/schema.rs` by hand in the same change; `check_for_backend` on the row types catches drift.
4. Decide sealed vs plain: anything SQL filters/sorts/joins on stays plain, content is sealed (see `devnotes-security-review`). Timestamps via `db::iso8601`; every read filters `deleted_at IS NULL`; foreign keys rely on the per-connection pragma set in `db::configure`.
5. Export/import: new fields carry `#[serde(default)]`; do not bump `FORMAT_VERSION` for an added variant. Backups and `recovery` must still read old libraries.
6. Tests: `cargo test` (in-memory SQLite; `tempfile::TempDir` for disk). Never run against the user's real library.
7. Run `cargo clippy --all-targets --all-features -- -D warnings`. Reference: "Persistence (Rust)", "How far back an upgrade reaches".
