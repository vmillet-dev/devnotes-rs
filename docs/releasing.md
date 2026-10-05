# Releasing

A release is a `workflow_dispatch`. One run writes the changelog, commits it, tags, builds
every bundle and publishes — there is no step to do by hand between them, and no partial
state to clean up if it stops.

## The three steps

1. **Bump the version.** `npm version <x.y.z> --no-git-tag-version` covers `package.json`
   and its lockfile; `src-tauri/Cargo.toml` is edited by hand, then `cargo update -p devnotes`
   writes the Cargo lockfile. Merge to `main` and wait for CI to go green — the release
   refuses to start on a commit CI has not passed.

   The workflow reads the version from `Cargo.toml` and stops unless `package.json`,
   `package-lock.json` and `Cargo.lock` all agree. It checks that **before the tag exists**,
   so a mismatch costs a re-run rather than a deleted tag.

2. **Actions → Release → Run workflow, with `dry_run` checked.** It prints the notes it
   would write to the run summary and changes nothing. Read them: this is the only moment
   where a misfiled entry is free to fix.

3. **Run it again unchecked.** It writes the `CHANGELOG.md` section, commits it, tags,
   builds the bundles and publishes the release.

## Where the notes come from

Release notes are generated from the pull requests merged since the last tag, and what sorts
them is **labels**.

GitHub propagates nothing on its own, so a pull request inherits the labels of the issue it
closes — which only works if the body says `Closes #42`. From there:

| what decides the section                   | when                                                      |
| ------------------------------------------ | --------------------------------------------------------- |
| a label on the pull request                | always wins, and overrides the issue's for that one entry |
| the labels of the closed issue             | the ordinary case                                         |
| the `(feat)` / `(fix)` prefix of the title | when there is no label anywhere                           |
| "Under the hood"                           | when there is none of the three                           |

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

`src-tauri/src/changelog.rs` pulls `CHANGELOG.md` in with `include_str!`, so the file is
baked into the binary at compile time. A release whose changelog was written after the build
would ship a binary that cannot show its own release notes in "What's new".

Which is also why the newest section is generated rather than hand-written: editing it by
hand is fine until the next release regenerates it. Older sections are never touched.

## What gets published

Per platform, from one matrix build:

- **Windows** — the NSIS installer (`*_x64-setup.exe`), the MSI (`*_x64_en-US.msi`) and the
  standalone executable (`devnotes-<version>-windows.exe`).
- **Linux** — AppImage, `.deb`, `.rpm`, and the standalone binary (`devnotes-<version>-linux`).

Plus `SHA256SUMS.txt` over all of them, and `latest.json` — the manifest the in-app updater
reads.

`bundle.createUpdaterArtifacts` makes the bundler emit a `.sig` beside every bundle,
including the `.deb` and `.rpm` that the updater can never install; system packages are
updated by their package manager by design, so the manifest step ignores those signatures
rather than choking on them.

⚠️ **The release build fails outright without the minisign private key.** That is deliberate:
an unsigned release would be one the updater refuses to install, discovered by users rather
than by CI. The public half lives in `tauri.conf.json`.

## If something goes wrong

The workflow is ordered so that everything reversible happens first. The version check, the
changelog generation and the commit come before the tag; the tag comes before the build; the
release is published last. A failure before the tag leaves nothing to undo but a branch
commit.

A failure after the tag is the awkward one: delete the tag and the draft release, then run
again. This is what the dry run exists to avoid.
