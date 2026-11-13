# CLAUDE.md

Guidance for Claude Code in this repository. `docs/architecture.md` is the detailed reference (headings quoted as → "Heading"): read the relevant section before a structural change and keep it in sync. Not this file, not the README. The decisions taken for the egui application are § 11 of `docs/egui-migration/egui-migration-plan.md` (D1–D33); a decision is changed there first, never silently in code.

## Project

DevNotes — desktop notes app for developers, in **Rust alone**: an **egui** window over a core, in one process (since 1.0.0; no WebView, no IPC). The core is **`crates/engine`** (SQLite via Diesel, vault, the in-memory index, HTTP, tools); the interface is `crates/ui`, the binary `crates/app` (`devnotes`); beside them `text` (the editing core), `richtext`, `drawing`, `diagram`, `i18n` and `platform` (what is asked of the OS). Notes are snippets (body + language + `{{fields}}`), Notes (Markdown, written formatted), todo lists, Calcul sheets (one result a line) or drawings; they live in spaces and folders, with search, tags, a canvas and a board, trash with undo, revisions, bulk actions, attachments, import/export, a palette, requests, tools and spaces. Libraries are **encrypted**, several, each behind its own passphrase.

The Angular application is no longer in the tree: it stays whole at the tag `angular-reference` (its `src/` and `e2e/`; `git show angular-reference:src/app/…`, D32). A screen that existed there keeps its look and behaviour (plan § 5); its captures are the files of the `angular-reference` pre-release.

Principles:

- **Data processing belongs to the core** (`crates/engine/src/notes/`): `query_notes` returns a ready-to-render `NotesView`; the UI filters, sorts and groups nothing. UI exceptions: relative-time formatting, syntax highlighting, layout, pure UI.
- **Rust is feature-first**: `crates/engine/src/<feature>.rs` = service functions (validate, lock, delegate, translate error); `<feature>/model.rs` = types and rules (no Diesel, no UI); `<feature>/store.rs` = SQL (no rules). Root keeps `error`, `db`, `host`, and library plumbing (`vault`, `libraries`, `layout`, `backup`, `recovery`). → "Feature-first, not layer-first".
- **One rule, one definition, in the core** (D10): what a `{{field}}`, an HTTP variable or a heading is is decided in `engine`; the UI and the editing crates take the result as an argument.
- **The UI tree is the shape of the screen** (plan § 3.2): `ui/src/{notes,http,tools,titlebar,banners,gate}` draw; `state/`, `services/`, `api/`, `backend.rs` hold what has no place on screen; `widgets/` crosses areas and reads no global state. Only `ui/src/api/` names engine functions.
- **Clean code, KISS — the code is maintained by a human.** Concise but plainly readable: the simplest design that works, small functions with one job, names that say what things are, early returns over nesting, no abstraction (trait, generic, macro, layer) before its second real use, no cleverness to save a few lines; delete rather than add when both fix it. Rust idiom: `rust-guidelines`, where the project's rules win; egui: `egui-ui`.
- Language: English for code and comments. **Comments record decisions, not narration**: only a load-bearing ordering, platform trap, non-obvious invariant, or a rejected approach; one or two lines, ~10 % of a file at most; no issue numbers; ⚠️ only for traps that cost a day.

## Commands (repo root)

