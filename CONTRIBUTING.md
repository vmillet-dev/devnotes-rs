# Contributing to DevNotes

Thanks for taking the time. This file covers what the compiler and CI cannot tell you: the
handful of conventions that are load-bearing, and where the traps are.

DevNotes is licensed under the [GNU GPL v3.0](LICENSE). Opening a pull request means you agree
to have your contribution distributed under those terms.

## Getting set up

You need Node.js 24 — the version `.nvmrc` pins, and the one CI installs — Rust via `rustup`, and
[Tauri's system dependencies](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev
```

`npm install` is required before the first Rust build too — Tauri's build script reads the
front-end config. The toolchain is pinned in `rust-toolchain.toml`, so a new stable release
cannot turn CI red on a commit nobody touched.

## What CI checks

Run these before pushing; they are exactly what the pipeline runs.

```bash
npm run lint && npm test && npm run test:scripts
```

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

`--all-features` is not optional: it is what compiles the `e2e` feature, which nothing else
type-checks.

The end-to-end suite drives a **built** binary, so it needs a rebuild after any change to
`src/` or `src-tauri/`:

```bash
npm run e2e:build && npm run test:e2e
```

That binary carries an identifier of its own, so running the suite never touches the library
you use day to day. `E2E_SPEC=05-spaces npm run test:e2e` runs one scenario alone, on a fresh
profile, the way each CI job does.

## The scripts

| Command                 | What it does                                            |
| ----------------------- | ------------------------------------------------------- |
| `npm start`             | Angular dev server only, port 1420                      |
| `npm run tauri dev`     | The main loop: Angular dev server plus the Tauri window |
| `npm run build`         | Production Angular build, into `dist/devnotes/browser`  |
| `npm run tauri build`   | Full production build, into `src-tauri/target/release`  |
| `npm test`              | Unit tests (Vitest on jsdom — no browser needed)        |
| `npm run test:watch`    | The same, re-running on change                          |
| `npm run test:coverage` | The same, with a v8 coverage report (80% thresholds)    |
| `npm run test:scripts`  | `node --test` on the release-notes generator            |
| `npm run e2e:build`     | Builds the binary the end-to-end suite drives           |
| `npm run test:e2e`      | The end-to-end scenarios, against that binary           |
| `npm run lint`          | ESLint plus a Prettier check                            |
| `npm run lint:fix`      | ESLint `--fix` plus Prettier write                      |
| `npm run bindings`      | Regenerates `bindings.ts` from the Rust signatures      |

`npm start` serves the front end alone, but every `invoke()` fails outside the Tauri window
and the canvas shows its retry screen. Use it for pure styling work and
`npm run tauri dev` for anything else. For Rust-only iteration, `cargo check` from
`src-tauri/` is much faster than a full build.

## The layout

```
src/              Angular front end: core/ (model, data, state, ipc, services) and the
                  interface itself — notes/, titlebar/, banners/, shared/
src-tauri/src/
  notes/          The notes feature: model.rs, language.rs, view.rs, placeholder.rs,
                  trash.rs, checklist.rs, store.rs
  spaces/         The spaces feature: model.rs, store.rs
  attachments/    The attachments feature: model.rs, store.rs (the bytes live on disk)
  transfer/       Import, export, share: the exchange format and the Markdown rendering
  db.rs           Connection, migrations, schema, stored-instant format
  error.rs        The three errors and the translation between them
  desktop.rs      Tray and global shortcuts — native glue, not a feature
docs/             Architecture notes, the release procedure, the UI mockup
```

Both halves are filed by **subject**, not by technical nature. On the Rust side each feature
owns its model, its SQL and the commands that expose it. Deleting `src-tauri/src/notes/`
deletes the feature. [docs/architecture.md](docs/architecture.md) is the detailed reference —
read it before a structural change, and keep it in step when you make one.

## Things that are not guesswork

**Migrations are append-only.** A model change means a new `src-tauri/migrations/`
directory, never an edit to one that already shipped — it has already run on installed
databases. `src-tauri/src/db/schema.rs` is **hand-written**, not generated, so a new column
means editing both it and the SQL. Deleting `app_data_dir()/devnotes.sqlite3` is a legitimate
reset while developing.

**The IPC surface is generated.** `src/app/core/ipc/bindings.ts` comes from the Rust
signatures. A new command needs `#[tauri::command]`, `#[specta::specta]`, and an entry in
`collect_commands![…]` in `src-tauri/src/lib.rs` — that single list both registers with Tauri
and drives the generation. Then `npm run bindings`. Commit the result.

**Commands that touch the database or the disk are `#[tauri::command(async)]`.** A plain
`#[tauri::command]` runs its body inline on the WebView's IPC thread and freezes the window
for as long as it takes. The tray and global-shortcut commands in `desktop.rs` are the
deliberate exception — they want the main thread.

**Errors are codes, not strings.** Commands return `Result<T, AppError>` carrying a stable
`code`, its interpolation `params` and a technical `detail`. A French sentence must never
reach an English UI. Adding an `ErrorCode` variant breaks the front-end build until
`CODE_KEYS`, `IPC_ERROR_CODES` and **both** locale files have their key — that is the
intended behaviour, not an obstacle.

**No user-facing text lives in Rust.** The front end holds the translations; a new note gets
an empty title and the UI renders a translated placeholder. The one exception is
`CHANGELOG.md`, which is data shipped as a file.

**Business rules live in `<feature>/model.rs`**, which imports neither Diesel nor Tauri, so
its tests run without a database. `<feature>/store.rs` is SQL and nothing else.
`<feature>.rs` holds the commands: validate, lock, delegate, translate the error — a command
that grows means a rule landed in the wrong place.

## Style

Code, comments, docstrings and test names are in **English**. UI strings are translation
keys, resolved from `fr.json` and `en.json` — add to both.

**Comments record decisions, not narration.** Keep one only when it says something the code
cannot: a load-bearing ordering, a platform trap, a non-obvious invariant, or why the obvious
approach was rejected. Anything restating a signature or narrating the next line belongs in
neither.

Rust unit tests are inline `#[cfg(test)] mod tests` blocks at the bottom of the file they
cover; `src-tauri/tests/` holds the integration binaries, which see only the crate's public
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
[Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode),
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
and the
[Angular Language Service](https://marketplace.visualstudio.com/items?itemName=Angular.ng-template),
or JetBrains RustRover and WebStorm. Nothing in the repository depends on either.
