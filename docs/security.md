# Your library, in detail

The [README](../README.md) says what DevNotes protects in a few lines. This page is the long version: how the library is sealed, what a stolen file still gives away, what lives in memory while the application runs, and where your files are.

## What leaves your machine

Nothing, unless you send it. A request you send from the HTTP client goes to the server you name and nowhere else, and tells it `DevNotes/<version>` as its `User-Agent` unless you set another, or none. The tools call nothing. The update check asks GitHub whether a newer release exists.

## How it is sealed

DevNotes asks for a passphrase the first time it runs, and once at every launch after that. The passphrase opens the library and is never stored anywhere — not in a keychain, not behind a "remember me".

| Aspect             | How                                                                                                                                                     |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Sealed on disk** | titles, bodies, sources and their history, list items, space and folder names, `{{field}}` values, attachment names and the attachment files themselves |
| **Key derivation** | Argon2id (64 MiB, 3 passes) over a random salt. The passphrase wraps the library key rather than being it, so changing it re-encrypts nothing           |
| **Encryption**     | AES-256-GCM with a fresh nonce per write; a tampered value is refused, never decrypted into nonsense                                                    |
| **Passphrase**     | at least twelve characters. Several unrelated words hold far better than one clever word with a digit in it                                             |
| **Exports**        | the one file meant to travel gets a passphrase of its own, or is written in the clear if you say so                                                     |

> [!WARNING]
> **There is no recovery.** No account, no escrow, no reset: a lost passphrase is a lost library. An export written in the clear is the only copy that does not depend on it.

### What a stolen file still gives away

What SQL filters, sorts and joins on is not sealed — sealing it would mean loading the whole library to answer a query. Without the passphrase, whoever holds your files cannot read a title, a body or an attachment, but they can read:

- how many notes there are, and how they hang in spaces and folders — the shape, not the names;
- when each note was created, changed, trashed or given a deadline, and whether it is pinned;
- each note's language and kind, how many items a list holds and which are ticked;
- every tag and every `{{field}}` name, as typed — `prod`, `aws`, `client-acme`, `db_password`;
- the length of every sealed value: AES-GCM hides what a title says, not how long it is;
- how many HTTP collections, folders and requests there are and how they nest, each request's kind and method, and when each was saved;
- when each request was sent and from which saved request — not to where, nor what was answered — and how many cookies are kept and when each expires, not for which site;
- how many HTTP environments there are and when each was made — not their names nor their variables;
- each attachment's type and size, the names of your libraries, and your preferences.

Keep that in mind before a tag or a field name carries the secret itself.

A sealed value is not bound to the row it belongs to: someone who can **write** your files, not just read them, could swap two sealed titles or bodies within the library and DevNotes would open both without complaint. Each still opens only under your key, and nothing new can be forged; protecting against an attacker who rewrites the database is outside what DevNotes promises.

### What lives in memory while it runs

The key, for the length of the session: there is no idle re-lock. And **your whole open library, decrypted**: once it is unlocked, DevNotes reads every note into an index in memory — titles, bodies, items, tags, folded for the search — so that searching, sorting and drawing the cards never go back to the sealed columns. It is kept until the library is closed or another one is opened, and then let go. The interface keeps caches beside it — the lines on screen, the pictures of a drawing, the diagrams of a Note, what was typed in a field — emptied whenever another library opens, but **not wiped byte by byte** when they are let go. Any of it can reach the swap file, a hibernation image or a crash dump. DevNotes protects files at rest — a stolen laptop, a copied profile — not a machine already compromised while it runs.

The passphrase is the exception. It is typed into a field of its own, which keeps no undo history and no copy anywhere in the interface's memory, and it is wiped as soon as the library has used it.

Opening an attachment writes a decrypted copy inside your own profile, never the shared temporary folder, because the program that opens it reads from disk. Those copies are deleted when DevNotes quits, and whatever survived is swept at the next launch.

### What the application itself guarantees

- **One process per profile.** A second launch on the same profile hands over to the first and exits: the index assumes it is the only writer of the library, and two writers would leave it silently wrong.
- **No note in the logs.** The log (`devnotes.log`, under `%LOCALAPPDATA%\com.devnotes.app\logs` or `~/.local/share/com.devnotes.app/logs`) records what happened, never what you wrote: a crash records where and on which thread, **never its message**, since a message can quote a piece of a note. Neither is a passphrase or a key ever logged.
- **No web page inside.** Since 1.0.0 the interface is native: there is no WebView, so no page, no script and no content policy to get wrong. A link in a note opens in your browser only if it is `http` or `https`; a `file:` link is a program to run, and is refused.
- **A request script runs with your rights.** The script host is DevNotes itself, started again: every send and every file a script asks for goes through the application, which asks where it should, but the host's isolation rests on the script engine alone, not on a sandbox of the operating system. Run only scripts you would run yourself.
- **No file sent behind your back.** A request script that gives a request a file to send, or sends one itself, stops the send and names the file; it leaves only after your yes, for that file alone. A collection run sends no such file. No file over 64 MB is read for a send.
- **Nothing of the window's state in clear.** The window remembers its size and place, and nothing else of what it showed.
- **Updates are signed.** An update is downloaded whole and checked against a signature only the release key can make before anything is installed; a file altered by a single byte is refused. Installed from a `.deb` or an `.rpm`, DevNotes leaves updates to your package manager.

## Your data

A library is a folder of your profile, and it is worth knowing where:

| Platform    | Folder                             |
| ----------- | ---------------------------------- |
| **Windows** | `%APPDATA%\com.devnotes.app\`      |
| **Linux**   | `~/.local/share/com.devnotes.app/` |

Each library lives in `libraries/<id>/`: the database, `vault.json` — the key file your passphrase opens — its attachments, and its backups. `libraries.json` lists your libraries and `preferences.json` holds the application's settings.

> [!IMPORTANT]
> The database and `vault.json` only mean anything together. Copy one without the other and you have copied something nobody can open again.

**Backups.** A rolling copy is taken at launch, at most one a day, and the last three are kept in `backups/`: database, key file and attachments together, restored in two clicks from Preferences → Security. The attachments are hard-linked rather than duplicated, so a copy costs little more than the notes. These copies sit next to the original, which is the accident they cover — an emptied trash, a botched update. For a dead disk, export somewhere else.

**Integrity.** The database is checked every time a library opens. If it is damaged, DevNotes says so instead of starting on it, and offers to set it aside with whatever could still be rescued; your passphrase does not change.
