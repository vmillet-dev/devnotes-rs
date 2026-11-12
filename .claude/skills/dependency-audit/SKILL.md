---
name: dependency-audit
description: Use to audit Rust and npm dependencies for vulnerabilities, licences and outdated versions, or before adding a dependency. Reports only; updates need user approval.
---

# Dependency audit

- Rust (from the root): `cargo deny check` — exactly what CI `security-cargo.yml` runs: advisories, licences (what GPL-3.0-only may ship, `deny.toml`), sources (crates.io alone) and bans (nothing refused by name, duplicates allowed). A new licence or source is a decision, not a line to add. `cargo-deny` is not part of the toolchain: if it is missing, say so rather than installing it, and read the job's result on the pull request. `cargo tree -d` for duplicates.
- npm (root): `npm audit --omit=dev --audit-level=high`, `npm outdated`.
- Adding a dependency: justify it, check maintenance, licence and size, pin like its siblings. Prefer none. An embedded font or icon set goes in `crates/ui/assets/README.md` with its licence file beside it.
- Do **not** update versions during an audit. egui and its family, the pinned Prettier and the Rust toolchain move only in deliberate, separate commits.
- Report: advisory id, crate/package, reachable or not, suggested action.
