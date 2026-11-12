---
name: rust-guidelines
description: Use when writing, reviewing or refactoring Rust in DevNotes (every crate of the workspace) and a question of idiom comes up — naming, errors, panics, module layout, docs, performance, lints. Microsoft's Pragmatic Rust Guidelines, vendored; the project's own rules win where they differ.
---

# Pragmatic Rust Guidelines

`guidelines/` is an unmodified copy of Microsoft's guidelines (see `PROVENANCE.md`). Start from
`guidelines/checklist/README.md`, then open the guideline itself: `guidelines/<category>/M-<ID>.md`
(the category `README.md` files are mdBook include lists, and the checklist's links point at the
rendered book, not at these files). Read the `<why>` before applying a rule: the spirit counts.

## Order of precedence

1. `CLAUDE.md`, `.claude/rules/security.md`, `docs/architecture.md` and the local skills.
2. `../../../docs/egui-migration/egui-migration-plan.md` for work on the migration.
3. These guidelines.

Keep it simple: a guideline is a reason to write less, plainer code, never a licence to add a
layer. Apply it to the code you are changing; do not sweep the codebase unasked.

## Where DevNotes already decided otherwise

| Guideline                                           | Here                                                                                                                                                               |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| M-ERRORS-CANONICAL-STRUCTS, M-APP-ERROR             | Errors stay `AppError { code, params, detail }` with `ErrorCode` (→ "Error contract"); no `anyhow`, no backtraces in answers.                                      |
| M-INTEGRATION-TESTS                                 | Unit tests stay inline `#[cfg(test)] mod tests` at the bottom of the file; `tests/` holds the integration binaries, as today.                                      |
| M-SMALLER-CRATES                                    | The crates are the ones the migration plan lists (§ 3.1); a new crate is a decision, not a reflex.                                                                 |
| M-MODULE-DOCS, M-CANONICAL-DOCS                     | Comments record decisions, not narration (`CLAUDE.md`): short doc comments, no essays.                                                                             |
| M-LOG-STRUCTURED                                    | Logging stays on `log`; what matters here is the redaction part — never log note content, passphrases, keys or tokens.                                             |
| M-STATIC-VERIFICATION                               | The lint set is the project's (`unsafe_code` forbidden, `clippy::pedantic` denied); an extra lint is a proposal to the user.                                       |
| M-MIMALLOC-APPS, M-FAST-HASHER, M-TARGET-CPU        | Dependency or build changes: only measured (`performance-profiling`) and approved first. `target-cpu` does not apply to a desktop app shipped to unknown machines. |
| M-UNSAFE, M-UNSOUND, FFI                            | `unsafe_code = "forbid"` in every crate: use crates with a safe API instead.                                                                                       |
| M-INIT-BUILDER, M-DI-HIERARCHY, M-MOCKABLE-SYSCALLS | Only when the need is already there (four optional parameters, a second implementation, a test that cannot run otherwise).                                         |