- Run: `cargo run -p app -- --profile <dir>` (the whole profile, logs and window geometry in `<dir>`; without it, the installed profile `com.devnotes.app`).
- Cargo workspace: `cargo check|build|test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` (⚠️ `--all-features` is what compiles the `e2e` flavour), `cargo fmt --all --check`, `cargo bench -p engine` (not in CI; `autobenches = false` is load-bearing; → "Benchmarks"). Every external dependency is declared once, in the root `[workspace.dependencies]`; a crate takes it with `dep.workspace = true` and adds only its own features (D33). Toolchain pinned in `rust-toolchain.toml`; `unsafe_code` forbidden in every crate (D7), `clippy::pedantic` denied.
- Node is only a build tool: `npm install` · `npm run lint` (Prettier over Markdown, JSON, YAML; `format` writes) · `npm run test:scripts` (⚠️ test files are named one by one in `package.json`; add new ones there) · `cargo run -p app --example icons` (writes `crates/app/icons/`; ⚠️ small `.ico` sizes come from `icons/source/`) · `npm run prettier` (copies the pinned Prettier into `crates/text/prettier/`, the formatter compiled in). The request-script libraries are bundled by `crates/engine/sandbox/build.mjs`, from that folder's own `package.json`.
- E2E: `cargo build -p app --features e2e --target-dir target/e2e`, then `cargo test -p e2e` (the driver is `e2e/` at the root; all scenarios, one after the other; `--test 01_first_launch` for one; `cargo test --workspace --exclude e2e` otherwise). `01`–`35` are the WebdriverIO scenarios ported (plan § 8.3, Annex D), the rest are this application's own. → "The scenarios, on the assembled window".
- Release is a `workflow_dispatch` after a version bump; the version is `[workspace.package] version` in `Cargo.toml` alone (D27), so a bump touches `Cargo.toml` and `Cargo.lock` (`cargo update --workspace`); `scripts/version.mjs` refuses a release where they disagree. `release.yml` builds, packages (`cargo-packager`: NSIS, AppImage, deb; rpm by `cargo-generate-rpm`) and signs; `packages.yml` installs and updates them (plan § 4.3). ⚠️ A version branch (`dev/x.y.z`) reaches `main` by a merge commit, never a squash: the release notes read what the merge brings. → "Releasing", `docs/releasing.md`.

## Things that will bite you

### Libraries, vault, disk

- ⚠️ **Nothing answers until unlocked**: `Db = Mutex<Option<Library>>` is empty until `vault::unlock`; services answer `Locked`. → "Encryption at rest".
- **Sealed vs plain**: titles, bodies, sources, items, names, field values, attachments are sealed; what SQL filters/sorts/joins on (tags, instants, ids, `kind`, `language`, `priority`) is not. Never SQLCipher. → "What is sealed, and what is not".
- **Passphrase wraps the key** and is `zeroize`d before a service returns. ⚠️ Changing it re-wraps every retained backup's `vault.json`. The 12-char floor applies only when _choosing_ a phrase, not unlocking. → "Changing the passphrase", "The floor".
- ⚠️ An open library answers `Library::directory()`; `libraries::open_directory` is only for when none is open. Names in `layout.rs` are the address of an installed library — never rename. → "Several libraries, and the registry beside them".
- **Switching library resets the library scope of the UI** (plan § 3.3.8); neither the open nor the last library can be deleted.
- **Two `preferences.json`**: `devnotes.notes.*` is per library, the rest per application (`automaticBackups` stays application-level, read by the core itself). → "Two scopes of preference, one prefix apart".
- **Profiles**: the binary defaults to `com.devnotes.app`, the packages' identifier (`com.devnotes.app.dev` until 1.0.0, D16); ⚠️ never two processes on one profile — the in-memory index assumes it is the only writer (D16).
- **Files are written durably** — staged, flushed, renamed, the folder flushed on Unix — through `engine::durable` (registry, key file, export, saved response, restore, attachments) or its twin `platform::durable` (preferences, window geometry).
- **Backups**: launch copy via `VACUUM INTO` before sweeps, key file and `attachments/` hard-linked, ≤ 1/day; restore empties the `Mutex` and the index first and sets files aside in `replaced/`. Every open runs `db::quick_check` before migrations; damaged libraries go through `recovery::set_aside`. → "The copies, and putting one back", "The gate".
- **Attachments** live in `attachments/` under `model::stored_name` (filters imported ids); write the file, then the record. → "Attachments".

### Rust core

