# Organising and finding

Spaces and their folders, the Board view, tags, search, filters, sorting and multiple selection.

## Spaces

Every note lives in a space: “Personal”, “Work”, a client, a project. The sidebar lists them; *All spaces* shows them together. *New space* creates one, and a space’s menu pins it to the top, renames it or deletes it.

Deleting a space always asks where its notes should go: they never leave with it. So deleting one takes at least a second space.

## Folders

A folder cuts a space into named, coloured regions: blue, amber, purple, green or red. *New folder* creates one in the open space; its menu renames it, changes its colour or deletes it, and *New sub-folder* creates one inside. The notes of a deleted folder stay, handed to the folder above. A folder dragged onto another goes inside it; <kbd>Alt ←</kbd> and <kbd>Alt →</kbd> do it from the keyboard. Dragged onto another space, it moves there with its notes.

## The Board view

The *List* / *Board* switch changes how a space is seen. On the *Board*, each folder is a zone you move and resize, and each note a card laid on it. The board shows one level at a time, and a click on a zone opens its own board:

- drag a card into a zone to file it in that folder;
- drop it on the background to take it out of its folder;
- *Align* (<kbd>A</kbd>) lines the loose cards up under the folders, leaving the zones alone;
- *Other arrangements*, beside it, offers to place and resize the folders and their cards again. It can be undone.

Without a mouse: <kbd>X</kbd> ticks cards, then *Move to…* sends them to a folder. Opening a folder shows only its notes; <kbd>Esc</kbd> leaves it.

## Search

<kbd>Ctrl K</kbd> goes to the search field. It reads titles, bodies, sources, list items and tags, accents included: `deploiement` finds “Déploiement”. Every card found quotes the matching line.

## Filters, tags and sorting

- **Quick filters**: *All*, *Pinned* and *Untriaged* (the notes that have a due date).
- **Tags and formats**: the button beside them filters by tag, language, kind of note or priority.
- **Sort and group**: by date modified or created, by priority, by format or by title; grouped by date, by priority, by format, or not at all. *Pinned first* keeps the pinned notes on top.

The tags’ *Manage* button opens *Manage tags*: rename a tag on all its notes, merge several, or remove them. Renaming onto an existing tag merges the two.

## Multiple selection

Tick cards with <kbd>X</kbd>, or <kbd>Ctrl</kbd> + click; <kbd>Shift</kbd> + click extends the selection. The bar that appears acts on all of them at once: *Move to…* a space or a folder, *Priority*, *+ tag*, *Move to trash*. Each action says what it changed, and undoes exactly that.

## The canvas from the keyboard

**On the canvas**

| Keys | Action |
| --- | --- |
| <kbd>Ctrl K</kbd> | Go to the search field |
| <kbd>↑ ↓ ← →</kbd> | Move from card to card |
| <kbd>Enter</kbd> | Open the focused card |
| <kbd>C</kbd> | Copy the card, or ask for its fields |
| <kbd>P</kbd> | Pin or unpin |
| <kbd>0 … 4</kbd> | Set a priority: 0 none, 1 low… 4 urgent |
| <kbd>X</kbd> | Tick the card for a bulk action |
| <kbd>A</kbd> | Align the loose cards (board) |
| <kbd>Delete</kbd> | Move to the trash, in two presses |
| <kbd>Ctrl Z</kbd> | Undo the last deletion |
| <kbd>Esc</kbd> | Step back from the note, the selection, the filters, then leave the folder |

Bare letters act only on the canvas, never while typing. They are changed in *Preferences… → Shortcuts*.
