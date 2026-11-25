# The documentation

The pages devnotes.fr shows, and the application reads offline. One Markdown file per page and per language: `fr/start.md` and `en/start.md` are the same page. Nothing is generated beside them; to change a page, edit its two files.

## Writing a page

A page begins with its title and one paragraph, its summary:

```md
# Prise en main

Installer DevNotes et faire le tour des notes d’exemple.

## Installer
```

| To write                                       | Type                                                                        |
| ---------------------------------------------- | --------------------------------------------------------------------------- |
| A section, a sub-section                       | `## Installer`, `### Sous Linux`                                            |
| A label of the application, as it is on screen | `*Fichier → Préférences…*`                                                  |
| A shortcut                                     | `<kbd>Ctrl Alt P</kbd>`                                                     |
| A word that matters                            | `**jamais**`                                                                |
| Code, a file name, a value                     | `` `{{host}}` ``                                                            |
| A link to another page, to one of its sections | `[Sécurité](safety.md)`, `[la corbeille](safety.md#la-corbeille-et-ctrl-z)` |
| A link elsewhere, the download page included   | `[le dépôt](https://github.com/…)`                                          |
| A tip, a warning                               | `> [!TIP]` or `> [!WARNING]`, then the text on the next line, after `>`     |

A value inside a label is an ellipsis: `*Définir dans …*`.

A table and a code block have their title in bold, alone on its line just above:

```md
**Installer sous Linux**

| Système | Commande           |
| ------- | ------------------ |
| Debian  | `sudo apt install` |
```

Two shapes are drawn in a way of their own: a table headed `Touches | Action` (`Keys | Action` in English) is a table of shortcuts, and a list of nothing but links to pages is the overview's list.

A paragraph, an item and a note hold one line, however long. Anything else (an image, HTML, a list inside a list) is refused by the site's build, with its file and its line.

## Adding a page

Write its two files, then name it in `index.json`, in the group it is read in: its colour, its address and its name in the sidebar, in each language. Add it to the list of the overview.

`checkedOn` is the version of the application the pages were last read against.

## Adding a language

Copy the folder of one, translate it, and give every name of `index.json` in that language. The site (`website/src/app/core/lang.ts`) and the application (`Language`, in `crates/i18n`) must speak it too.
