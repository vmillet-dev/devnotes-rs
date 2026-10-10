---
name: add-feature
description: Use when adding or changing a DevNotes feature end to end (Rust command + IPC + Angular UI + tests + docs). Not for pure bug fixes (bug-investigation) or schema-only work (database-migration).
---

# Add a feature

1. Read the matching section of `docs/architecture.md` (headings are named in `CLAUDE.md`) and the nearest existing feature of the same shape. Copy its layout; do not invent a new one.
2. Decide the side: data processing, filtering, rules and validation are **Rust** (`src-tauri/src/<feature>/{model,store}.rs`, command in `<feature>.rs`). The front end describes and displays.
3. Schema change? Run `database-migration` first.
4. New command? Run `tauri-integration` (annotations, `collect_commands!`, `npm run bindings`, `ErrorCode` keys).
5. UI? Run `angular-ui`. Place files per "Where a file goes". Every string goes in `fr.json` **and** `en.json`.
6. Sealed data (titles, bodies, names, attachments) touched? Run `devnotes-security-review`.
7. Tests with the code: Rust inline or `src-tauri/tests/`, Vitest spec using `provideAppTesting()`, an e2e spec if a user journey changes (`desktop-e2e-testing`).
8. Update `docs/architecture.md` and, only if a new trap exists, one line of `CLAUDE.md`.
9. Finish with `verify-change`. Report what ran and what did not.
