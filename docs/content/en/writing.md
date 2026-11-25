# Writing

The five kinds of note, a snippet’s fields to fill, the code editor and Prettier, formatted notes and presenting them, calculation sheets, drawings, checklists and attachments.

## Five kinds of note

The *+ New note* button offers each of them. Each also has its shortcut, active on the canvas:

**Creating a note**

| Kind | For | Shortcut |
| --- | --- | --- |
| **Code snippet** | a block of code, its language and its fields to fill | <kbd>Ctrl N</kbd> |
| **Note** | formatted text: headings, lists, links, tables, code blocks | <kbd>Ctrl Shift N</kbd> |
| **Todo list** | steps to tick off, in order | <kbd>Ctrl Shift L</kbd> |
| **Calculation** | one line, one result: units, dates, totals | <kbd>Ctrl Shift M</kbd> |
| **Drawing** | boxes, arrows and text: a sketch | <kbd>Ctrl Shift D</kbd> |

The editor’s *Kind* menu turns a note into another kind, a drawing excepted.

A note has a title, a source (the *Context…* it comes from: a tool, a doc, a project) and tags. It is written to disk only once it holds something: a note opened then closed empty leaves nothing behind. <kbd>Esc</kbd> closes the editor, saving.

> [!TIP]
> The kind offered by default is chosen in *Preferences… → General → Default note type*.

## Snippets

A snippet is highlighted by its language: 142 to choose from, from JSON, TypeScript, Python, Rust, Go or SQL to Shell, PowerShell, YAML, TOML or Elixir, plus plain text. The colours are the same on the card as in the editor. The language menu filters from the keyboard: type the first letters, then <kbd>Enter</kbd>.

The code editor numbers its lines, keeps <kbd>Tab</kbd> in the text to indent, and finds with <kbd>Ctrl F</kbd>, replaces with <kbd>Ctrl H</kbd>. Line wrapping is set in the preferences.

### Formatting with Prettier

The *Format* button (<kbd>Shift Alt F</kbd>) runs the snippet through Prettier. A banner says how many lines changed, and *Undo* goes back.

- **Formatted:** JavaScript, TypeScript, JSON, CSS, SCSS, HTML, Markdown, YAML, GraphQL.
- **Not by Prettier:** SQL, Python, Rust, Go, Java, C#, PHP, C, Shell, TOML, XML. The button is disabled there and says why.

Line width, indentation, quotes, semicolons and trailing commas are set once for the whole library, like formatting on save. A `{{host}}` field goes through formatting intact; if Prettier were to damage one, the text is left untouched.

### Reading JSON as a graph

A JSON, YAML, TOML or XML snippet also opens as a *Graph* or a *Tree*. *Side by side* keeps the code on one side and the graph on the other: selecting a node highlights its lines. In the graph, the arrows move from node to node, <kbd>Enter</kbd> opens or closes an entry, and <kbd>Shift →</kbd> expands everything.

## Fields to fill

Write a name between double braces in a snippet, and its value will be asked for when you copy it. A name is made of letters, digits, hyphens and underscores. A suggested value is written after an equals sign:

**A snippet with three fields**

```sh
ssh -p {{port=22}} {{user}}@{{host}}
```

Copy it from its card (the button, or <kbd>C</kbd> on the focused card): DevNotes asks you to *Fill the fields*, shows a preview of what goes to the clipboard, then offers *Copy filled* or *Copy as is*. A field left empty keeps the value the text suggests.

What you type stays with the note, for next time. For a value valid everywhere, add a variable in *Preferences… → Variables*: it is offered to every snippet with a field of that name, and what is typed on a note comes first.

> [!TIP]
> The quick paste palette asks the same questions: a snippet with fields can be pasted from any application. See [Quick paste](paste.md).

## Formatted notes

A note is written formatted, and saved as Markdown. The toolbar and the shortcuts cover the essentials; a pasted code block keeps its language.

**Formatting a note**

| Keys | Action |
| --- | --- |
| <kbd>Ctrl Alt 1 … 3</kbd> | Heading 1, 2 or 3 |
| <kbd>Ctrl B</kbd> | Bold |
| <kbd>Ctrl I</kbd> | Italic |
| <kbd>Ctrl Shift S</kbd> | Strikethrough |
| <kbd>Ctrl E</kbd> | Inline code |
| <kbd>Ctrl Alt C</kbd> | Code block |
| <kbd>Ctrl Shift 8</kbd> | Bulleted list |
| <kbd>Ctrl Shift 7</kbd> | Numbered list |
| <kbd>Ctrl Shift 9</kbd> | Task list |
| <kbd>Ctrl Shift B</kbd> | Quote |
| <kbd>Ctrl K</kbd> | Link (Ctrl + click opens it) |

A table is inserted from the toolbar, then grows row by row or column by column.

### Presenting a note

*Present* shows the Note full screen, one section a slide: the arrows step through them, <kbd>M</kbd> opens the map of the headings and <kbd>Esc</kbd> leaves.

## Todo lists

A list is ticked from its card, without opening it, and the card shows its progress. In the editor, <kbd>Enter</kbd> adds an item below this one, <kbd>Backspace</kbd> removes an empty item and <kbd>Alt ↑</kbd> or <kbd>Alt ↓</kbd> moves it.

## Calculation sheets

A calculation sheet gives one result per line, as you type, and keeps none of them: only what you wrote is saved. Name a value, convert units, take a percentage, add durations:

**A few lines**

```
price = 120 €
3 h 20 min in min
20 % of 340
```

A name is usable on the lines below its own. The sheet reads units (CSS units and frequencies too), dates, every form of percentage, fractions, rounding and functions, bases and bits, and the finance forms: compound interest, returns, present value and loans. Offline, it knows no exchange rate, and says so. A line that cannot be computed says why, beside itself.

Click a result to select it, <kbd>Shift</kbd> or <kbd>Ctrl</kbd> to select more: the sheet adds them up. The number format, the date format and the decimals (up to fourteen) are set in *Preferences… → General*.

## Drawings

A drawing is boxes, arrows and text on a free surface. An arrow started on a shape follows it when the shape moves; let go on the middle of a side, it stays attached there. Both its ends can be picked up again. <kbd>A</kbd> draws an arrow and <kbd>Delete</kbd> removes the selection.

<kbd>Ctrl</kbd> and the wheel zoom the drawing in or out, the wheel alone scrolls it, and <kbd>F</kbd> shows the whole of it.

A drawing is exported (*Export as PNG* or *Export as SVG*), and inserted into a Note with *Insert a drawing*: the Note shows its picture, sized by its corners.

## Attachments

While a note is open, drop a file on the window, use *+ File*, or paste an image: it is encrypted and attached to the note. A file can weigh up to 10 MB. The card shows how many; a preview grows on click, and each attachment opens or is saved elsewhere.

## Pinning, prioritising, dating

- **Pinning** (<kbd>P</kbd> on a card) puts a note at the top of its group.
- **Priority** goes from `0` (none) to `4` (urgent): type the digit on a card.
- **A due date**, set from the editor’s date field, makes a note one to sort: the *Untriaged* filter finds them before they are forgotten.

## Copying, duplicating, deleting

1. A card’s copy button puts the note on the clipboard without opening it.
2. A card’s **⋯** menu duplicates it, or sends it to a space or a folder with *Move to…*
3. <kbd>Delete</kbd> on a card moves it to the trash in two steps: once to arm, a second time to confirm. <kbd>Ctrl Z</kbd> undoes it afterwards.
