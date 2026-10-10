# CLAUDE.md

Guidance for Claude Code in this repository. `docs/architecture.md` is the detailed reference (headings quoted as → "Heading"): read the relevant section before a structural change and keep it in sync. Not this file, not the README.

## Project

DevNotes — desktop notes app for developers. **Angular 22** (standalone, signals, zoneless), **Rust / Tauri v2**, SQLite via Diesel. Notes are snippets (body + language + `{{fields}}`), Notes (Markdown, written formatted) or todo lists; they live in spaces and folders, with search, tags, a canvas and a board, trash with undo, revisions, bulk actions, attachments, import/export, a quick-paste palette, and tools (JSON, HTTP, WebSocket…). Libraries are **encrypted**, several, each behind its own passphrase.

Principles:

- **Data processing belongs to Rust** (`src-tauri/src/notes/`): `query_notes` returns a ready-to-render `NotesView`. Front-end exceptions: relative-time formatting, ISO↔`Date`, syntax highlighting, Prettier, searching the tools catalogue/references, pure UI.
- **Rust is feature-first**: `<feature>.rs` = `#[tauri::command]`s (validate, lock, delegate, translate error); `<feature>/model.rs` = types and rules (no Diesel, no Tauri); `<feature>/store.rs` = SQL (no rules). Root keeps `error`, `db`, `desktop`, and library plumbing (`vault`, `libraries`, `layout`, `backup`, `recovery`). → "Feature-first, not layer-first".
- **The front-end tree is the shape of the screen**: `notes/{sidebar,header,canvas,overlays,ui}`, `tools/`, `http/`, `titlebar/`, `banners/` hold components only; everything without a place on screen is in `core/` (`model`, `data`, `state`, `ipc`, `utils`, `services/<subject>`); `shared/` crosses areas and injects nothing (except `DialogComponent`). ⚠️ `core/ipc/` stays at the root of `core/` (`bindings.ts` is written there). → "Where a file goes".
- Language: English for code, comments and UI strings in code. **Comments record decisions, not narration**: only a load-bearing ordering, platform trap, non-obvious invariant, or a rejected approach; one or two lines, ~10 % of a file at most; no issue numbers; ⚠️ only for traps that cost a day.

## Commands (repo root unless noted)

- `npm install` · `npm run tauri dev` (hot reload, regenerates `bindings.ts`) · `npm start` (Angular alone, port **1420**) · `npm run build` (run before `e2e:build`; `npm test` does not type-check) · `npm run tauri build`.
- `npm test` (Vitest, jsdom; `test:watch`, `test:coverage` 80 %) · `npm run lint` (ESLint, Prettier check, `tsc` over `e2e/`; `lint:fix`, `format`) · `npm run test:scripts` (⚠️ test files are named one by one in `package.json`; add new ones there).
- From `src-tauri/`: `cargo check|build|test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo bench` (not in CI; `autobenches = false` is load-bearing; → "Benchmarks"). Toolchain pinned in `rust-toolchain.toml`; `unsafe_code` forbidden, `clippy::pedantic` denied.
- `npm run bindings` regenerates `src/app/core/ipc/bindings.ts` from Rust signatures. `npm run icons` regenerates icons (⚠️ small `.ico` sizes come from `icons/source/`).
- E2E: `npm run e2e:build` then `npm run test:e2e` (WebdriverIO + `tauri-driver`, specs in `e2e/specs/`, against the assembled app: rebuild after any change).
- Release is a `workflow_dispatch` after a version bump (`Cargo.toml`, `package.json`, both lockfiles). → "Releasing", `docs/releasing.md`.

## Things that will bite you

### Libraries, vault, disk

