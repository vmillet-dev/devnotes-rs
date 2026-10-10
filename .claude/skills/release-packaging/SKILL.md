---
name: release-packaging
description: Use when building the desktop executable/installer, preparing a version bump or a release (Tauri bundle, updater, release workflow, changelog).
---

# Release and packaging

- Local build: `npm install`, `npm run build`, `npm run tauri build` → `src-tauri/target/release`. Rust only: `cargo build` in `src-tauri/`.
- Release is a `workflow_dispatch` (`.github/workflows/release.yml`), steps in `docs/releasing.md`: bump the version in `src-tauri/Cargo.toml`, `package.json`, `package-lock.json`, `Cargo.lock` (`cargo update -p devnotes`) — all four must agree — merge to `main`, CI green, Actions → Release with `dry_run`, read the notes, run again.
- Release notes come from PR labels; PR bodies need `Closes #N`. The newest `CHANGELOG.md` section is generated; do not hand-edit it.
- A version on its own branch (`dev/x.y.z`): PRs are squashed into it, `main` is merged into it to catch up, and it reaches `main` by a merge commit (`git merge --no-ff`), never a squash — the notes read what that merge brings. From the branch, `node scripts/release-notes.mjs generate --version x.y.z` prints what the version holds so far and writes nothing.
- Never commit signing keys (`*.key`, `*.pfx`, `.tauri/` are ignored). Never trigger a release, tag or push unless the user asks.
- Before a release: `verify-change` (all rows), then `dependency-audit`.
