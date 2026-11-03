# Contributing to DevNotes

Thanks for taking the time. This file covers what the compiler and CI cannot tell you: the
handful of conventions that are load-bearing, and where the traps are.

DevNotes is licensed under the [GNU GPL v3.0](LICENSE). Opening a pull request means you agree
to have your contribution distributed under those terms.

DevNotes is being rewritten from Angular + Tauri to egui, in Rust alone
([the plan](docs/egui-migration-plan.md)). Until the egui application is complete, `main` has no
usable application; the Angular one is kept for reading in `reference/angular/`, and runs from
the `angular-reference` tag (see [its README](reference/angular/README.md)).

## Getting set up

You need Rust via `rustup` (the toolchain is pinned in `rust-toolchain.toml`, so a new stable
release cannot turn CI red on a commit nobody touched) and Node.js 24 — the version `.nvmrc`
pins — for Prettier, the icons and the release scripts only.

```bash
npm install
cargo run -p app -- --profile ../devnotes-profile
```

`--profile` keeps the run apart from an installed DevNotes: its libraries, logs and window
geometry all go in that directory.

## What CI checks

Run these before pushing; they are exactly what the pipeline runs.

```bash
npm run lint && npm run test:scripts
```

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace
```

`--all-features` is not optional: it is what compiles the `e2e` flavour, which nothing else
type-checks.

## The scripts

| Command                 | What it does                                                |
| ----------------------- | ----------------------------------------------------------- |
| `npm run lint`          | Prettier check over the Markdown, JSON and YAML             |
| `npm run format`        | Prettier write                                              |
| `npm run test:scripts`  | `node --test` on the release-notes generator                |
| `npm run icons`         | Regenerates `crates/app/icons/` from `devnotes.svg`         |
| `cargo bench -p engine` | The core's benchmarks (not in CI; see docs/architecture.md) |

## The layout

```
crates/engine/src/
  notes/          The notes feature: model.rs, language.rs, view.rs, placeholder.rs,
                  trash.rs, checklist.rs, store.rs
  index/          The open library's notes, decrypted and folded, in memory
  spaces/         The spaces feature: model.rs, store.rs
  attachments/    The attachments feature: model.rs, store.rs (the bytes live on disk)
  transfer/       Import, export, share: the exchange format and the Markdown rendering
  db.rs           Connection, migrations, schema, stored-instant format
  error.rs        The three errors and the translation between them
crates/i18n/      The interface's translations
reference/angular The Angular application and its scenarios, for reading only
docs/             Architecture notes, the migration plan, the release procedure, the mockups
```

Everything is filed by **subject**, not by technical nature. Each feature owns its model, its
SQL and the service functions that expose it. Deleting `crates/engine/src/notes/` deletes the
feature. [docs/architecture.md](docs/architecture.md) is the detailed reference — read it before
a structural change, and keep it in step when you make one.

## Things that are not guesswork

**Migrations are append-only.** A model change means a new `crates/engine/migrations/`
directory, never an edit to one that already shipped — it has already run on installed
databases. `crates/engine/src/db/schema.rs` is **hand-written**, not generated, so a new column
means editing both it and the SQL.

**Errors are codes, not strings.** Service functions return `Result<T, AppError>` carrying a
stable `code`, its interpolation `params` and a technical `detail`. A French sentence must never
reach an English UI, and `detail` never reaches the screen at all.

**No user-facing text lives in Rust code.** The interface's strings are translation keys,
resolved from `fr.json` and `en.json`; a new note gets an empty title and the UI renders a
translated placeholder. The one exception is `CHANGELOG.md`, which is data shipped as a file.

**Business rules live in `<feature>/model.rs`**, which imports neither Diesel nor the UI, so
its tests run without a database. `<feature>/store.rs` is SQL and nothing else.
`<feature>.rs` holds the service functions: validate, lock, delegate, translate the error — one
that grows means a rule landed in the wrong place.

## Style

Code, comments, docstrings and test names are in **English**. UI strings are translation
keys — add to both `fr.json` and `en.json`.

**Comments record decisions, not narration.** Keep one only when it says something the code
cannot: a load-bearing ordering, a platform trap, a non-obvious invariant, or why the obvious
approach was rejected. Anything restating a signature or narrating the next line belongs in
neither.

Rust unit tests are inline `#[cfg(test)] mod tests` blocks at the bottom of the file they
cover; `crates/<crate>/tests/` holds the integration binaries, which see only the crate's public
API. Test names are sentences — `a_blank_name_is_refused_with_its_field`, not `test_name_2`.

## Pull requests

Branch off `main`, keep one subject per pull request, and make sure the checks above pass.

**Label your issues.** Release notes are generated from merged pull requests, and a pull
request inherits the labels of the issue it closes (`Closes #42`) — GitHub propagates
nothing on its own. See [docs/releasing.md](docs/releasing.md) for how an entry gets
filed. Nothing is ever dropped for want of a label.

Found something broken or surprising? Open an issue with what you did, what you expected and
what happened. A version number and an OS help more than they look like they would.

## Editor setup

[VS Code](https://code.visualstudio.com/) with
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer), or
JetBrains RustRover. Nothing in the repository depends on either.
