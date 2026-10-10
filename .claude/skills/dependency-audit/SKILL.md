---
name: dependency-audit
description: Use to audit Rust and npm dependencies for vulnerabilities, licences and outdated versions, or before adding a dependency. Reports only; updates need user approval.
---

# Dependency audit

- Rust (from `src-tauri/`): `cargo deny check advisories` — exactly what CI `security.yml` runs, and all `deny.toml` configures. A bare `cargo deny check` also runs licences, bans and sources, which have no configuration here: their failures are noise, and GPL-3.0 compatibility is a release-time review. `cargo-deny` is not part of the toolchain: if it is missing, say so rather than installing it. `cargo tree -d` for duplicates.
- npm (root): `npm audit --omit=dev --audit-level=high`, `npm outdated`.
- Adding a dependency: justify it, check maintenance, licence and size, pin like its siblings, confirm CSP and permissions are unaffected. Prefer none.
- Do **not** update versions during an audit. Angular, Tauri, specta/tauri-specta (`=` pinned rc) and the Rust toolchain move only in deliberate, separate commits.
- Report: advisory id, crate/package, reachable or not, suggested action.
