---
name: add-feature
description: Use when adding or changing a DevNotes feature end to end (core service + UI call + screen + tests + docs). Not for pure bug fixes (bug-investigation) or schema-only work (database-migration).
---

# Add a feature

1. Read the matching section of `docs/architecture.md` (headings are named in `CLAUDE.md`), the plan's section for the screen, the nearest existing feature of the same shape, and the screen's Angular twin in `reference/angular/` (same look, same behaviour). Copy the layout; do not invent a new one.
2. Decide the side: data processing, filtering, rules and validation are the **core** (`crates/engine/src/<feature>/{model,store}.rs`, service functions in `<feature>.rs`). The interface describes and displays.
3. Schema change? Run `database-migration` first.
4. The call: one function in `crates/ui/src/api/<feature>.rs`, the only place that names the engine, returning a `Pending` from the right `Backend` queue (`db` for anything touching the library or the index, `compute` for pure work, `spawn` for the network). A new `ErrorCode` gets its sentence in the UI's exhaustive `match` and its key in `fr.json` **and** `en.json`.
5. The screen: run `egui-ui`. State in `AppState`, actions applied in `apply`, keys through `keys::dispatch`, widgets from `widgets/`, colours and gaps from `theme::tokens`, every string a key in both locales.
6. Something asked of the OS (a dialog, the clipboard, the tray)? Run `platform-integration`.
7. Sealed data (titles, bodies, names, attachments) touched? Run `devnotes-security-review`.
8. Tests with the code: Rust inline or `crates/engine/tests/`; kittest over a real core (`testing::backend`/`unlocked`); an e2e scenario if a user journey changes (`desktop-e2e-testing`).
9. Update `docs/architecture.md` and, only if a new trap exists, one line of `CLAUDE.md`.
10. Finish with `verify-change`, including a look at the window. Report what ran and what did not.
