# Import and export

Exporting everything, a space or a selection into a .devnotes archive, protected or not, importing it again without duplicates, and copying notes as Markdown.

## Exporting

The *File* menu exports, as you choose:

- *Export everything…*: the whole library;
- *Export active space…*: the open space;
- *Export selection…*: the ticked notes.

The export is a `.devnotes` archive: the notes, their spaces and their attachments. Snippet versions are not part of it.

### Protecting an export

Before writing the file, DevNotes asks *Protect this export?* A passphrase seals it with a key of its own that travels with the file, not with this machine: whoever you give the phrase to can open it, no one else. *Export in the clear* writes every note and every attachment readable by anyone who holds the file.

> [!TIP]
> A plain export, kept somewhere safe, is the only copy that does not depend on your passphrase. See [Security and backups](safety.md).

## Importing

*File → Import…* reads a `.devnotes` archive (or an older `.json` export). A protected archive asks for its passphrase first.

- An import merges, it never replaces: spaces are matched by their name.
- A note already present is counted as skipped, not overwritten: importing the same file again creates no duplicate.
- DevNotes always says what it imported, even when that is no note at all.

## Copying as Markdown

*File → Copy selection as Markdown* puts the ticked notes on the clipboard: for each, its title, its space, its context, its tags, then its content in a code block. Enough to paste a procedure into a ticket or a wiki.
