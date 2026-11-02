---
name: egui-ui
description: Use when writing or reviewing the egui interface of the migration (crates/ui, crates/app, the phase 0 spikes) — screens, widgets, state, theme, kittest tests, or looking at the running egui app. The Angular front in src/ keeps angular-ui while it exists.
---

# egui UI

Sources of truth: `docs/egui-migration-plan.md` (§ 3.3–3.8 architecture, § 5 visual parity, § 8
tests), then `docs/architecture.md` as sections are ported. The look to reproduce is the Angular
application: `src/` while it exists, the `angular-reference` tag afterwards, and the reference
captures (§ 5.2).

## Rules

- **Keep it simple.** Write the plain version first; a helper, trait or generic comes at the second
  real use. Small functions, names that say what things are, early returns over nesting.
- The UI thread does no I/O, crypto or waiting: every call to the core goes `api/` → `Backend` →
  `Pending` (§ 3.3) — `db`, the core's single ordered queue, for anything that touches the database
  or the index (reads included); `compute` for pure work; `spawn` for the network (D11). Never
  `block_on`, never a lock taken while drawing. Only exception: a pure core function whose input
  the screen bounds, under 1 ms, called directly but still through `api/` (D12). Every concurrency
  rule is in § 3.3.6 of the plan; one that is not there does not exist.
- A data rule (what a `{{field}}`, a variable name, a heading is) belongs to the core: `text`,
  `richtext`, `drawing` take its result as an argument and never redefine it (D10).
- Drawing reads a screen's state (`&…State`) and may change its view (`&mut …View`: text being
  typed, editor buffers, open menus, scroll, gestures). It pushes `Action`s; state changes only in
  `AppState::apply` (§ 3.4, D13).
- Keys: a focused widget consumes what it handles; everything else goes through `keys::dispatch`
  (modal, then area, then application). No screen reads shortcuts on its own (§ 3.12, D14).
- Anything longer than ~100 rows is virtualised; a costly `LayoutJob` is cached, and any cache
  holding library content lives in `LibraryCaches` (§ 3.6, D17).
- Colours, spacing and durations come from `theme::tokens`: no hex or millisecond literal in a
  widget (§ 3.7).
- Every interactive widget has an accessible name and carries, as its AccessKit `author_id`, the
  `data-testid` its Angular twin had (§ 3.12, § 8.2).
- No user-visible string in code: an `i18n` key and its params (§ 3.11).

## Tests

- **Screen and widget**: `egui_kittest::Harness`, queried by label, role or `author_id`. Its
  accessibility check panics on an input widget without a name: keep it on.
- **Snapshot**: `Harness::snapshot` (features `wgpu` + `snapshot`), small images, default
  thresholds; accept a change with `UPDATE_SNAPSHOTS=true cargo test` and look at the image first.
- **End to end**: the assembled binary, driven through `egui_inspection` (§ 8); one scenario per
  file, a virgin profile each, waits on conditions, never on durations.

## Looking at the running app

Build with the `e2e` feature (it turns on eframe's `inspection`), run with `EGUI_INSPECTION=1`
(binds `127.0.0.1:5719`), then drive it with `egui_mcp` (`cargo install egui_mcp`, declared in
your local `.mcp.json`): widget tree, click, type, screenshot. Never in a release build: the port
has no authentication.

## References

egui 0.37 (`emilk/egui@fc9d66dda742d722eedaa8684d23f86369cf8e5f`):
`crates/egui_kittest/README.md`, `crates/egui_inspection/README.md`, the API docs on docs.rs, and
eframe's examples. Read them for egui questions; the project wins where they differ.