- ⚠️ **A data function takes `&Engine` and runs on the engine's one db thread**, in order (`lock` asserts it in debug); a network one takes `Arc<Engine>` and reaches the queue through `db::run`; a send writes only into the library it read from (`Library::number`). Nothing in `crates/engine` names egui, eframe or Tauri. One lock serialises everything. ⚠️ HTTP sends and WebSockets never hold it; `open_library` and a restore close every socket and cancel sends and runs. → "Persistence (Rust)", "Who holds the lock", "Sending a request, and its answer", "A WebSocket".
- HTTP history masks before sealing; the cookie jar is sealed whole and matched in Rust. → "The history of what was sent", "How a request travels, and the cookie jar".
- A module holding service functions is `pub`, the rest `pub(crate)`.
- **Migrations are append-only** (`crates/engine/migrations/`); `db/schema.rs` is hand-written, so add a column in both. `PRAGMA foreign_keys` is per connection (`db::configure`); `db::iso8601` always writes milliseconds. → "Persistence (Rust)", "How far back an upgrade reaches".
- Every read filters `deleted_at IS NULL`; trash retention 30 days. `updated_at` moves only when the user edited the note (deleting a space, retag, restore, undo, field fill, priority never touch it). → "The trash, and undoing a deletion".
- **Search is Rust** (`view::fold` lowercases and strips accents) and reads the in-memory index (`index::NoteIndex`): `query_notes`, `board_view` and `list_tags` open no sealed column. ⚠️ A service function that writes notes, tags, items, field values, attachments, folders or globals goes through `Engine::write` and declares its `index::Changed`. **Tags**: `notes::model::normalize_tags` only; renaming onto an existing tag merges. **Batches** answer what they changed and undo exactly that. → "Multiple selection and bulk actions".
- Side tables read by bound ids up to `BIND_AT_MOST` (500), by subquery past it. Revisions: last 20 bodies, never for checklists, never exported. → "Revisions: the body before the edit".
- **Fresh install seeds samples in one transaction.** → "The first launch". **Input is validated in the model**, before locking. → "Input validation".
- **`{{field}}`** is decided in `notes::placeholder` alone (`[A-Za-z0-9_-]`); values in `note_placeholders`; global variable = proposed default, never copied. → "`{{fields}}` in a snippet".
- ⚠️ An HTTP request's `{{name}}` is `http::variables`, never `notes::placeholder`: resolved in Rust at send from request › folder › collection › environment › globals; a name set nowhere refuses the send. → "Variables, and where each resolves from".
- **Kinds**: a todo list has items, not a body; only a snippet has a language; a Note is Markdown (`notes::markdown::plain` for list previews); a Calcul sheet is computed by Rust (`notes::calc`), its results never stored. ⚠️ A Dessin's body is its Fabric scene alone; its two pictures are in `drawing_pictures`, opened only where shown (a card asks by id, a Note embeds `devnotes:drawing/<id>`): a list decrypts every body. → "The five kinds of note", "A Calcul note: one result a line", "A Note is written formatted, and stored as Markdown", "A Dessin: a scene, and its pictures beside it".
- **Folders** nest inside one space, every rule in `folders::model::Lineage`; deleting hands notes and sub-folders to the parent; board geometry is separate from `Note`/`Folder`. Pinning is the only space order. → "Folders inside a space", "A folder travels; its coordinates do not", "Managing spaces from the switcher".
- **Tools**: a pure Rust function each, called on the `compute` queue; `result` is an output, never an input; secrets are plain values. → "The tools: one contract…".
- ⚠️ **The identity tool's lists are files, not constants**: `crates/engine/resources/identity/` is read through `Paths::resources` and checked when read (`tools/identity/data.rs`); a missing or damaged list is an answer, never a crash. ⚠️ The names are **one list** (`names.txt`): never sort people's names by origin or by country — not in data, file names, order, code or tests. A new country is a file of places there, named in `data.rs`. → "The identity tool is that design…".
- ⚠️ **Request scripts run in the app relaunched as a script host** (`--devnotes-script-host`, read in `main` before any window): QuickJS, one host per request, a phase at a time; `pm.sendRequest` is sent by the parent. What `require` finds is bundled by `crates/engine/sandbox/build.mjs` into the committed `sandbox/dist/`. → "Request scripts, in a process of their own".
- **JSON explorer/visualiser**: inputs only; Rust owns the model (`explore_json`, nesting capped at 256, node/row caps). → "Exploring a JSON snippet", "The JSON visualiser's model".
- `closed_enum!` declares enums once. **Imports degrade rather than fail**, in one transaction; an added variant does not bump `FORMAT_VERSION`. → "Import, export and copying out".
- App metadata is read from `Cargo.toml` at compile time; the changelog is baked in (`include_str!`).

