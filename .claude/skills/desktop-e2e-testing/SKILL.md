---
name: desktop-e2e-testing
description: Use when writing, running or debugging an end-to-end scenario of the assembled egui app (crates/e2e, driven through egui_inspection). Not for screen or widget tests (kittest, egui-ui), nor the website.
---

# Desktop e2e

- Build the flavour first, every time `crates/` changed: `cargo build -p app --features e2e --target-dir target/e2e` (its own target directory: no other build overwrites it). Then one scenario a run: `cargo test -p e2e --test NN_name`. `cargo test --workspace` runs with `--exclude e2e`.
- A scenario is `crates/e2e/tests/NN_name.rs` (Annex D of the plan keeps the old numbers and names), one `#[test]` of named `step`s. It launches its own application on its own `tempfile` profile, seeds its own state, and must pass alone. A new file joins the group with the shortest run in `crates/e2e/groups.json`, or `e2e-scenarios` fails (`npm run test:scripts` checks it).
- Drive it through `e2e::App`: `wait_for`, `has`, `click`, `fill`, `type_text`, `press` (a physical key: the area keys read the position), `eventually`, `capture`, `relaunch` (a real restart through `--action quit`), `quit`. Add a method there before reaching for the protocol in a scenario.
- Wait on conditions (`eventually`), never a duration: `std::thread::sleep` is refused by `crates/e2e/clippy.toml`; the one exception, asserting that **nothing** happens, carries an `#[expect]` saying so. `Settle` waits for egui, not for the core: its answer comes frames later, which is why `eventually` reads again.
- Address controls by `author_id` (the old `data-testid` names), never by text: the language is the machine's. Missing one? Give the widget its id through `widgets::address` / `widgets::interact`.
- What the protocol cannot reach goes through the flavour's two doors: a global action is `--action <name>` from a second launch; a file dialog answers `DEVNOTES_E2E_DIALOG` (`platform::dialog::from_scenario`). A file dropped from the explorer is not injectable: go through « Joindre ».
- A failure leaves `failure.png` and the application's log in `target/e2e/scenarios/<scenario>/`: read them before changing the scenario, and decide whether the app or the test is wrong. CI uploads the same folder per platform and group.
- Never point a scenario at a real profile: `--profile` is always a temporary directory. The inspection port has no authentication; the flavour never becomes a release.
- Lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