- ⚠️ **Nothing answers until unlocked**: `Db = Mutex<Option<Library>>` is empty until `unlock_vault`; commands answer `Locked`. → "Encryption at rest".
- **Sealed vs plain**: titles, bodies, sources, items, names, field values, attachments are sealed; what SQL filters/sorts/joins on (tags, instants, ids, `kind`, `language`, `priority`) is not. Never SQLCipher. → "What is sealed, and what is not".
- **Passphrase wraps the key** and is `zeroize`d before a command returns. ⚠️ Changing it re-wraps every retained backup's `vault.json`. The 12-char floor applies only when _choosing_ a phrase, not unlocking. → "Changing the passphrase", "The floor".
- ⚠️ An open library answers `Library::directory()`; `libraries::open_directory` is only for when none is open. Names in `layout.rs` are the address of an installed library — never rename. → "Several libraries, and the registry beside them".
- **Switching library reloads the page**; neither the open nor the last library can be deleted.
- **Two `preferences.json`**: `devnotes.notes.*` is per library, the rest per application (`automaticBackups` stays application-level). → "Two scopes of preference, one prefix apart".
- **Half-written files are staged then renamed** (registry, key file, export).
- **Backups**: launch copy via `VACUUM INTO` before sweeps, key file and `attachments/` hard-linked, ≤ 1/day; restore empties the `Mutex` first and sets files aside in `replaced/`. Every open runs `db::quick_check` before migrations; damaged libraries go through `recovery::set_aside`. → "The copies, and putting one back", "The gate".
- **Attachments** live in `attachments/` under `model::stored_name` (filters imported ids); write the file, then the record. → "Attachments".

### Rust back-end

- ⚠️ **A command taking the lock is `async` and runs its body in `db::blocking`**; with an `AppHandle`, generic over `R: Runtime`, registered as `name::<tauri::Wry>`. One lock serialises everything. ⚠️ HTTP sends and WebSockets never hold it; `open_library` closes every socket. → "Persistence (Rust)", "Who holds the lock", "Sending a request, and its answer", "A WebSocket".
- HTTP history masks before sealing; the cookie jar is sealed whole and matched in Rust. → "The history of what was sent", "How a request travels, and the cookie jar".
- A module holding commands is `pub`, the rest `pub(crate)`.
- **Migrations are append-only** (`src-tauri/migrations/`); `db/schema.rs` is hand-written, so add a column in both. `PRAGMA foreign_keys` is per connection (`db::configure`); `db::iso8601` always writes milliseconds. → "Persistence (Rust)", "How far back an upgrade reaches".
- Every read filters `deleted_at IS NULL`; trash retention 30 days. `updated_at` moves only when the user edited the note (deleting a space, retag, restore, undo, field fill, priority never touch it). → "The trash, and undoing a deletion".
- **Search is Rust** (`view::fold` lowercases and strips accents). **Tags**: `notes::model::normalize_tags` only; renaming onto an existing tag merges. **Batches** answer what they changed and undo exactly that. → "Multiple selection and bulk actions".
- Side tables read by bound ids up to `BIND_AT_MOST` (500), by subquery past it. Revisions: last 20 bodies, never for checklists, never exported. → "Revisions: the body before the edit".
- **Fresh install seeds samples in one transaction.** → "The first launch". **Input is validated in the model**, before locking. → "Input validation".
- **`{{field}}`** is decided in `notes::placeholder` alone (`[A-Za-z0-9_-]`); values in `note_placeholders`; global variable = proposed default, never copied. → "`{{fields}}` in a snippet".
- **Kinds**: a todo list has items, not a body; only a snippet has a language; a Note is Markdown (`notes::markdown::plain` for list previews). → "The three kinds of note", "A Note is written formatted, and stored as Markdown".
- **Folders** cut one space; deleting leaves notes unfiled; board geometry is separate from `Note`/`Folder`. Pinning is the only space order. → "Folders inside a space", "A folder travels; its coordinates do not", "Managing spaces from the switcher".
- **Tools**: a pure Rust function plus a component implementing `Tool`; catalogue in `tools/catalogue/catalogue.ts`; one command per tool via `off_thread`; `result` is an output, never an input; secrets are plain signals. → "The tools: one contract…".
- **JSON explorer/visualiser**: inputs only; Rust owns the model (`explore_json`, nesting capped at 256, node/row caps). → "Exploring a JSON snippet", "The JSON visualiser's model".
- `closed_enum!` declares enums once. **Imports degrade rather than fail**, in one transaction; an added variant does not bump `FORMAT_VERSION`. → "Import, export and copying out".
- App metadata is read from `Cargo.toml` at compile time; the changelog is baked in (`include_str!`).

