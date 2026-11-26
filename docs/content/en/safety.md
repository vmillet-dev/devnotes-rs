# Security and backups

What is encrypted and how, the passphrase, several libraries, backup copies, the trash, versions and undo.

## What leaves your machine

Nothing, except what you send. No account, no cloud, no telemetry. A request from the API client goes to the server you name, and nowhere else; the tools call nothing; the update check only asks GitHub whether a newer version exists.

## Encryption

DevNotes asks for your passphrase once at each launch. It opens the library and is stored nowhere: no keychain, no “remember me”.

**How a library is sealed**

| What | How |
| --- | --- |
| **Sealed on disk** | titles, bodies, sources and their history, list items, names of spaces and folders, field values, names and contents of attachments |
| **Key derivation** | Argon2id (64 MiB, 3 passes) over a random salt |
| **Encryption** | AES-256-GCM, a fresh nonce on every write; a tampered value is refused |
| **Passphrase** | at least twelve characters; it wraps the library’s key, it is not the key |

### What a stolen file shows

What the database filters and sorts on is not sealed: sealing it would mean decrypting everything to answer a search. Without the passphrase, no one reads a title, a body or an attachment, but they see:

- the number of notes and how they are spread across spaces and folders, without their names;
- the dates of creation, modification and due dates, and the pinned notes;
- the language and kind of each note;
- every tag and every field name, as you typed them;
- the length of each sealed value.

> [!TIP]
> So do not let a tag or a field name carry the secret: `db_password` stays readable, its value does not.

## The passphrase

It is changed in *Preferences… → Security → Change…*: the new one applies at the next launch. Nothing is re-encrypted, the phrase only wraps the key; the backup copies kept beside it are rewrapped with it.

> [!WARNING]
> **There is no recovery.** A forgotten phrase cannot be reset. *Forgotten your passphrase?* only offers to archive the library, still sealed, and to start a new one. Keep a plain export somewhere safe if your notes matter.

## Several libraries

*File → Libraries…* creates, renames, opens or deletes them. Each has its own database, attachments, backup copies and passphrase: nothing passes from one to another. Switching library reloads the window; neither the open one nor the last one can be deleted.

## Backup copies

With *Copy the library at launch* (on by default), DevNotes takes a copy at startup, one a day at most, and keeps the last three: the database, the key that opens it and the attachments. Attachments are linked rather than duplicated: a copy costs barely more than the notes.

*Preferences… → Security → Restore…* replaces the whole library with a copy. Nothing is deleted: the current one is set aside, and you are told where.

> [!TIP]
> These copies are kept beside the original: they cover a trash emptied too soon or a failed update, not a dead disk. For that, [export](transfer.md) elsewhere.

## The trash and Ctrl Z

A deleted note goes to the trash and stays there thirty days. The banner that follows offers *Undo*, and <kbd>Ctrl Z</kbd> still does it after the banner has gone. The *Trash*, at the bottom of the sidebar, restores a note or deletes it for good.

## A snippet’s versions

A snippet, like a Note, keeps its last twenty versions. The editor’s *History* shows what would come back and what would go before you *Go back to this version*. Versions are not included in exports.

## If the database is damaged

The database is checked at every opening. If it is damaged, DevNotes says so instead of starting on it, and offers to set it aside with what can still be recovered. Your passphrase does not change.

## Where your files are

**The folder of your libraries**

| System | Folder |
| --- | --- |
| Windows | `%APPDATA%\com.devnotes.app\` |
| macOS | `~/Library/Application Support/com.devnotes.app/` |
| Linux | `~/.local/share/com.devnotes.app/` |

Each library lives in `libraries/<id>/`: the database, `vault.json` (the key your phrase opens), the attachments and the backup copies.

> [!WARNING]
> The database and `vault.json` are only worth something together. Copy one without the other, and you will have copied something no one can open again.

## While DevNotes runs

1. The key stays in memory for the session: there is no relocking after inactivity.
2. Opening an attachment writes a decrypted copy to your profile, erased when DevNotes closes.
3. DevNotes protects your files at rest (a stolen laptop, a copied profile), not a machine already compromised while it runs.
