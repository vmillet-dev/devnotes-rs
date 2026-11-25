# Quick paste and shortcuts

Finding a snippet and pasting it without leaving what you are doing, capturing the clipboard into a note, and setting the shortcuts.

## The palette

<kbd>Ctrl Alt P</kbd> opens the palette over any application, even when the DevNotes window is closed. It searches all your spaces at once, ignoring the canvas filters: when you recall a snippet, you do not always remember where you filed it.

1. Type a few letters of the title, the code or a tag.
2. Choose with the arrows.
3. <kbd>Ctrl C</kbd> copies the note and closes the palette: you are back in the application you came from, ready to paste.

A snippet with fields, like `{{host}}`, first asks for their values: copying a command with holes as it is would give a command you cannot use. <kbd>Enter</kbd> or a click opens the note instead of copying it, which also makes the palette a “find this note”.

It finds more than notes. The same field searches the requests of the API client, the tools and the spaces, and runs commands; three prefixes narrow the search:

**Narrowing the search**

| Type | To |
| --- | --- |
| <kbd>@</kbd> | choose where to search: the notes, the requests, the tools, the spaces, or the notes of one space |
| <kbd>#</kbd> | list the notes of a tag |
| <kbd>></kbd> | run a command: new request, new folder, preferences, the shortcut sheet |

**In the palette**

| Keys | Action |
| --- | --- |
| <kbd>↑ ↓</kbd> | Go through the results (Tab too) |
| <kbd>Enter</kbd> | Open the note, the request, the tool or the space |
| <kbd>Ctrl C</kbd> | Copy the note and close |
| <kbd>Esc</kbd> | Close the palette |

> [!TIP]
> Nothing matches? The last line offers *+ Create a note “…”* with what you typed.

## Capturing the clipboard

<kbd>Ctrl Alt V</kbd> does the opposite of the palette: what you have just copied, wherever it was, becomes a note. <kbd>Ctrl Alt N</kbd> opens an empty note. Both work with the window closed.

## From the system tray

The DevNotes icon in the system tray offers *Palette*, *New note* and *Paste from clipboard*. By default, closing the window puts it in the system tray, where DevNotes keeps listening for its shortcuts (*Close to tray*, in the preferences); *Start with the system* launches it when you log in.

## Global shortcuts

**Active everywhere, even with the window closed**

| Default | Action |
| --- | --- |
| <kbd>Ctrl Alt P</kbd> | Open the palette |
| <kbd>Ctrl Alt V</kbd> | Capture the clipboard into a note |
| <kbd>Ctrl Alt N</kbd> | Create a note |

These three act in all your applications: they take at least one modifier, or the key would be swallowed everywhere. First come, first served: if another application already holds a combination, DevNotes says so at startup, and you choose another.

## Switching areas

**In all three areas**

| Keys | Action |
| --- | --- |
| <kbd>Ctrl 1</kbd> | Go to the notes |
| <kbd>Ctrl 2</kbd> | Go to the tools |
| <kbd>Ctrl 3</kbd> | Go to the HTTP client |
| <kbd>Ctrl Shift T</kbd> | Search for a tool |

## Setting your shortcuts

In *File → Preferences… → Shortcuts*, click an action’s field, then press the combination. DevNotes warns you if another action already has it. *Reset* goes back to the original.

- **Global:** at least one modifier (Ctrl, Alt, Shift or Super).
- **Canvas:** a bare letter is enough, since they never act while typing.
- **Fixed:** the arrows, Esc, Tab and clicks with Ctrl or Shift do not move. They are navigation, leaving and selection.

The full list of shortcuts, wherever you are, is in *About → Keyboard shortcuts*. The canvas ones are detailed in [Organising and finding](organising.md).