### IPC boundary

- **Generated**: `#[tauri::command]` + `#[specta::specta]`, add to `collect_commands!` in `lib.rs`, run `npm run bindings`; `u32`, not `usize`/`i64`. Only `core/data/` and `core/ipc/` call generated commands; repositories `unwrap()` the `Result` into `IpcError`. → "IPC boundary".
- **Errors are codes**: a new `ErrorCode` needs `CODE_KEYS` (both locales) and `IPC_ERROR_CODES`. → "Error contract".
- ⚠️ `rename_all` on an enum does not rename struct-variant fields: add `rename_all_fields = "camelCase"`.
- `NotePatch`: a missing field is untouched (`#[specta(optional)]`), `null` overwrites. Conversion only where the wire shape differs (`note.mapper.ts`). → "Serialisation contract".
- **Lists send previews** (`PREVIEW_LINES`); ⚠️ `NotesStore.openNote` always reads with `get_note`; copy/fill use `NotesRepository.whole`. → "A list sends previews, and the body is read by id".
- Native→front actions go through one event carrying `GlobalAction`; constants cross via `.constant(…)`. → "The downward direction: events". A rule lives on one side only.
- **CSP is on**: keep `ipc:` and `http://ipc.localhost` in `connect-src`; self-hosted fonts; nothing compiles code at runtime.

### Front-end state

- **Zoneless, `OnPush`, signals.** Store signals are private `_x` behind `.asReadonly()`; derived = `computed()`. Writes are not optimistic: persist, adopt the answer, bump `NotesRevision`; nothing reloads a view by hand.
- A `computed` feeding a `resource` needs an `equal` comparator; read `resource.value()` behind `hasValue()`; whole-corpus loaders go through `OneInFlight`; read `NotesQueryStore.view` first in an `&&`.
- **Time comes from `ClockService`** (no `new Date()` in `computed`). In specs fake `Date` only; a bare `useFakeTimers()` hangs `whenStable()`.
- **Canvas stores**: `NotesQueryStore` (which notes), `NoteSelectionStore`, `NotesStore` (the note; `applyPatch` is the one writer), `NoteBatchStore`, `UndoStore` (banner timer ≠ `Ctrl+Z` record). → "State".
- Creating a note writes nothing until worth keeping; ⚠️ `draftMaterialisation` holds the promise of the write. → "Creating a note writes nothing". Editor drafts are keyed on note id and committed on blur and every closing path.
- ⚠️ A `@defer`red component is imported on a line of its own (`import type` for types). There is no router: areas, date view and board are state. The notes page is hidden, never destroyed; area keys use `KeyboardEvent.code` (AZERTY). → "No router".
- ⚠️ TipTap is its own chunk: query the rich editor by template reference. On WebKit use `focusFirst` before `chain().focus()`. Markdown escaping is ours (`escapeMarkdownText`). Tab stays in the text (`codeIndent`). → "Editing a note".
- `null` space = "all spaces". Nothing corpus-wide runs before saying what it touches (count from Rust). Sections are exhaustive and local-day based (`tzOffsetMinutes` sign flipped); search debounce 150 ms; "À trier" ends at `endOfLocalDay`; `pinnedFirst` is the user's choice.
- Every library operation reports through `StatusNotifier`, even when nothing changed. A startup initialiser injects everything before its first `await`.

### Canvas, board, folders