### Interface (egui)

- ⚠️ **The UI thread does no I/O, crypto or waiting** (D11, D12): every call goes `ui::api` → `Backend` → `Pending`, polled each frame — `db` (the core's one ordered queue) for anything touching the library or the index, reads included; `compute` for pure work; `spawn` for the network. Never `block_on`, never a lock taken while drawing. Only exception: a pure core function whose input the screen bounds, under 1 ms, still called through `api/`. The one wait allowed is draining the db queue at exit (D19). → "Backend and events".
- **State and view** (D13): drawing reads a screen's state (`&…State`) and may change its view (`&mut …View`: text typed, menus, scroll, gestures); everything else is an `Action`, applied by `AppState::apply` at the end of the frame. → "Drawing, state and actions".
- **One keyboard order** (D14): IME → focused widget (`consume_key`) → `keys::dispatch` (modal, area, application). No screen reads shortcuts on its own; bindings are tables. → "The keyboard".
- **Every cache of library content lives in `LibraryCaches`** (D17), emptied with egui's widget memory when the library changes; not wiped byte by byte (in the threat model).
- **Anything longer than ~100 rows is virtualised** (`widgets::list`, the canvas rows, the code editor); a costly `LayoutJob` is cached; nothing costly runs in the frame. → "Rendering and virtualisation".
- **Widgets go through `widgets::interact`** (≥ 24 × 24, role and name, `author_id` = the reference's `data-testid`, focus ring); `egui::Ui::interact` is refused elsewhere by `disallowed-methods`.
- ⚠️ **The passphrase is typed into `gate::passphrase::PassphraseField`, never egui's `TextEdit`** (which keeps an undo history in egui's memory); the text is a `Zeroizing<String>` handed to the core and wiped. ⚠️ **eframe persistence stays off**: it would write egui's state, text fields included, in clear.
- ⚠️ **Installing an update is the last thing the process does** (Windows ends there): everything is written and the library closed before. → "Closing is not quitting", "Application updates".

### Core and interface

- **Errors are codes**: `AppError { code, params, detail }`; the UI translates by an exhaustive `match` on `ErrorCode`, so a new code does not compile until it has its key in both locales; `detail` goes to the log, never to the screen.
- `NotePatch`: a field set to `None` is untouched.
- **Lists send previews** (`PREVIEW_LINES`); ⚠️ opening a note always reads it whole by id (`get_note`). → "A list sends previews, and the body is read by id".
- No user-visible string in Rust outside the translation files (`crates/i18n/translations/{fr,en}.json`): code returns a key and its params. ⚠️ Never interpolate user content (a snippet's `{{fields}}`) into a translation template.

### Behaviours every screen keeps

Read at the `angular-reference` tag; the egui mechanism is in the plan (§ 3.3–3.12).

- Writes are not optimistic: persist, adopt the answer, bump the revision; nothing reloads a view by hand. Time comes from an injectable clock.
- Creating a note writes nothing until worth keeping. Editor drafts are keyed on note id and committed on blur and every closing path. → "Creating a note writes nothing".
- No router: areas, list view and board are state; an area keeps its state when not shown. Area keys are read by physical key (AZERTY). → "No router".
- `null` space = "all spaces". Nothing corpus-wide runs before saying what it touches (count from Rust). Sections are exhaustive and local-day based; search debounce 150 ms; "À trier" ends at `endOfLocalDay`; `pinnedFirst` is the user's choice.
- Every library operation reports through the status notifier, even when nothing changed.
- A card is two layers (→ "A card is two layers"). Every drag is a pointer gesture plus a keyboard twin. The board dims, never narrows; gestures commit on release, snapped to `GRID_PX`, saved debounced. Escape falls through modal → overlay → selection → search → up a folder level. The canvas keyboard is one table (`CANVAS_KEYS`); arrangement is the library's (`devnotes.notes.arrangement`). → "The board: the second view of a space", "Descending into a folder".
- A preference is applied on a button (only theme and density preview before Apply). Global shortcuts are first come, first served and stored by W3C `code`. The window starts hidden and is shown when ready; its geometry is remembered without visibility. A silenced update is a version written down (`skippedUpdate`). → "Preferences", "Shortcuts: two vocabularies, two storage paths", "Window geometry".
- Theming: `ui::theme::tokens` (copied from the reference's `styles.scss`), read through `theme::palette(ctx)` / `theme::spacing(ctx)`; no colour, gap or duration literal elsewhere. Dark the base, light with its own amber; text ≥ 4.5:1, held by a Rust test. Accessibility: 24×24 targets, destructive controls red at rest, modals stacked by the rungs of `dialog.model.ts`. → "Theming", "Accessibility", "The modal frame".

### Tests

- State is tested against the real core on a temporary library; only the platform (dialogs, window, clipboard, notifications, clock) is faked, and no fake reimplements a core rule. → "Testing".
- ⚠️ An e2e scenario runs alone on a virgin profile, seeds its own state, waits on conditions (`eventually`), never durations, and addresses controls by `author_id` (the old `data-testid` names). → "One application per run, one run per scenario".

## Tickets and pull requests

- **A pull request's title is its changelog entry** (`scripts/release-notes.mjs`), and a ticket's title is where it comes from: both say **what changes, factually and briefly** — `<Area>: <what now happens>`, the area in one or two words (a head of three words or more before `: ` is all the notes keep; ⚠️ never a commit kind — `CI`, `Docs`, `Test`, `Build`, `Fix`, `Security`… is read as a prefix and dropped), then the control, the action, the result; ≤ 70 characters. No figure of speech, no implementation detail, no « of our own ». ✗ « Search the code through a bar of our own » → ✓ « Code editor: new find-and-replace bar with match count and options ».
- **A ticket wears one section label**: `enhancement`, `change`, `bug` (any fix), `removal`, `security`, or `refactor`/`documentation`/`test`; a pull request that closes no ticket wears it itself. ⚠️ Without one, the release notes file it under « Under the hood » without a word.
- **A ticket's body opens on its context, then its change**: `**Context** —` what the application does today and where it was seen or who asked; `**Change** —` what will be different, in a sentence or two; then `**To do**` and `**Tests**` (naming the e2e scenario). A milestone's description lists what it changes, plainly.
- **No other software in a title, a description or a milestone**, unless the feature reads, writes or plugs into it (an import format, an extension's host): never a comparison with another product.

## Security and verification

- Never rewrite cryptography, key derivation or file formats unasked. Never log or commit passphrases, keys or note content (a panic hook logs location and thread, never the message — D18). eframe persistence stays off. `.claude/rules/security.md` holds the agent limits.
- Before saying a change is done, run the checks for what changed (`verify-change`) and report only what actually ran.

## Claude Code setup

- `.claude/skills/` is versioned. Local: `add-feature`, `egui-ui`, `platform-integration`, `database-migration`, `devnotes-security-review`, `verify-change`, `desktop-e2e-testing` (the `e2e/` driver), `backup-restore`, `bug-investigation`, `release-packaging`, `marketing-site`, `performance-profiling`, `dependency-audit`. Vendored, unmodified and pinned in `.claude/skills/PROVENANCE.md`: `frontend-design`, `webapp-testing` (`anthropics/skills`); `seo`, `core-web-vitals`, `accessibility` (`addyosmani/web-quality-skills`); `rust-guidelines` (`microsoft/rust-guidelines`, its `SKILL.md` ours). A vendored skill is generic: where it disagrees with this file or a local skill, the project wins. `frontend-design`, `webapp-testing`, `seo`, `core-web-vitals` and `accessibility` are for the website. `.claude/settings.json` holds the permission rules that back `.claude/rules/security.md`; `settings.local.json`, `launch.json` and `.mcp.json` stay local.

## Design reference

`docs/mockups/scratch-mockup-v2.html` and `docs/mockups/scratch-folders.html` are static mockups of the intended UI (canvas; folders and board). Visual references, not code to import.
