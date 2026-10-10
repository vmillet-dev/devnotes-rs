---
name: verify-change
description: Use after modifying code to choose and run the right checks (lint, unit, Rust, bindings, build, e2e) and report honestly. Use before saying a change is done or opening a PR.
---

# Verify a change

Run from the repo root; pick by what changed, and report only what actually ran.

| Changed                                                   | Run                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `src/**`                                                  | `npm run lint`, `npm test`, `npm run build`                                                      |
| `src-tauri/src/**`                                        | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`) |
| Rust signatures of commands/types                         | `npm run bindings`, then the `src/**` row; commit `bindings.ts`                                  |
| `scripts/**`, palette, motion, focus rings, release notes | `npm run test:scripts`                                                                           |
| User journey, anything in the assembled app               | `npm run build`, `npm run e2e:build`, `npm run test:e2e` (see `desktop-e2e-testing`)             |
| Visual change                                             | `preview_start`, screenshot, console clean, light + dark                                         |

Rules: a failing check is reported with its output, never skipped; "not run" is stated plainly. `npm test` does not type-check, so `npm run build` is required for TS changes. Fix the cause, not the test. Check `git status` for stray files before concluding.