- A card is not a `<button>`: `.card-open` is a layer underneath. → "A card is two layers". No host `(document:*)` listener per card.
- ⚠️ HTML5 drag & drop does not reach the WebView: every drag is pointer events plus a keyboard twin; file drops are native (`FileDropService`).
- The board dims, never narrows; gestures commit on `pointerup`, snapped to `GRID_PX`, saved debounced (`save_board_layout`). A `note_positions` row means "loose". Opening a folder is `FoldersStore.activeFolderId`; Escape falls through selection → search → folder. → "The board: the second view of a space", "Descending into a folder".
- The canvas keyboard is one table (`CANVAS_KEYS`). Arrangement is the library's (`devnotes.notes.arrangement`).

### Preferences, shortcuts, window

- A preference is applied on a button via `SettingsDraftStore` (only theme and density preview); native services read `SettingsStore`, never the draft. Global shortcuts are first come, first served (`set_global_shortcuts` answers what it could not take) and captured by `KeyboardEvent.code`. → "Preferences", "Shortcuts: two vocabularies, two storage paths".
- Window geometry is remembered without `VISIBLE` (`WINDOW_STATE_FLAGS`); the window starts hidden and is shown from `setup`. A silenced update is a version written down (`skippedUpdate`). → "Window geometry", "Application updates".

### i18n, accessibility, theming

- **Translation keys, not strings**: code returns `{ key, params }`; add every string to **both** `core/services/i18n/translations/fr.json` and `en.json`. No user-visible string in Rust. → "i18n".
- ⚠️ Transloco drops an unknown `{{name}}`: no translated string can carry a snippet's `{{fields}}`. Counts are ICU plurals read by our own transpiler (`transloco-messageformat` breaks the CSP).
- Prettier runs in a worker (hand it text, never code; every `{{…}}` must come back intact). Highlighting is highlight.js in one module; its theme is global. → "Formatting a snippet with Prettier", "Syntax highlighting".
- ⚠️ CSS variables stay on `:root` in `src/styles/styles.scss`; `--*-rgb` triplets are comma-separated; a parent's CSS cannot reach a child component (pass a custom property). Dark is the base; light has its own amber; `scripts/palette.test.mjs` holds text to 4.5:1. → "Theming".
- Accessibility: linted, plus contrast, 24×24 targets and destructive controls red at rest. Modals are `DialogComponent` (the rung in `dialog.model.ts` is `z-index` and Escape priority). → "Accessibility", "The modal frame".

### Tests

- Specs substitute repositories by class through `provideAppTesting()`; fakes use `implements Pick<…, keyof …>` and reimplement no Rust rule; `AppWindowService` and `FileDialogService` are always faked. → "Testing".
- ⚠️ E2E runs share one process, DB and preferences file; the numeric prefix is the run order; each spec seeds its own state; `reopenSession()` is not a restart. The harness is a build flavour (`tauri.e2e.conf.json`, `e2e` feature). ⚠️ Wait on conditions (`eventually`), never durations; address controls by `data-testid`. → "One application, every spec file".

## Security and verification

- Never rewrite cryptography, key derivation or file formats unasked. Never log or commit passphrases, keys or note content. Add Tauri permissions narrowest-first (`src-tauri/capabilities/default.json`). `.claude/rules/security.md` holds the agent limits.
- Before saying a change is done, run the checks for what changed (`verify-change`) and report only what actually ran.

## Claude Code setup

- `.claude/skills/` is versioned. Local: `add-feature`, `angular-ui`, `tauri-integration`, `database-migration`, `devnotes-security-review`, `verify-change`, `desktop-e2e-testing`, `backup-restore`, `bug-investigation`, `release-packaging`, `marketing-site`, `performance-profiling`, `dependency-audit`. Vendored, unmodified and pinned in `.claude/skills/PROVENANCE.md`: `frontend-design`, `webapp-testing` (`anthropics/skills`); `angular-developer` (`angular/skills`); `seo`, `core-web-vitals`, `accessibility` (`addyosmani/web-quality-skills`, for the marketing site). A vendored skill is generic: where it disagrees with this file or a local skill, the project wins. `settings.local.json` and `launch.json` stay ignored.

## Design reference

`docs/scratch-mockup-v2.html` and `docs/scratch-folders.html` are static mockups of the intended UI (canvas; folders and board). Visual references, not code to import.
