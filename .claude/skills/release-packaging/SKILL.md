---
name: release-packaging
description: Use when building the desktop executable/installer, preparing a version bump or a release (cargo-packager packages, signatures, updater, release workflow, changelog).
---

# Release and packaging

- Local build: `cargo build -p app --release` → `target/release/devnotes`. Packages: `cargo install --locked cargo-packager@0.11.8 cargo-generate-rpm@0.21.1`, then `cargo packager -p app --release -f nsis` (Windows) or `-f deb,appimage` and `cargo generate-rpm -p crates/app` (Linux); it builds nothing. Configuration: `[package.metadata.packager]` and `[package.metadata.generate-rpm]` in `crates/app/Cargo.toml`; the CI steps are `.github/actions/package`.
- Signatures: minisign, by `cargo-packager` with `CARGO_PACKAGER_SIGN_PRIVATE_KEY[_PASSWORD]` (the release secrets `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`, the same key since before 1.0.0, D26). The updater installs the NSIS installer and the AppImage only; `latest.json` is `scripts/latest-json.mjs`, each platform with its `format`. `packages.yml` proves install and update 1.0.0 → 1.0.1 on both OSes, an altered package refused, with a key made for the run.
- Release is a `workflow_dispatch` (`.github/workflows/release.yml`), steps in `docs/releasing.md`: bump `[workspace.package] version` in the root `Cargo.toml` and `Cargo.lock` (`cargo update --workspace`) — the only two files that carry it, checked by `scripts/version.mjs` — merge to `main`, CI green, Actions → Release with `dry_run`, read the notes, run again.
- Release notes come from PR labels; PR bodies need `Closes #N`. The newest `CHANGELOG.md` section is generated; do not hand-edit it.
- A version on its own branch (`dev/x.y.z`): PRs are squashed into it, `main` is merged into it to catch up, and it reaches `main` by a merge commit (`git merge --no-ff`), never a squash — the notes read what that merge brings. From the branch, `node scripts/release-notes.mjs generate --version x.y.z` prints what the version holds so far and writes nothing.
- Never commit signing keys (`*.key`, `*.pfx`, `.tauri/` are ignored). Never trigger a release, tag or push unless the user asks.
- Before a release: `verify-change` (all rows), then `dependency-audit`.
