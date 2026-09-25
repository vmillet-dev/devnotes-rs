# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

DevNotes — a desktop notes application for developers. Front-end **Angular 22** (standalone components, signals, zoneless change detection), native engine **Rust / Tauri v2**, SQLite through Diesel.

A note is a snippet (a body with a language, a source, `{{fields}}`; in the Text language, a formatted body stored as Markdown) or a todo list (ordered items). Notes live in spaces and folders, are searched, filtered and tagged, and can be seen as a dated canvas or as a board. Around them: a 30-day trash with undo, revisions of a body, multiple selection with bulk actions, corpus-wide tag management, a quick-paste palette on a global shortcut, attachments, import / export / share, and help panels in the "À propos" menu. The library is **encrypted**, there can be several, each behind its own passphrase, and a fresh install opens on sample notes.

**Data processing belongs to Rust.** Filtering, grouping into sections, facets, tag normalisation, search matching and what a card's footer shows all run in `src-tauri/src/notes/`: `query_notes` returns a ready-to-render `NotesView`, and the front end describes the query and displays the answer. Deliberate exceptions: relative-time **formatting** (labels age without a round trip), the ISO ↔ `Date` conversion at the boundary, syntax highlighting (it colours the unsaved draft), and pure UI concerns.

**The Rust back-end is feature-first.** `notes/`, `spaces/`, `folders/`, `attachments/` and `transfer/` each own their model, their SQL and their commands: `<feature>.rs` holds the `#[tauri::command]`s (validate, lock, delegate, translate the error — a command that grows means a rule landed in the wrong place), `<feature>/model.rs` the types and rules (no Diesel, no Tauri), `<feature>/store.rs` the SQL (no rules). What belongs to no feature stays at the root: `error.rs`, `db.rs` (the connection `Mutex`, `db::schema`, `db::migration`, `db::iso8601`), `desktop.rs` (tray and global shortcuts), and the library plumbing (`vault`, `libraries`, `layout`, `backup`, `recovery`). → `docs/architecture.md` → "Feature-first, not layer-first".

**The front-end tree is the shape of the interface.** `notes/`, `titlebar/` and `banners/` hold components only, and a folder's path is its address on screen: a component lives in its parent's folder, one with two parents rises to their nearest common ancestor. `notes/` puts the page and its keyboard at its root, what several zones draw in `notes/ui/`, then its four zones — `sidebar/`, `header/`, `canvas/`, `overlays/` — each with a container that injects what it draws. **Everything without a place on screen lives in `core/`**: `model/`, `data/` (repositories and the wire mapper), `state/` (the stores, flat), `ipc/`, and one folder per subject under `services/`. `shared/` crosses areas of the screen and **injects nothing**, except `DialogComponent`, which injects its neighbour `DialogStack`. ⚠️ `core/ipc/` stays at the root of `core/`: `src-tauri/src/lib.rs` writes `bindings.ts` to that exact path. → "Where a file goes".

Primary language for code comments, docstrings, and UI strings in this repo is **English**.

**Comments record decisions, not narration.** Keep a comment only when it says
something the code cannot: a load-bearing ordering, a platform trap, a
non-obvious invariant, or why the obvious approach was rejected. Delete anything
that restates a signature, narrates the next line, or repeats what
`docs/architecture.md` already says.

**There is a budget, and it is about 10% of a file's lines.** Past that, a file
is explaining itself instead of reading. Two rules keep it there: a comment
answers "what would a reader get wrong here?" and nothing else, and it says so in
**one or two lines**, four at the very most. What a pull request explained — what
was tried, what broke, which run caught it, how many milliseconds the rejected
version cost — belongs in the pull request: it describes a state of the code that
no longer exists, and nothing fails when it goes stale. No issue numbers in code.
⚠️ is for the expensive ones only: a trap that costs a reader a day.

`docs/architecture.md` is the detailed reference, and the canonical place for it — not this file and not the README. Read it before a structural change, and keep it in sync with one. Every bullet below is a trap in one or two lines; the explanation is there, under the heading named after the arrow.

## Commands

Run all commands from the repo root (`package.json` there wraps both Angular and Tauri).

