# Releasing

Before a release, pass the manual checklist (`docs/recette/recette.html`, plan § 8.5) on both operating systems.

A release is a `workflow_dispatch`. One run writes the changelog, commits it, tags, builds
every bundle and publishes — there is no step to do by hand between them, and no partial
state to clean up if it stops.

## The three steps

1. **Bump the version.** One place holds it (D27): `version` under `[workspace.package]` in
   the root `Cargo.toml`, which every crate inherits. Edit it, then `cargo update --workspace`
   writes it into `Cargo.lock`; nothing else carries a version (`package.json` has none). The
   bump touches those two files alone. Merge to `main` and wait for CI to go green — the release
   refuses to start on a commit CI has not passed.

   The workflow reads the version with `scripts/version.mjs` and stops unless `Cargo.lock`
   says the same for `app`. It checks that **before the tag exists**, so a mismatch costs a
   re-run rather than a deleted tag.

2. **Actions → Release → Run workflow, with `dry_run` checked.** It prints the notes it
   would write to the run summary, builds and signs the packages (kept as the run's
   artifacts), and changes nothing. Read the notes: this is the only moment where a misfiled
   entry is free to fix.

3. **Run it again unchecked.** It writes the `CHANGELOG.md` section, commits it, tags,
   builds the packages and publishes the release.

## A version on a branch of its own

A version that takes weeks lives on `dev/x.y.z`, while `main` goes on releasing.

- Pull requests are **squashed into `dev/x.y.z`**, each with its `Closes #42`, exactly as into
  `main`. CI runs on both.
- When `main` moves under it — a fix, a release — **merge `main` into the branch**. A fix
  already released sits behind its tag and does not come back in the version's notes.
- The branch reaches `main` by **a merge commit, never a squash**:
  `git merge --no-ff dev/x.y.z`. The notes read what that merge brings. Squashed, the version
  is one commit and one line, and the dry run says so.
- Then the three steps above, from `main`.

To read what a version holds so far, run this from its branch — it prints the section and
writes nothing:

```bash
node scripts/release-notes.mjs generate --version x.y.z
```

## Where the notes come from

Release notes are generated from the pull requests the release carries and the last tag does
not — merged into `main`, or into a version branch merged since — and what sorts them is
**labels**. A pull request an older section already lists is left out: nothing is released
twice.

An entry is the pull request's **title**, as it reads on GitHub when the release runs — so a
title says what changed for the person using the application (`Code editor: new
find-and-replace bar with match count and options`), never how it was built. A title can still
be edited after the merge, up to the release. The rule is in `CLAUDE.md`, « Tickets and pull
requests ».

GitHub propagates nothing on its own, so a pull request inherits the labels of the issue it
closes — which only works if the body says `Closes #42`. GitHub does not always record that
link for a pull request aimed at a version branch, so the description is read when it is
missing. From there:

| what decides the section                   | when                                                      |
| ------------------------------------------ | --------------------------------------------------------- |
| a label on the pull request                | always wins, and overrides the issue's for that one entry |
| the labels of the closed issue             | the ordinary case                                         |
| the `(feat)` / `(fix)` prefix of the title | when there is no label anywhere                           |
| "Under the hood"                           | when there is none of the three                           |

**A version can open on a summary written by hand.** When the pull requests alone cannot say
what a version changes — 1.0.0 changes the whole application — `docs/release-notes/<version>.md`
holds a few bullets, which the generator puts first, under « 📌 In short ». Only its `- ` lines
(and the indented lines continuing them) are read; the rest of the file is a note to its author.

Nothing is ever dropped for want of a label, and the dry run names the rule that filed each
entry — so a surprise in the summary tells you which of the four fired.

Beyond GitHub's defaults, the labels worth reaching for are `change` (behaviour that already
existed and now differs), `security` and `removal`.

The sections, in their order: ✨ Added, 🔧 Changed, 🐛 Fixed, 🗑️ Removed, 🔒 Security,
🧰 Under the hood, 📦 Dependencies. The last one holds the pull requests labelled
`dependencies` — Dependabot applies that label itself — and nothing else. When a pull request
wears two mapped labels, the first section in that order wins: `dependencies` + `enhancement`
is an addition.

## Why the changelog is committed before the tag

`crates/engine/src/changelog.rs` pulls `CHANGELOG.md` in with `include_str!`, so the file is
baked into the binary at compile time. A release whose changelog was written after the build
would ship a binary that cannot show its own release notes in "What's new".

Which is also why the newest section is generated rather than hand-written: editing it by
hand is fine until the next release regenerates it. Older sections are never touched.

## What gets published

Per platform, from one matrix build: `cargo build -p app --release`, then
`cargo packager -p app --release` (`.github/actions/package`), which builds nothing and packages
what the build left in `target/release`. Its configuration is `[package.metadata.packager]` in
`crates/app/Cargo.toml`.

- **Windows** — the NSIS installer, per user and without administrator rights
  (`devnotes_<version>_x64-setup.exe`, the only Windows format: no `.msi`, D8), and the
  standalone executable (`devnotes-<version>-windows.exe`).
- **Linux** — the AppImage (`devnotes_<version>_x86_64.AppImage`), the `.deb` (by
  `cargo-packager`), the `.rpm` (by `cargo-generate-rpm`, `[package.metadata.generate-rpm]`),
  and the standalone binary (`devnotes-<version>-linux`).

Plus `SHA256SUMS.txt` over all of them, and `latest.json` — the manifest the in-app updater
reads, written by `scripts/latest-json.mjs`. Neither package needs WebView2 or WebKitGTK any
more; the Linux ones depend on GTK (the tray alone), xkbcommon, Vulkan and the indicator
library.

Each package is checked to hold the application alone (one binary, and the identity tool's
lists where `Paths::resources` looks for them) before anything is uploaded.

**Signatures.** `cargo-packager` signs every package it writes with the minisign key, and the
`.rpm` is signed by `cargo packager signer sign`. The key and its password are the release
environment's secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`,
passed as `CARGO_PACKAGER_SIGN_PRIVATE_KEY[_PASSWORD]`: the key the published versions were
signed with, kept (D26). The updater (`cargo-packager-updater`, `crates/platform/src/updater`)
installs only the NSIS installer (passive, `/P /R`) and the AppImage (replaced in place), so the
manifest carries those two, each with its `format`; the `.deb` and `.rpm` are updated by their
package manager by design, and their signatures are ignored rather than fatal.

⚠️ **A release without the private key publishes nothing.** That is deliberate: an unsigned
release would be one the updater refuses to install, discovered by users rather than by CI.
The public half is `UPDATES.public_key` in `crates/app/src/desktop.rs`.

## If something goes wrong

The workflow is ordered so that everything reversible happens first. The version check, the
changelog generation and the commit come before the tag; the tag comes before the build; the
release is published last. A failure before the tag leaves nothing to undo but a branch
commit.

A failure after the tag is the awkward one: delete the tag and the draft release, then run
again. This is what the dry run exists to avoid.
