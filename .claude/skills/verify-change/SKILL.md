---
name: verify-change
description: Use after modifying code to choose and run the right checks (fmt, clippy, Rust and kittest tests, scripts, e2e, a look at the window) and report honestly. Use before saying a change is done or opening a PR.
---

# Verify a change

Run from the repo root; pick by what changed, and report only what actually ran.

| Changed                                          | Run                                                                                                                                                         |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/**`                                      | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --exclude e2e`                   |
| `crates/i18n/translations/*.json`                | the row above (the keys are generated and the plural corpora checked by `cargo test -p i18n`)                                                               |
| Markdown, JSON, YAML, `scripts/**`, `.github/**` | `npm run lint`, `npm run test:scripts`                                                                                                                      |
| A user journey, anything the assembled app does  | `cargo build -p app --features e2e --target-dir target/e2e`, then `cargo test -p e2e --test NN_name` for each scenario it touches (`desktop-e2e-testing`)   |
| Anything visible                                 | `cargo run -p app -- --profile <temp dir>` and a screenshot, both themes; or the kittest captures: `DEVNOTES_CAPTURES=<dir> cargo test -p ui app::captures` |
| Dependencies                                     | `dependency-audit`; the licence must be in `deny.toml`                                                                                                      |

Rules: a failing check is reported with its output, never skipped; "not run" is stated plainly, with the OS it would need. Fix the cause, not the test. `--all-features` is what compiles the `e2e` flavour. Check `git status` for stray files before concluding.