- `npm install` — JS dependencies, also required before the first Rust build (Tauri's build script reads front-end config).
- `npm run tauri dev` — the dev loop: `ng serve` with hot reload, and the Tauri window rebuilt on Rust changes. It also regenerates `bindings.ts` at every launch.
- `npm start` / `ng serve` — Angular alone, on port **1420** (fixed in `angular.json`; `devUrl` in `tauri.conf.json` depends on it).
- `npm run build` — production Angular build into `dist/devnotes/browser` (`frontendDist`). Run it before `e2e:build`: `npm test` does not type-check like the build does.
- `npm run tauri build` — the native executable and installer, in `src-tauri/target/release`.
- `cargo build` / `cargo check` from `src-tauri/` — Rust alone, faster than a full `tauri build`.
- `npm test` — unit tests with **Vitest** through `@angular/build:unit-test` (jsdom). `npm run test:watch`, and `npm run test:coverage` with 80% thresholds.
- `npm run lint` — ESLint (`angular-eslint` with its template accessibility rules, member order, one import style), a Prettier check, and `tsc` over `e2e/`, whose scenarios `tsx` runs without type-checking. `npm run lint:fix`, `npm run format`.
- `cargo test` from `src-tauri/` — inline `#[cfg(test)] mod tests`, plus the integration binaries in `src-tauri/tests/` (`notes/` one module per subject, `spaces`, `folders`, `transfer`, `ipc_contract`, `language_corpus`) sharing `tests/common/mod.rs`. In-memory SQLite; a test writing to disk holds a `tempfile::TempDir`.
- `npm run test:scripts` — `node --test` on the seven sweeps and scripts under `scripts/` that read shipped files off disk (release notes, palette contrast, e2e waits, focus rings, language hues, board geometry, motion). ⚠️ The files are named one by one in `package.json`: Node 24 (`.nvmrc`, CI) does not expand a directory. A new test file has to be added there.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` from `src-tauri/` — `Cargo.toml` forbids `unsafe_code`, denies `clippy::all` and `clippy::pedantic`, and warns on `rust_2018_idioms` and `unreachable_pub`, which `-D warnings` turns into errors. The toolchain is pinned in `rust-toolchain.toml`.
- `npm run bindings` — regenerates `src/app/core/ipc/bindings.ts` from the Rust signatures without launching the app.
- `npm run icons` — regenerates `src-tauri/icons` from `devnotes.svg`. ⚠️ The `.ico`'s 16, 24 and 32 px come from the pixel-aligned drawings in `icons/source/`, not from the master: scaled, they blur in the tray and the taskbar.
- `npm run e2e:build` then `npm run test:e2e` — WebdriverIO + `tauri-driver`, twenty-five spec files under `e2e/specs/`, against the **assembled** application: rebuild after any change to `src/` or `src-tauri/`.
- `cargo bench` from `src-tauri/` — criterion against a file-backed library of 8000 notes of ~13 kB. Not in CI, not in `cargo test`. `--save-baseline main` then `--baseline main` compares locally; the durable record is the table in `docs/architecture.md`. ⚠️ `autobenches = false`, `bench = false` on the lib and both bins, and no `Drop` on `Corpus` are all load-bearing. → "Benchmarks".
- **Releasing is a `workflow_dispatch`.** Bump the version in `src-tauri/Cargo.toml`, `package.json` and both lockfiles, merge to `main`, then Actions → Release, `dry_run` first. → "Releasing".

## Things that will bite you

### Libraries, the vault and the disk

- **⚠️ Nothing answers until the library is unlocked.** `Db = Mutex<Option<Library>>` is empty until `unlock_vault` fills it, and a command run first answers `Locked`; the front end never creates the outlet before then, so no store has a "locked" branch. → "Encryption at rest".
- **Sealed and not sealed is a line.** Titles, bodies, sources, items, space and folder names, field values and attachments are sealed; what SQL filters, sorts or joins on (tags, instants, ids, `kind`, `language`) is not. Never SQLCipher (measured 4× slower, vendored OpenSSL). → "What is sealed, and what is not".
- **The passphrase wraps the key, it does not derive it**, and is `zeroize`d before a command returns. ⚠️ Changing it rewrites every retained backup's `vault.json` too (`backup::rewrap`), or the old phrase still opens them. → "Changing the passphrase".
- **The 12-character floor is for choosing a phrase** (`vault::MINIMUM_LENGTH`, crossing as `MINIMUM_PASSPHRASE_LENGTH`): a new library, a changed phrase, a protected export. Unlocking checks nothing but emptiness — the key file refuses a wrong phrase. → "The floor".
- **⚠️ An open library answers `Library::directory()`.** `libraries::open_directory(app)` reads the registry off disk and is only for what runs while none is open; `app_data_dir()` is the profile, which holds no library. The names in `layout.rs` are the address of an installed library — renaming one loses it. → "Several libraries, and the registry beside them".
- **Switching library reloads the page.** `open_library` empties the `Mutex` and `LibrariesStore` calls `AppWindowService.reload()`: every store is `providedIn: 'root'` and would carry the other library's state past the gate. Neither the open library nor the last one can be deleted, refused in Rust and in the store.
- **Two `preferences.json`, one prefix apart.** `devnotes.notes.*` is the library's (`LibraryPreferencesService`, re-opened on every switch), everything else the application's; `automaticBackups` stays with the application because Rust reads it before the front boots. → "Two scopes of preference, one prefix apart".
- **Files that must not be half-written are staged then renamed**: the registry, the key file, an export. `fs::write` truncates first.
- **The launch copy is taken before the sweeps** (`backup::rotate`, `VACUUM INTO` — under WAL the file alone is no snapshot), with its key file and `attachments/` **hard-linked** (`backup::link_files`, which never copies over a name already there), at most one a day. **Restoring** empties the `Mutex` before a file moves, sets the live pair and `attachments/` aside in `replaced/` and matches the id against the listing. → "The copies, and putting one back".
- **Every open runs `db::quick_check` before the migrations.** A damaged or forgotten library is set aside by `recovery::set_aside`, whose two reasons treat `vault.json` in opposite ways. → "The gate".
- **An attachment's bytes live in the library's `attachments/`** under `model::stored_name`, which filters both halves of the name: an imported id decides it. Write the file, then the record; a purge collects the file names before the `DELETE`. `open_attachment` writes its one decrypted copy under the profile's `open/`. → "Attachments".

### The Rust back-end

- **⚠️ A command touching the database or the disk is `#[tauri::command(async)]`**, or it runs on the WebView's thread and freezes the window. It lands on a Tokio worker, not `spawn_blocking`. `desktop.rs` is the exception. → "Persistence (Rust)".
- **One lock serialises every command**, and that is measured: `query_notes` holds it for 90 % of its cost, so only a read connection would change it. → "Who holds the lock".
- **A module holding commands is `pub`; the rest is `pub(crate)`,** so `dead_code` and `unreachable_pub` can speak. `#[specta::specta]` resolves its macro from the crate root.
- **Migrations are append-only** (`src-tauri/migrations/`, `embed_migrations!`), and `db/schema.rs` is written by hand: a column is added in both, and `check_for_backend` on `NoteRow` catches a divergence. Libraries from 0.2.0 on open; 0.1.0's `PRAGMA user_version` is refused by name. → "How far back an upgrade reaches".
- **`PRAGMA foreign_keys` is per connection** (`db::configure`): without it every `ON DELETE CASCADE` is inert.
- **`db::iso8601` always writes milliseconds.** The TEXT columns sort lexicographically, and `.` precedes `Z`.
- **Every read filters `deleted_at IS NULL`**: deleting a note stamps it, `notes::trash::RETENTION` (30 days) decides when it goes, and `purge` only touches trashed rows. → "The trash, and undoing a deletion".
- **`updated_at` moves only when the user edited the note.** Deleting a space, a retag, restoring, undoing, filling a field and restoring a revision all leave it: the canvas sorts on it.
- **Search matching is Rust, not SQL** (`view::fold` lowercases and strips accents; SQLite's `LOWER()` is ASCII-only). Coarse filters stay in SQL.
- **Tag normalisation is `notes::model::normalize_tags`'s alone**, and `replace_tags` re-reads rather than sorts (`COLLATE NOCASE`). A rename onto an existing tag is a merge (`store::retag`).
- **A batch answers what it changed**, and its undo reverses exactly that: `move_notes` the placements left, `tag_notes` the pairs added (compared ASCII-only, like `NOCASE`). Never rebuilt from the selection. → "Multiple selection and bulk actions".
- **Side tables are read by bound ids up to `BIND_AT_MOST` (500), by the space's subquery past it** (`related::attach_related`).
- **A body before an edit is kept** (`note_revisions`, the last 20, snippets only, never exported), taken inside `store::update` before the row moves; `restore_revision` goes back to that point and drops the newer ones. → "Revisions: the body before the edit".
- **A fresh install is seeded in one transaction** (`seed_samples`: the space, two folders, four notes), guarded by the samples marker **and** "no space at all", instants a millisecond apart in opposite directions. → "The first launch".
- **Input is validated in the model**, before locking (`ValidationError`, `SpaceDraft::validated_name`, `validate_move_target`): a rule held only by a form is not held.
- **A `{{field}}` is decided in `notes::placeholder` alone.** Names are `[A-Za-z0-9_-]` (Angular's `{{ user.name }}` is not a field); values live in `note_placeholders`, written only by `set_placeholder_values`; a global variable is a proposed default (note value → global → the text's default), never copied into `value`. → "`{{fields}}` in a snippet".
- **A todo list has items, not a body** (`note_items`, keyed by position, rewritten whole). `kind` and `items` carry `#[serde(default)]` for older exports; the search scans items, export writes `- [x]`, and language detection is skipped. → "The two kinds of note".
- **A Text note is Markdown** (`Note::is_rich_text`: a snippet in `txt`), written by the rich editor. A list sends its words (`notes::markdown::plain`), marked `truncated`; the search matches the Markdown and quotes the words. A patch never detects a language: `detect_language` is asked on a paste into an empty Text note. → "A Text note is written formatted, and stored as Markdown".
- **A folder cuts one space into regions**: deleting it leaves its notes unfiled (`ON DELETE SET NULL`), filing is a batch command of its own (`file_notes`), and a note moving space leaves its folder. → "Folders inside a space".
- **Pinning is the only order a space has** (`spaces.pinned`, then the name in Rust, the names being sealed); `pinned` is `#[serde(default)]` for older exports. → "Managing spaces from the switcher".
- **The application describes itself from `Cargo.toml`** at compile time (`app_info::METADATA`, with `[package.metadata.devnotes]` read by `build.rs`); its version is asked of the running binary.
- **A closed enum is declared once** (`closed_enum!`: `Language`, `NoteKind`, `FolderColour`, `GlobalAction`): one literal per variant for serde, the column, `Display` and `FromStr`.
- **An import degrades rather than fails** (`transfer::model::read_bundle` brings an unknown enum value to the default and counts it), and runs in one transaction. An added variant does not bump `FORMAT_VERSION`. → "Import, export and copying out".
- **Folders and their board geometry are separate**: `Note` and `Folder` carry no coordinates, since `transfer::Bundle` deserialises both. On import a folder is matched by name in its space, and a `folderId` the file did not carry is dropped. → "A folder travels; its coordinates do not".

### The IPC boundary

- **The IPC surface is generated.** A command is annotated `#[tauri::command]` and `#[specta::specta]`, added to `collect_commands!` in `lib.rs`, then `npm run bindings`. Specta refuses `usize`/`i64`, hence `u32`. Not wired as a `#[test]`: on Windows the test exe cannot load `WebView2Loader.dll`. → "IPC boundary".
- **Calls return a Result**, which repositories `unwrap()` into an `IpcError`. Only `core/data/` and `core/ipc/` call a generated command.
- **Repositories are the data-source seam**: plain `providedIn: 'root'` classes, no interface, substituted by class in specs. `NotesRepository` has no method returning a raw list — one would invite re-filtering on the front.
- **Errors are codes.** A new `ErrorCode` variant needs `CODE_KEYS` (`error-notifier.service.ts`, both locales) and `IPC_ERROR_CODES`; an unknown command rejects with a plain string, so `IpcError.code` can be `null`. → "Error contract".
- **A conversion exists only where the wire shape differs** (`note.mapper.ts`: dates, the patch). A `NotePatch` field left out is untouched, which `#[specta(optional)]` allows; `null` would overwrite. → "Serialisation contract".
- **A list sends previews; the body is read by id.** `query_notes` and `board_view` cut bodies to `PREVIEW_LINES` as their last pass; ⚠️ `NotesStore.openNote` always reads with `get_note`, and a copy or fill goes through `NotesRepository.whole`. → "A list sends previews, and the body is read by id".
- **The native side asks for an action** through one event carrying a generated `GlobalAction`, and constants cross by `.constant(…)` (`APP_METADATA`, `DEFAULT_SHORTCUTS`, `FIELD_NAME_PATTERN`, `PREFERENCES_FILE`). → "The downward direction: events".
- **A rule lives on one side.** A checklist's Markdown is Rust's (`copy_text`); `isVariableName` builds its pattern from `FIELD_NAME_PATTERN`, held to `is_field_name` by one test.
- **CSP is on.** `ipc:` and `http://ipc.localhost` must stay in `connect-src`; fonts are self-hosted; nothing may compile code at runtime (`new Function`).

### Front-end state

- **Zoneless, `OnPush`, signals.** Writable signals in a store are private (`_x`) behind `.asReadonly()`; derived state is `computed()`. Writes are not optimistic: persist, adopt the answer, bump.
- **Nothing reloads a view by hand.** Every writer bumps `NotesRevision`; the canvas and the board both read it.
- **A `computed` feeding a `resource` needs an `equal` comparator**, or a fresh literal fires a query on every clock tick. `resource.value()` throws in error: read behind `hasValue()`.
- **`NotesQueryStore.view` is a retained `linkedSignal`**: read everything through it, and read it first in an `&&`.
- **Time comes from `ClockService`**: a `new Date()` inside a `computed()` freezes it. In specs, fake `Date` (or `setTimeout`/`clearTimeout`) only — a bare `useFakeTimers()` fakes `requestAnimationFrame` and hangs `whenStable()`.
- **The canvas stores, one way**: `NotesQueryStore` (which notes), `NoteSelectionStore` (which one is pointed at), `NotesStore` (the note itself), `NoteBatchStore`, `UndoStore`. One method writes a note's fields (`applyPatch`, with an exhaustive `UNCHANGED` table). → "State".
- **Creating a note writes nothing** until it is worth keeping; ⚠️ `draftMaterialisation` holds the **promise** of the write, or one close creates two notes. → "Creating a note writes nothing".
- **The editor keeps local drafts** keyed on the note id (and the restore counter), committed on blur and on every closing path.
- **⚠️ TipTap is its own chunk**, behind `@defer`: query the rich editor by template reference, never `viewChild(RichTextEditorComponent)`. Its Markdown escaping is ours (`escapeMarkdownText`), or `{{db_host}}` is stored as `{{db\_host}}`.
- **`null` space means "all spaces"**, a choice and not a loading state. Deleting a space needs a refuge; there is no one-argument variant.
- **The undo banner and the undo record differ**: the timer hides `banner()`, `Ctrl+Z` reads `last()`. `Reversible` is exhaustive.
- **Nothing corpus-wide runs before saying what it touches** (tag changes, emptying the trash, deleting a library): the count comes from the back end, and the confirm button is not the trigger.
- **Sections are exhaustive and local-day based**: `tzOffsetMinutes` travels with the query, with the sign flipped. A search or a facet switches to one flat `results` section; a quick filter does not. The search is debounced 150 ms.
- **"À trier" is a note with a deadline**, set at the end of the local day (`endOfLocalDay`).
- **`pinnedFirst` hoists pinned notes in both shapes of view**; only the palette sends `false`.
- **Every library operation reports, including when it changed nothing** (`file.importedNothing`), through `StatusNotifier` under the titlebar — the menu closes on the click.
- **The File menu owns its entries** as an array, in screen order; the trash and tag management deliberately live next to the canvas instead.
- **A startup initialiser injects everything before its first `await`** (`startApplication()`): an `inject()` after one fails with NG0203 and a black window.

### The canvas, the board and folders

- **A card is not a `<button>`**: its click surface (`.card-open`) is a layer underneath, and the card is `pointer-events: none`. → "A card is two layers".
- **⚠️ HTML5 drag & drop does not reach this WebView** (`dragDropEnabled` stays on for the native file drop): every drag is pointer events, doubled by a keyboard twin. A dropped file is a native event (`FileDropService`).
- **The board dims, it never narrows**, and every gesture commits on `pointerup` alone, snapped to `GRID_PX` (the background's lattice), written as one debounced `save_board_layout`. → "The board: the second view of a space".
- **A card flows inside a zone and is placed outside one**: a `note_positions` row means "loose", and `file_many` deletes it. `store::board::geometry` is a read that writes.
- **Opening a folder is one state** (`FoldersStore.activeFolderId`); inside, the view is a flat grid, and Escape falls through selection → search → folder, asking `hasUserFilters`. → "Descending into a folder".
- **The library rail is the navigation**: while it shows, the two switchers leave the topbar. `FoldersStore` loads every space's folders. → "The library rail".
- **The canvas keyboard is one table** (`CANVAS_KEYS`): the sheet is derived from it, and a `run` answers whether it acted.

### Preferences, shortcuts and the window

- **A preference is applied on a button** through `SettingsDraftStore`; only the theme and density preview. The native services read `SettingsStore`, never the draft, in effects built with an explicit injector. A setting is one line (`SettingsStore.setting`). → "Preferences".
- **A global shortcut is first come, first served** across the machine; the loser gets no error, so `set_global_shortcuts` answers what it could not take and the front says so. The defaults are Rust's (`DEFAULT_SHORTCUTS`), registered before the front starts. → "Shortcuts: two vocabularies, two storage paths".
- **A shortcut is captured, not typed**: `KeyboardEvent.code` for a global one (the position), the printed key for a canvas one; a global one needs a modifier.
- **The window's geometry is remembered without `VISIBLE`** (`WINDOW_STATE_FLAGS`), and the window is declared hidden, then shown from `setup`. → "Window geometry".
- **A silenced update is a version written down** (`skippedUpdate`); the About dot reads `UpdateStore.hasPendingUpdate`. → "Application updates".
- **The changelog is baked into the binary** (`include_str!`), with a thin grammar a test holds; its newest section is generated by `release.yml`.

### i18n, accessibility and theming

- **Translation keys, not strings**: code returns `{ key, params }`, and a string goes into **both** `src/app/core/services/i18n/translations/fr.json` and `en.json`. No user-visible string in Rust. → "i18n".
- **⚠️ Transloco replaces an unknown `{{name}}` with nothing**: no translated string can carry a snippet's `{{fields}}`. The application's name is `{{app}}`, a sibling key the loader adds.
- **A count is an ICU plural read by our own transpiler** (`plural-transpiler.ts`); ⚠️ `transloco-messageformat` breaks the CSP. Tiny grammar; `=0` written out where zero reads badly.
- **Syntax highlighting is highlight.js in one module** (`notes/ui/code-viewer/highlighter.ts`, grammars imported one by one), and its theme is global (`_code-theme.scss`): `[innerHTML]` carries no `_ngcontent`.
- **⚠️ CSS variables stay on `:root` in `src/styles/styles.scss`**: in a component's SCSS, `:root` never matches. The `--*-rgb` triplets are comma-separated. A parent's CSS cannot reach a child component: pass a custom property. → "Theming".
- **The light theme has its own amber**, three variables for three jobs, and dark stays the base (no white flash). `scripts/palette.test.mjs` holds every text colour to 4.5:1. `tint-badge` is for a state, `hue-badge` when the hue is the information.
- **Accessibility is linted, except what the linter cannot see**: contrast, a 24×24 target (`hit-target`), and a destructive control red at rest (`destructive`). → "Accessibility".
- **A modal is a shell** (`DialogComponent`, `shared/layout/dialog/`): the rung in `dialog.model.ts` is both the `z-index` and the Escape priority. → "The modal frame".

### Tests

- **Specs substitute repositories by class** through `provideAppTesting()`; the fakes keep `implements Pick<…, keyof …>` and reimplement no rule Rust owns. `AppWindowService` and `FileDialogService` are always faked. → "Testing".
- **⚠️ The e2e runs share one process, one database and one preferences file.** The profile is wiped once, before wdio starts; each spec file seeds its own preconditions, and the numeric prefix is the run order. `reopenSession()` is not a restart. → "One application, every spec file".
- **The e2e harness is a build flavour** (`e2e:build`: `tauri.e2e.conf.json`, the `e2e` feature and the polyfill, which go together), with its own identifier and so its own profile.
- **⚠️ A scenario waits on a condition, never a duration** (`eventually` in `support/app.ts`); a `browser.pause` stays only for "nothing happened", marked `deliberately`, which `scripts/e2e-waits.test.mjs` checks. Address controls by `data-testid`: the suite switches language.

## Design reference

`docs/scratch-mockup-v2.html` and `docs/scratch-folders.html` are static HTML/CSS mockups of the intended UI (the canvas; folders and the board). Visual references, not code to run or import.
