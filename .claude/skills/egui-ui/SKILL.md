---
name: egui-ui
description: Use when writing or reviewing the egui interface (crates/ui, crates/app) — screens, widgets, state, theme, kittest tests, or looking at the running egui app.
---

# egui UI

Source of truth: `docs/architecture.md` (« Decisions », « Interface (egui) », « Backend and events »,
« Testing »). The look to reproduce, for a screen that existed there, is the Angular
application, whole at the `angular-reference` tag (its `src/` and `e2e/`, e.g. `git show angular-reference:src/app/…`,
D32): read a screen's component and its SCSS before changing it, and its captures.

## Rules

- **Keep it simple.** Write the plain version first; a helper, trait or generic comes at the second
  real use. Small functions, names that say what things are, early returns over nesting.
- The UI thread does no I/O, crypto or waiting: every call to the core goes `api/` → `Backend` →
  `Pending` — `db`, the core's single ordered queue, for anything that touches the database
  or the index (reads included); `compute` for pure work; `spawn` for the network (D11). Never
  `block_on`, never a lock taken while drawing. Only exception: a pure core function whose input
  the screen bounds, under 1 ms, called directly but still through `api/` (D12). Every concurrency
  rule is in `docs/architecture.md` → "The concurrency rules"; one that is not there does not exist.
- A data rule (what a `{{field}}`, a variable name, a heading is) belongs to the core: `text`,
  `richtext`, `drawing` take its result as an argument and never redefine it (D10).
- Drawing reads a screen's state (`&…State`) and may change its view (`&mut …View`: text being
  typed, editor buffers, open menus, scroll, gestures). It pushes `Action`s; state changes only in
  `AppState::apply` (D13).
- Keys: a focused widget consumes what it handles; everything else goes through `keys::dispatch`
  (modal, then area, then application). No screen reads shortcuts on its own (D14).
- Anything longer than ~100 rows is virtualised; a costly `LayoutJob` is cached, and any cache
  holding library content lives in `LibraryCaches` (D17).
- Colours, spacing and durations come from `theme::tokens`: no hex or millisecond literal in a
  widget.
- Every interactive widget has an accessible name and carries, as its AccessKit `author_id`, the
  `data-testid` its Angular twin had.
- No user-visible string in code: an `i18n` key and its params.
- **Widgets go through `widgets::interact`** (at least 24 × 24, role and name, `author_id`, focus
  ring); `Ui::interact` is refused elsewhere by `crates/ui/clippy.toml`. A label or frame that is
  not clickable gets its id with `widgets::address`.
- A secret is typed into `gate::passphrase::PassphraseField`, never `TextEdit` (whose undo history
  keeps copies in egui's memory), held in a `Zeroizing<String>` and handed over by `take`.
- egui keeps **one popup open in memory**: a sub-menu needs its own open state (`open_bool`). egui
  moves the focus with the arrows inside a menu by itself — do not add a second move. A modal's
  focus goes back to its opener only once `top_modal_layer()` is `None` (`DialogStack`).
- A panel whose colours are translucent (the banners) sits on an explicit fill: under it is the
  clear colour, transparent in a kittest render.

## Tests

- **Screen and widget**: `testing::harness()` (the application's fonts — egui's defaults are not
  loaded), queried by `author_id`. Its accessibility check panics on an input widget without a
  name: keep it on. State tests run over a real core: `testing::backend()` on an empty temporary
  profile, `testing::unlocked()` with a library created, `testing::locked()` closed for a later
  launch. **Wait on conditions, never on a number of frames**: `testing::settle(harness, …)`
  waits for the core to be idle and the window to have drawn it, and fails if the window repaints
  forever. Work the core goes on with after an answer is started through `Backend::background`,
  so that it is waited for too. No `run_steps(n)`, no `with_max_steps`: a bench with a spinner
  runs with reduced motion.
- **The whole window**: `Harness::new_eframe` / `build_eframe` over `DevNotesApp`, through the gate
  like a person (`app.rs` tests).
- **Captures, not snapshots** (D24): `app::captures` renders the window's states in both themes into
  `DEVNOTES_CAPTURES` for a look by eye beside the reference's captures; CI uploads them. Nothing
  compares pixels; a new screen adds its states there.
- **End to end**: the assembled binary, driven through `egui_inspection` (`desktop-e2e-testing`).

## Looking at the running app

`cargo run -p app -- --profile <temp dir>` and a screenshot of the window, in both themes (the
toggle is in the title bar); a test library's phrase is `testing::PASSPHRASE`, never a real one.
To drive it, build the `e2e` flavour (`cargo build -p app --features e2e --target-dir target/e2e`),
run it with `EGUI_INSPECTION=1` (binds `127.0.0.1:5719`) and attach `egui_mcp` (`cargo install
egui_mcp`, declared in your **local** `.mcp.json`, never versioned): widget tree, click, type,
screenshot. Never in a release build: the port has no authentication.

## References

egui 0.37 (`emilk/egui@fc9d66dda742d722eedaa8684d23f86369cf8e5f`):
`crates/egui_kittest/README.md`, `crates/egui_inspection/README.md`, the API docs on docs.rs, and
eframe's examples. Read them for egui questions; the project wins where they differ.
