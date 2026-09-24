# DevNotes

[![CI](https://github.com/vmillet-dev/devnotes-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/vmillet-dev/devnotes-rs/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/vmillet-dev/devnotes-rs)](https://github.com/vmillet-dev/devnotes-rs/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)

A notes and snippets manager for developers, on the desktop. Write a snippet once, find it
by tag or by full text, and paste it into any application from a global shortcut.

Search it, narrow it by tag, open it — and from any application, `Ctrl+Alt+P` brings up the
palette, where `Enter` copies and the window steps aside:

![A tour of DevNotes: the board of notes, a search narrowing it and quoting the line that matched, a tag filter, a note open in the editor, then the quick-paste palette asking a snippet for its fields](docs/quick-paste.gif)

Everything stays on your machine, encrypted with a passphrase you choose and type once at
launch. Nothing is uploaded, there is no account, and the application works with the network
off.

## What it does

| Feature               | What it gives you                                                                                 |
| --------------------- | ------------------------------------------------------------------------------------------------- |
| **Quick paste**       | `Ctrl+Alt+P` from any application: search a snippet, `Enter` copies it and the window steps aside |
| **`{{fields}}`**      | `psql -h {{host}} -p {{port=5432}}` asks for its values before landing in the clipboard           |
| **Keyboard canvas**   | arrows to move, `Enter` to open, `C` to copy, `P` to pin, `X` to select, `Del` to trash           |
| **Todo lists**        | a second kind of note: an ordered, tickable list instead of a body                                |
| **Trash**             | deleting is undoable, and reversible for 30 days                                                  |
| **Bulk actions**      | select several notes, then move, tag, export or trash them in one go                              |
| **Tag management**    | rename, merge or drop a tag across the whole library                                              |
| **Attachments**       | drop a file on the editor or paste an image; open it, save it elsewhere, preview it inline        |
| **Import / export**   | a `.devnotes` archive both ways, attachments included — everything, one space, or the selection   |
| **Copy as Markdown**  | the selection rendered for a pull request, a ticket or a chat message                             |
| **Encrypted at rest** | one passphrase at launch; notes and attachments sealed on disk, exports optionally too            |

Syntax highlighting covers eighteen languages, the interface is available in French and
English, and it ships with a light and a dark theme.

## Your library is encrypted

DevNotes asks for a passphrase the first time it runs, and once at every launch after that.
It is what opens the library, and it is never stored anywhere — not in a keychain, not
behind a "remember me".

What is sealed on disk: note titles, bodies and sources, the earlier bodies kept as history,
checklist items, space and folder names, `{{field}}` values, attachment file names, and the
attachment files themselves.

| What               | How                                                                                                                                                                                                                    |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Key derivation** | Argon2id, 64 MiB and 3 passes, over a random salt kept beside the database. The passphrase seals the library key rather than being it, so you can change it from Preferences → Security without re-encrypting anything |
| **Encryption**     | AES-256-GCM, a fresh nonce per write; the authentication tag refuses a tampered value rather than decrypting it into nonsense                                                                                          |
| **Not sealed**     | tags, dates, ids and the links between rows — what the database filters, sorts and joins on. Sealing them would mean loading the whole library to answer a query. What that leaves readable is listed just below       |

A passphrase is at least **twelve characters**, and that is what the protection rests on:
Argon2id makes each guess slow, but only the length makes the guesses many. Several unrelated
words hold far better than one clever word with a digit in it.

⚠️ **There is no recovery.** No account, no escrow, no reset: a lost passphrase is a lost
library. An export written in the clear is the only copy that does not depend on it.

⚠️ **Opening an attachment** writes a decrypted copy — inside your own profile, never the
shared temporary folder — because the program that opens it reads from disk. DevNotes deletes
those copies when it quits, and sweeps whatever survived — a file another application still
held, a crash — at the next launch.

An export is the one file meant to leave the machine, so it is offered a key of its own:
give it a passphrase and it travels sealed, attachments included, or write it in the clear
for a file any DevNotes can read. The application asks which, every time, and says which one
it wrote.

### What a stolen file gives away

Without the passphrase, whoever holds your files cannot read a title, a body or an
attachment. They can read everything that is not sealed, and that says more than the tags:

- how many notes there are, and how they hang in spaces and folders — the shape, not the names;
- when each note was created, last changed, put in the trash or given a deadline, and whether
  it is pinned;
- each note's language and kind, how many items a list holds and which are ticked;
- every tag and every `{{field}}` name, as typed — `prod`, `aws`, `client-acme`, `db_password`;
- the length of every sealed value: AES-GCM hides what a title says, not how long it is;
- each attachment's type and size;
- the names of your libraries and your preferences (`libraries.json`, `preferences.json`).

Keep that in mind before a tag or a field name carries the secret itself.

### What lives in memory while it runs

The key, for the length of the session: there is no idle re-lock. And your notes, decrypted
as they are read — a search reads the bodies it matches against, the interface holds the first
lines of every card on screen and the whole of the note you open. None of that is wiped when
it is let go, so it can reach the swap file, a hibernation image or a crash dump. DevNotes
protects a file read at rest — a stolen laptop, a copied profile — not a machine already
compromised while it runs.

## Where your library lives, and how it is backed up

A library is two files that only mean anything together — the database is sealed, and the
key file is what opens it — and they live in your profile:

|             |                                    |
| ----------- | ---------------------------------- |
| **Windows** | `%APPDATA%\com.devnotes.app\`      |
| **Linux**   | `~/.local/share/com.devnotes.app/` |

In it, `libraries.json` lists your libraries and `preferences.json` holds the application's
preferences. Each library is a folder of its own, `libraries/<id>/`, holding
`devnotes.sqlite3`, `vault.json`, `attachments/`, its own `preferences.json` and the
`backups/` described below. ⚠️ Copy the database without the key file and you have copied
something nobody can open again.

DevNotes takes a **rolling copy at launch**, at most one a day, and keeps the last three in
`backups/`. Each one is a full library — database, key file and attachments together — so
restoring is copying a folder back. It is written with `VACUUM INTO` rather than by copying
the file, because under WAL the database on its own is not a consistent snapshot. The
attachments are hard-linked rather than copied: a copy shares their bytes with the library,
so it costs little more than the notes, and an attachment you delete afterwards is still
whole in the copies taken before. Turn it off in Preferences → Security if you would rather
it did not.

DevNotes also checks the database is still sound every time it opens one. If it is not, it
says so rather than starting on it, and offers to set it aside: the database, its
attachments and whatever could still be rescued from it move into `damaged/`, and the next
unlock starts on a fresh library. Your passphrase does not change — so what was set aside,
like the copies in `backups/`, still opens with the one you already have.

⚠️ These copies sit **next to the original**, which is the accident they cover: an emptied
trash, a botched update, a file gone wrong. They are not a defence against a dead disk.
For that, export somewhere else — or copy that folder to another machine.

## Install

DevNotes runs on **Windows and Linux**. There is no macOS build: it cannot be tested here,
and Gatekeeper wants a paid Apple Developer account with no free way around it on recent
versions — shipping for a platform that can be neither tested nor distributed would be a
promise nobody can keep.

Download from the [latest release](https://github.com/vmillet-dev/devnotes-rs/releases/latest).

**Windows**

| File                               | Pick it if                                                     |
| ---------------------------------- | -------------------------------------------------------------- |
| `devnotes_<version>_x64-setup.exe` | You just want DevNotes installed. This is the one to take.     |
| `devnotes_<version>_x64_en-US.msi` | You deploy software through group policy or a management tool. |
| `devnotes-<version>-windows.exe`   | You want no installer at all — run it from where it lands.     |

**Linux**

| File                                | Pick it if                            |
| ----------------------------------- | ------------------------------------- |
| `devnotes_<version>_amd64.AppImage` | Any distribution, nothing to install. |
| `devnotes_<version>_amd64.deb`      | Debian, Ubuntu and derivatives.       |
| `devnotes-<version>-1.x86_64.rpm`   | Fedora, RHEL and derivatives.         |
| `devnotes-<version>-linux`          | The bare executable, no packaging.    |

```bash
chmod +x devnotes_*_amd64.AppImage        # make the AppImage runnable, then launch it
sudo apt install ./devnotes_*_amd64.deb   # Debian and Ubuntu — pulls in what it needs
sudo dnf install ./devnotes-*.x86_64.rpm  # Fedora and RHEL
```

On Windows, the installer and the MSI are opened by double-clicking them; the standalone
`.exe` needs nothing at all.

Updates are offered inside the application, so you only download by hand once.

Windows shows a SmartScreen warning the first time — choose **More info**, then **Run
anyway**. It is about the identity of the publisher, not about the file: DevNotes has no
code-signing certificate, while its updates are signed with minisign and verified before
they install. Every release also publishes `SHA256SUMS.txt`.

## Build from source

You need Node.js 24 — the version `.nvmrc` pins, and what CI installs — Rust through `rustup`, and
[Tauri's system dependencies](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev
```

Front-end changes hot-reload; Rust changes trigger a slower automatic recompile.
[CONTRIBUTING.md](CONTRIBUTING.md) has the rest: the full script list, what CI checks, and
the conventions that are load-bearing.

## Documentation

- [Contributing](CONTRIBUTING.md) — how to build and test, what CI checks, and the handful of
  conventions the compiler cannot enforce. Read it before your first change.
- [Architecture](docs/architecture.md) — front-end structure, state and data-access patterns,
  i18n, theming, the Angular ↔ Rust boundary, and testing conventions.
- [Releasing](docs/releasing.md) — how a version gets cut, and how release notes are sorted.
- [Changelog](CHANGELOG.md) — what changed, and when.
- [`docs/scratch-mockup-v2.html`](docs/scratch-mockup-v2.html) — the static UI mockup used as
  the visual reference. Not code to run or import.

## Built with

**Angular 22** on the front — standalone components, signals, zoneless change detection —
and **Rust with Tauri v2** as the native shell, persisting to an embedded SQLite database
through Diesel. The two halves are filed by subject rather than by technical nature, and the
IPC surface between them is generated from the Rust signatures.

## License

DevNotes is free software under the [GNU General Public License v3.0](LICENSE). You may use,
study, share and modify it; a distributed fork has to stay under the same terms and ship its
source.

Contributions are accepted under that same license: opening a pull request means you agree to
have your work distributed under the GPL-3.0.
