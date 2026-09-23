# Architecture

How DevNotes is put together, and the conventions to follow when extending it.
For build/run instructions see the [README](../README.md).

## Overview

DevNotes is a Tauri v2 desktop app: an Angular single-page front-end rendered in a WebView,
and a Rust process that owns everything native (storage, and later hashing and filesystem).
The two halves talk only through Tauri's `invoke()` bridge.

Notes and spaces are complete end to end: the front-end has no in-memory dataset left, every
read and write goes through `invoke()`, and the Rust commands persist to an embedded SQLite
database. Built on top of that: a 30-day trash with undo, multiple selection and bulk actions,
corpus-wide tag management, `{{fields}}` in snippets, a quick-paste palette on a global
shortcut, attachments, and import / export / share. The planned domains (crypto, formatters)
have **no** module of their own yet: a
placeholder would ship dead code in the binary, and an empty file documenting a contract
drifts from whatever eventually gets written.

**Where the work happens.** Data processing belongs to Rust. Filtering (space, full-text,
tags, languages, quick filters), grouping into display sections, facet aggregation and tag
normalisation, and the choice of what a card's footer shows all run in `src-tauri/src/notes/`.
The front-end describes what the user asked for and renders the view it gets back — it does
not filter, sort or group. The deliberate exceptions are relative-time **formatting** (labels
must age on their own, without a round trip), the ISO ↔ `Date` conversion at the serialisation
boundary, syntax highlighting (it colours the in-flight editor draft, which is not persisted
yet — a round trip per keystroke), and plain UI concerns like keyboard shortcuts and drafts.

```
src/                Angular front-end
├── app/
│   ├── app.component.*   the frame: titlebar, banners, <router-outlet>
│   ├── core/       everything that has no place on screen
│   │   ├── model/      the vocabulary: note, space, checklist, variable, language
│   │   ├── data/       repositories and the wire mapper
│   │   ├── state/      the stores, flat
│   │   ├── ipc/        generated bindings, error contract, native events
│   │   └── services/   one folder per subject: i18n, errors, time, preferences,
│   │                   settings, updates, app-info, clipboard, dialogs, window,
│   │                   shortcuts, autostart, tray, notifications
│   ├── notes/      the page at the root, then its four zones: sidebar/ (the library
│   │               rail), header/ (above the canvas), canvas/ (the cards),
│   │               overlays/ (drawn over the page), plus ui/ for what two of them share
│   ├── titlebar/   titlebar.component, then file-menu/ and about-menu/ with the panels
│   │               each of them opens, nested where they open from
│   ├── banners/    error banner, status toast, update prompt — siblings of the outlet
│   └── shared/     what crosses two areas of the screen: the modal frame, a11y directives
├── assets/         static images
├── styles/         global theme (styles.scss) and SCSS partials
└── testing/        test doubles, fixtures and shared providers
src-tauri/          Rust back-end
├── src/notes/      the notes feature: model, language, view, placeholder, trash, store/
├── src/spaces/     the spaces feature: model, SQL
├── src/attachments/ the attachments feature: model, SQL (the bytes live on disk)
├── src/transfer/   import, export and share: bundle rules, the file, the exchange format
├── src/db.rs       connection, migrations, schema, stored-instant format
├── src/error.rs    the three errors and the translation between them
├── src/desktop.rs  tray and global shortcuts — native glue, not a feature
├── src/lib.rs      Tauri builder, database setup + command registration
└── capabilities/   Tauri v2 permission manifests
```

### Feature-first, not layer-first

The back-end is filed by **subject**. `notes.rs`, `spaces.rs`, `folders.rs`, `attachments.rs`
and `transfer.rs` are the features, and each owns everything about itself: its model, its SQL,
and the Tauri commands that expose it. Deleting `src/notes/` deletes the feature.

`changelog.rs` is the smallest of them, and the odd one out: it owns no table and reads no
database — the repository's `CHANGELOG.md` is baked into the binary by `include_str!` and
parsed by `changelog/model.rs`. It keeps the convention all the same, the command in the file
and the rules in the module, which is what lets the parser be tested without a Tauri runtime.

`transfer` is the one without a `store.rs`: import and export read and write **whole
libraries**, so they compose the two other stores rather than owning a table. That is also why
it is the only feature allowed a `notes::store::all` — a raw note list, which no command ever
returns to the front (see [Data access](#data-access)). Its rules sit in two modules of its
own instead: `transfer/bundle.rs` (what travels with an export, how an import merges) and
`transfer/file.rs` (reading and writing the file, staged then renamed).

Both follow the same three-part convention:

| File                 | Holds                                                                   |
| -------------------- | ----------------------------------------------------------------------- |
| `<feature>.rs`       | the `#[tauri::command]` functions — validate, lock, delegate, translate |
| `<feature>/model.rs` | the types and the business rules, testable without opening a database   |
| `<feature>/store.rs` | the SQL, and nothing else                                               |

`notes/` carries extra modules, all of them notes-specific vocabulary: `language.rs`
(the closed `Language` enum and the heuristics that guess one from pasted content),
`view.rs` (what is asked — `NotesQuery`, `NoteFilter` — what comes back — `NotesView`,
`NoteSection` — plus the search matching and chronological placement that produce it),
`trash.rs` (the 30-day retention, `TrashedNote`, and the purge that erases the attachment
files with the rows) and `checklist.rs` (the closed `NoteKind` enum, `ChecklistItem`, the
normalization of a list and its Markdown rendering).

Its SQL is the one store large enough to be split, into `notes/store/`: `related.rs` for the
three side tables a note owns — tags, checklist items, `{{field}}` values, each keyed on
`note_id` and rewritten whole, so each read, replaced and bulk-loaded the same way — and
`trash.rs` for the queries that reason about `deleted_at`. ⚠️ `notes::trash` and
`notes::store::trash` are the retention's **rules** and its **SQL**; they are the same split
as `model.rs` and `store.rs` everywhere else.

⚠️ `notes::store::trash` is a **soft** delete — it stamps `deleted_at` — where
`spaces::store::delete` and `attachments::store::delete` erase. `notes::store::trash::purge`
is the destructive one, and it is restricted to rows already in the trash.

What is left at the root is what belongs to no single feature:

| Module        | Holds                                                                                          |
| ------------- | ---------------------------------------------------------------------------------------------- |
| `error.rs`    | `ValidationError`, `StorageError`, and the `AppError` that crosses the bridge                  |
| `db.rs`       | the connection and its `Mutex`, `open`/`open_in_memory`, plus `db::schema` and `db::migration` |
| `db::iso8601` | the stored-instant format — millisecond-exact, because the canvas sorts on a TEXT column       |
| `desktop.rs`  | tray and global shortcuts, including the `sync_tray` command that feeds the tray its labels    |
| `app_info.rs` | what the application says about itself, read from `Cargo.toml` at compile time by `build.rs`   |

This replaces an earlier split into three technical layers (`commands/ → domain/ ← storage/`),
which cost three files and three modules per subject and a `check-layers.sh` script in CI to
hold the direction — for a back-end whose whole job is a CRUD over two entities.

The property that split was really protecting survives on its own: `notes/model.rs`,
`notes/view.rs`, `notes/language.rs` and `spaces/model.rs` import neither Diesel nor Tauri,
so their tests run without opening a database — section placement, timezone boundaries, tag
normalisation, search folding, footer choice and expiry thresholds, in a few milliseconds
with no fixture setup.

The features are not fully independent, and that is visible rather than hidden:
`notes/store.rs` calls `spaces::store::exists` before filing a note, `spaces/store.rs`
moves notes out before dropping a space, and `folders/store.rs` writes `notes.folder_id`. The Diesel schema therefore stays shared in
`db/schema.rs` — splitting it per feature would break `allow_tables_to_appear_in_same_query!`.

Serde attributes sit on the model types rather than on a separate DTO family. At this size a
second set of types and their mapping would cost more than it protects. Each of these types
also derives `specta::Type`, which is what lets tauri-specta generate the front-end's
`bindings.ts` from them.

## Front-end

### Where a file goes

> **A folder's path is its address in the interface** — for everything that has one. A
> component lives in the folder of its parent on screen; one with two parents rises to their
> nearest common ancestor. What has no place on screen lives in `core/`: the model, the data
> access, the stores, the IPC, and one folder per cross-cutting subject under `services/`.

The tree used to answer a question nobody asks. Every path said what _technical kind_ a file
was — `state/`, `data/`, `ui/`, `layout/` — which you know before you open the tree. What you
do not know, and what you are actually looking for, is **where the thing is on screen**: you
see a button, you want the code behind it. `titlebar/about-menu/whats-new-dialog/` says
exactly where to click. Walking down the tree is walking down the screen.

**But a store is not a component.** A `providedIn: 'root'` singleton has no place on screen —
it has consumers, and `SpacesStore`, `NoteSelectionStore` and `LibraryStore` are read by the
File menu in the titlebar as much as by the canvas. Filing one under a consumer would claim a
containment that does not exist. The same holds for the model and the repositories: there is
**one application and one domain**, so its vocabulary sits at the top rather than inside the
single screen that happens to show it. `core/model/`, `core/data/` and `core/state/` are one
flat folder each, and `notes/` holds the interface alone.

⚠️ The cost is explicit and was accepted: deleting `notes/` no longer deletes the feature — the
vocabulary, the repositories and the stores stay behind in `core/`. `core/` is therefore no
longer "what a second tool would inject verbatim"; it is "what has no place on screen".

Membership is decidable, not a matter of taste:

| Question                                    | Answer                                       |
| ------------------------------------------- | -------------------------------------------- |
| Where does this component appear on screen? | in the folder of the component that shows it |
| Two or more parents show it?                | their nearest common ancestor                |
| Is it a model, a repository or a store?     | `core/model/`, `core/data/`, `core/state/`   |
| Is it a cross-cutting service?              | `core/services/<subject>/`                   |
| Does it cross two areas of the screen?      | `shared/`                                    |

**The zones come from the template, not from taste.** `notes/` used to hold eleven entries
that mixed screen zones with invented categories — `tag-rail` sat outside `topbar/` while
`search-box` sat inside, `image-lightbox` outside `overlays/` while the palette sat inside, and
nothing said why. The page's template has four zones: what sits beside the canvas, what sits
above it, the canvas, and what is drawn over the page. `sidebar/`, `header/`, `canvas/` and
`overlays/` are those four, so finding a component is one question with four answers — left of
the notes, above them, among them, or over them.

Two consequences worth stating. **Rendering something is not owning it:** the preferences
panel hosts the variables page through `NgComponentOutlet`, and the shortcuts sheet imports
the notes' key groups — neither moves those files into `titlebar/`, because the notes own
them. And **deletion became local**: the trash is one folder, where it used to be a panel in
`ui/` and a store adrift among twenty-four others in `state/`.

There is **one** exception, and it is deliberate: `DialogComponent` (`shared/dialog/`)
injects `DialogStack`, its own neighbour in the same folder. What the rule forbids is a shared
component knowing a feature or an application store; a modal has to know which modal is in
front, and that knowledge cannot be handed down an `input()` from twelve callers.

### Imports

**One alias per area of the screen**: `@notes/*`, `@titlebar/*`, `@banners/*`, plus `@core/*`,
`@shared/*` and `@testing/*` (declared in `tsconfig.json`, and nowhere else). An import says
which part of the interface it reaches into before it says which file.

The rule is **relative when a single `../` reaches the target, alias otherwise** — so
`core/state/notes.store.ts` reads `../data/notes.repository`, while
`notes/canvas/note-section/note-card/` reaches the model through `@core/model/note.model`. There is
no `../../` anywhere in `src/`, and since this reorganisation that is **enforced**:
`no-restricted-imports` in `eslint.config.mjs` refuses the pattern. The aliases are what make
it possible — the tree is four levels deep in places, and without them a card reaching the
model would write `../../../`.

**No `index.ts` barrels.** Three reasons, in order of weight: a barrel at `notes/index.ts`
would pull `data/`, `state/` and every component into the lazy chunk _while hiding that it
does_ — the explicit `loadComponent` path is what keeps the chunk honest; barrels re-close
import cycles by construction, and this codebase has one deliberate cycle broken by hand
(`core/ipc` ↔ `core/data`, see below); and with six aliases the
import lines are already short. The tree has zero barrels — keep it that way.

The app bootstraps standalone components (`src/main.ts` → `bootstrapApplication`); there
are no NgModules. Change detection is **zoneless** (`provideZonelessChangeDetection()`).
State lives in signals and every component is `OnPush` — enforced by the
`prefer-on-push-component-change-detection` lint rule, not by convention alone.

### Routing

`AppComponent` _is_ the persistent chrome — titlebar, global error banner, status toast and
update prompt — around a `<router-outlet>`. Features are lazy-loaded with
`loadComponent`, so adding the planned crypto and formatters tools will not weigh on the
initial bundle.

Routing uses **hash location** (`withHashLocation()`). Tauri serves the built files from an
internal protocol with no server to rewrite deep URLs back to `index.html`; the fragment
sidesteps the problem entirely.

### Component contracts

Components communicate through signal inputs (`input()` / `input.required()`), `model()` where
a value is genuinely two-way (the search field), and `output()` emitters.

**Where the line sits between an input and an injected store:** data still flows **down** as
inputs — a component is driven by its parent and stays testable with a literal — but an
**action goes straight to the store** rather than bubbling up to be forwarded. That is why
`NoteEditorOverlayComponent` takes `[note]` as an input and injects `AttachmentsStore` and
`PlaceholderFillStore`: attachments and `{{field}}` filling have a write cycle of their own,
and routing them through the page cost seventeen bindings and made adding one a four-file
change. What stays an output is what the **page** has to arbitrate: closing the overlay, a
patch (only `NotesStore` knows whether the note exists yet), a deletion.

**`shared/` is the exception that injects nothing at all** — it is a presentation kit, and
a component there takes everything through `input()`. `CodeViewerComponent` knows nothing
about notes. Components specific to the notes live under their own `ui/` folder, and are free
to reach for the feature's stores.

Components never reach into each other imperatively. A keyboard shortcut belongs to the
component that owns the affected element: `Ctrl/⌘+K` is handled inside `SearchBoxComponent`,
which also renders the hint, rather than travelling down a chain of `viewChild` calls.

A component that only relays inputs and outputs is not a component. The page composes
`SpaceSwitcher`, `SearchBox`, `FilterChips` and `NoteSection` directly rather than through a
topbar and a canvas wrapper, which added two files and eleven declarations without a single
decision between them. The same rule applied to `NoteSectionComponent`, which used to forward
eleven bindings to the card without reading one of them: the section now takes `[section]`,
the card takes `[note]` and reads selection, focus and ticks off the stores, and the whole
binding list is

```html
<app-note-section [section]="section" (noteActivated)="onNoteActivated($event)" />
```

`noteActivated` stays an output because **the card cannot decide what its own click means**:
ticking, extending a range or opening depends on the visible list, which is the canvas's.

### Shared behaviour lives in one place, not in copies

Three menus (space switcher, card actions, about) share their interaction rules through
directives in `shared/a11y/`, applied with `hostDirectives` so no wrapper element is needed:

| Directive              | Selector                                  | Owns                                                                              |
| ---------------------- | ----------------------------------------- | --------------------------------------------------------------------------------- |
| `MenuTriggerDirective` | `[appMenuTrigger]`, `exportAs: 'appMenu'` | open state, outside click, Escape, focus returned to `[appMenuAnchor]`            |
| `MenuPanelDirective`   | `[appMenuPanel]`                          | `role="menu"`, focus on the first entry, arrows and Home/End over `[appMenuItem]` |

Two details are load-bearing:

- `MenuPanelDirective` walks `[appMenuItem]` rather than every button, because a menu may
  carry a secondary action deliberately outside the arrow cycle — the `⋯` that opens a
  space's edit panel is reachable by Tab, not by arrows.
- `MenuTriggerDirective` **emits** `escaped` instead of closing on Escape. A single-level menu
  wires it straight to `close()`; the space switcher first collapses its create/edit panel and
  only closes on the second press.

Putting the click listener in a directive also removes the `click-events-have-key-events`
suppressions the three modal templates used to carry: the keyboard equivalent exists, it is
Escape, and the template no longer declares a bare `(click)` for the linter to flag.

#### The modal frame

The twelve modals do **not** each carry a frame. `DialogComponent` (`shared/dialog/`) owns
the scrim, the panel, `role="dialog"`, `aria-modal`, the focus trap, Escape and the backdrop
click; a dialog projects its content into it and says which rung it sits on:

```html
<app-dialog [layer]="'app'" labelledBy="about-dialog-title" (closed)="closed.emit()">
  …the panel's content…
</app-dialog>
```

| Input                  | Decides                                                                           |
| ---------------------- | --------------------------------------------------------------------------------- |
| `layer` (required)     | the rung: `app`, `editor`, `settings`, `update`, `palette`, `fields`, `zoom`      |
| `variant`              | `fitted` (height follows the content), `framed` (fixed, scrolling middle), `bare` |
| `fullscreen`           | fills the window — the editor's toggle                                            |
| `dismissible`          | `false` refuses Escape and the backdrop click (an update being installed)         |
| `labelledBy` / `label` | what names it to assistive technology                                             |

What a shell cannot guess — how wide the panel is, the gap between its blocks, its padding,
where it sits on the scrim — comes from CSS custom properties the consumer sets on the
`app-dialog` element from its own stylesheet: `--dialog-width`, `--dialog-height`,
`--dialog-gap`, `--dialog-padding`, `--dialog-align`, `--dialog-offset`, `--dialog-scrim`.

**No measurement travels through an `input()`**, and none is spelled in a template. The
clamps stay in `dialog.component.scss` — `min(var(--dialog-width, 480px), 90vw)` on both
surfaced variants, `min(var(--dialog-height, 560px), 86vh)` on `framed` alone, and
`fullscreen` last so it wins over either. `bare` is deliberately left unsized: it is
whatever it shows, an image the panel only bounds.

**The stacking order is a list, not a set of magic numbers.** `dialog.model.ts` holds
`LAYERS` in back-to-front order, and its index _is_ both the `z-index` and the priority
Escape follows. No stylesheet carries a modal `z-index` any more, and adding a rung is one
entry in that array.

⚠️ `app` — the About menu's four help panels — sits **under** `editor`. A help panel covers
the whole page, so the only way to a note while one is up is a global shortcut, which comes
from outside the application altogether; the other way round drew the note behind the help.
Nothing is lost by it: an About panel can only be opened from the titlebar, and the titlebar
is under the scrim while the editor, the trash panel or the tag manager is up.

`DialogStack` is what makes Escape reach **one** dialog. Every open modal listens on
`document`, so without it they all answer the same keystroke — which used to be patched case
by case (the editor checked whether the lightbox was open; the "About" menu's four panels
shared one signal so two could never stack). The stack orders by rung and not by arrival,
because a dialog opened _by_ another one — the fields form, from the palette — is created
second but drawn in front. It also answers `hasOpenDialog()`, which is how the notes page
knows to keep its hands off the keyboard: the page used to name its five modals one by one and
could not see the ones the titlebar opens at all.

**80 is reserved for the two banners** in `banners/` — `StatusToastComponent` and
`ErrorBannerComponent`. They sit in the flow under the titlebar, so without it a modal's fixed,
blurred backdrop covers them; and it is precisely from a modal that they get raised ("copied
with your field values" from the editor, "could not save the note" while editing). A new rung
therefore goes **below** that line, never above it.

### Syntax highlighting

`CodeViewerComponent` renders read-only coloured code — a card excerpt, or the layer under the
editor's textarea. It delegates to `notes/ui/code-viewer/highlighter.ts`, the **only** module
that imports highlight.js.

- **Grammars are imported one by one** from `highlight.js/lib/`, never the default bundle,
  which carries close to 200 languages. `GRAMMARS` maps a `LanguageTag` onto the grammar that
  describes it; five do not share a name (`toml` is `ini`, `html` is `xml`, `yml` is `yaml`,
  `rs` is `rust`, `cs` is `csharp`), and `txt` deliberately has none — free text has nothing
  to colour, so it is only escaped. The discipline is what keeps the cost legible: the six
  compiled languages added in v0.1.4 weigh 5.9 kB over the wire, all of it in the lazy
  notes-page chunk, which is where the viewer already lived.
- **Adding a language is five edits and no migration.** A variant in `closed_enum!`
  (`notes/language.rs`), a grammar in `GRAMMARS`, a label in `LANGUAGE_LABELS`, a
  `.lang-*` rule in `language-badge.component.scss`, and `npm run bindings`. The column
  stores the literal and carries no `CHECK`, so nothing on disk changes. Three
  `Record<LanguageTag, …>` tables — the grammars, the labels and the highlighter spec's
  samples — are exhaustive by type, so a variant added in Rust stops the front end compiling
  until each has an answer for it. ⚠️ The badge rule is the one edit **no type can catch**: a
  format with no `.lang-*` is valid CSS and simply draws bare text, which is how six of the
  nineteen ended up with no badge at all. `scripts/language-hues.test.mjs` is what catches it
  now, by reading the enum and the stylesheet off disk. A detection heuristic in
  `language::from_content` is optional, and it is **weights** now rather than an order —
  see below.

**Detection scores, and can say it does not know.** `from_content` guessed by _first match
wins_, down a hand-ordered chain of `if`s, and the order **was** the priority: a Rust
snippet carrying a match arm came back `js`, because `is_javascript` matched `=>` anywhere
in the text and sat near the end of the chain — which made JavaScript the default answer
rather than an answer. Fixing that meant finding the right slot in a list, and the next
language would need someone to find one again.

Every language scores itself and the highest score wins. Three weights and no more:
`SIGNATURE` (nothing else writes it), `STRONG` (strongly associated, occasionally shared),
`WEAK` (many languages write it). A scale with ten steps would be the same invisible
ordering written differently — the point is that a marker declares how much it _proves_.

⚠️ The two facts the old order encoded are weights rather than positions now: `<?php` is a
signature, so it outscores the markup shape its `<` also earns; and Rust's `println!` is a
signature where JavaScript's `=>` is the weakest marker in the file.

⚠️ **It can abstain, and that is a behaviour change.** Below `MIN_CONFIDENCE` — one weak
marker — or on a **tie**, the answer is `txt`. `class Note { }` used to come back `js` and
is left alone now: it is written the same way in six of these languages, and a wrong answer
costs more than none, since it colours the body, badges the card and files the note under a
facet in the rail. Some notes that were coloured before are not any more.

⚠️ `score_typescript` deliberately does **not** inherit JavaScript's markers: inheriting
them would make TypeScript score at least as much as JavaScript on every file, and a tie is
no answer.

⚠️ **`src-tauri/tests/corpus/` is what says whether this is better rather than differently
wrong.** One file per case, named `<language>[-<variant>].txt`, read from disk and asserted
in bulk — adding a case is adding a file. It earned its keep immediately: CSS's "a selector
and a declaration" scored a signature on `export interface Note {` with `id: string;` inside
it, **and** on PHP's `foreach (…) {` with an `echo …;` inside it, so both tied at six and
came back as prose. That shape is strong evidence now, not proof; a unit or a custom property
is what makes it a stylesheet.

⚠️ The languages still absent — C++, Kotlin, Swift, Ruby, Dockerfile, PowerShell and the
rest — were deliberately **not** added in the same pass. Each is five edits of its own, and
the whole point of the change is that adding one is no longer a slot to find by hand.

- **`ignoreIllegals` is on.** A note is free text, often a fragment that does not parse end to
  end; without it a truncated JSON snippet would throw instead of rendering.
- **Output is re-split into lines** by `splitHighlightedLines`. highlight.js colours the whole
  block — that is exactly what lets it handle a comment or string spanning several lines — but
  the viewer renders one element per line for its gutter. A plain `split('\n')` would cut
  through `<span>`s straddling a line break, so the splitter tracks the open tag stack, closes
  it at end of line and reopens it on the next.
- **The theme is global**, in `src/styles/_code-theme.scss`. The coloured HTML arrives through
  `[innerHTML]`, and Angular does not stamp `_ngcontent-*` on DOM created that way: a
  `.hljs-keyword` rule written in `code-viewer.component.scss` would be rewritten into
  `.hljs-keyword[_ngcontent-xxx]` and never match. No highlight.js stylesheet is imported —
  they hard-code hex values, where the rest of the app only reads theme variables.
- **The markup is never trusted.** highlight.js escapes the source text and the result still
  goes through Angular's sanitizer; nothing calls `bypassSecurityTrust*`, and nothing should —
  the content is typed by the user.
- Two inputs let a card reuse it: `showLineNumbers` (a gutter on a three-line excerpt is
  noise) and `compact` (no padding, no scroll, no font size of its own — the card decides).
  The viewer renders `<span>`s rather than `<div>`s for the same reason: a card is a
  `<button>`, whose content model only admits phrasing content.

### State

The canvas is held by **three** stores, all `providedIn: 'root'`, split by the question they
answer. They were one 800-line class, which is the shape a store takes when nobody asks what
it is _about_:

| Store                | Answers                                         | Depends on               |
| -------------------- | ----------------------------------------------- | ------------------------ |
| `NotesQueryStore`    | which notes the canvas shows                    | `SpacesStore`, the clock |
| `NoteSelectionStore` | which one it is pointing at, and what is ticked | `NotesQueryStore`        |
| `NotesStore`         | the open note: creating, writing, deleting      | both of the above        |

The dependency runs one way only, which is what makes each of them readable on its own. The
notes page injects all three under names that say which is which (`canvas`, `selection`,
`store`).

`NotesQueryStore` holds the **query state** — search text, active filter, selected tags,
selected languages — and the view the back-end returned for it. It does no filtering, sorting
or grouping of its own: those criteria are sent to `query_notes`, and `sections`, `allTags`,
`allLanguages`, `isFiltering` and `hasNoResults` are all reads of the resulting `NotesView`.

⚠️ Any new query criterion needs three edits in lockstep: a field in `QueryParams`, its clause
in `sameQueryParams`, and the copy into the `NotesQuery` the loader builds. `resource` compares
its params by identity, so a criterion missing from the comparator changes nothing on screen —
the rail looks wired and simply never refetches.

Two consequences worth knowing:

- **The search is debounced (`SEARCH_DEBOUNCE_MS`, 150 ms).** The field updates on every
  keystroke so typing never lags, but the query crosses the IPC bridge, and one round trip
  per character would be wasted work.
- **The last view is kept during a reload** (a `linkedSignal` over the resource). Otherwise
  each debounced keystroke would blank the canvas and the list would flicker between
  "Loading…" and the results. `isLoading` is therefore true only until the _first_ view
  arrives. That `linkedSignal` only retains what it has been read through, so everything the
  store exposes reads it, and `isLoading` does so without short-circuiting.

`SpacesStore` owns the spaces and the active one. Two decisions matter there:

- **The active space is a filter, not a label.** `NotesStore` injects `SpacesStore` (never the
  other way round) and sends `activeSpaceId()` with every query, so the space also scopes both
  facet rails — a tag or a language that filters nothing in the current space has no reason to
  be offered.
- **`null` means "all spaces", and is a choice, not a loading state.** There is deliberately
  no "All" row in the data: it would be a phantom space that notes could be filed into by
  mistake. The label lives in the translations, and an active id matching no known space
  degrades back to `null` rather than hiding every note.

Creating a note files it in the active space, falling back to the first one in "all spaces"
mode; with no space at all, creation is refused with a translated message, because a note
with no `spaceId` would vanish as soon as a space filter is applied.

`NoteSelectionStore` owns two things that are **not** query criteria and deliberately never
reach `query_notes`. They live together because they are the same idea — a **position in the
visible list** — and a range selection spans from one to the other:

- **The multiple selection** (`checkedIds`), derived through `checkedNotes` so that an id
  checked and then gone — note deleted, filter tightened — never reaches a bulk action.
- **The keyboard focus** (`focusedNoteId`), read against `visibleNotes`, the sections
  flattened in display order. Both the arrow keys and the Shift-range selection reason in
  **indexes** into that list, the only reference that survives a note being renamed.

`NotesStore` is left with the note itself: the open one, its unsaved draft, every write to it,
its deletion and the undo window. **One method writes a note's fields**, `applyPatch(id,
patch)`, and a table of per-field comparators (`UNCHANGED`) decides what actually moved — so
closing the editor on an untouched note makes no round trip. There used to be nine setters,
each restating that comparison, and a new field meant editing four files.

Smaller stores sit beside them, each for a screen or a job that is not the canvas:

| Store                  | Owns                                                                          |
| ---------------------- | ----------------------------------------------------------------------------- |
| `TrashStore`           | the trash panel: open/close, list, restore, purge, empty                      |
| `TagsStore`            | global tag management: list with counts, selection, rename/merge              |
| `LibraryStore`         | import, export and copy-out, each reporting through `StatusNotifier`          |
| `PaletteStore`         | the quick-paste palette: its own search, highlight and copy                   |
| `AttachmentsStore`     | the open note's attachments, the preview, and the three ways of adding one    |
| `PlaceholderFillStore` | filling a snippet's `{{fields}}` before copying, from card, palette or editor |
| `NoteCopyService`      | putting text on the clipboard, and saying so when that failed                 |

The first four know nothing of `NotesStore` — the reverse dependency exists and closing the
loop would be an injection cycle — so each writes through `NotesRevision` and the canvas
re-queries on its own. The last three do inject it, in that direction only: they orchestrate
_around_ the open note rather than being read by it.

⚠️ **`NotesStore` bumps that same revision**, rather than reloading the canvas it happens to
hold. There are **two** views of the same notes: `NotesQueryStore.queryParams` and
`BoardStore.queryParams` both read `NotesRevision`, and a write that reloaded only the first
one is exactly what left a todo list ticked on the board still showing unticked until the view
was switched. The same trap in the other direction: `NotesStore.find` resolves a note through
`NotesQueryStore.findVisible` **and** `BoardStore.findVisible`, because the board _dims_ where
the canvas _narrows_ — a card there can be ticked, moved or deleted while its note is nowhere
in the canvas view, and an unresolved note is a gesture that writes nothing, silently.

`TrashStore` and `TagsStore` load **on opening** rather than through a permanent `resource`:
neither is displayed anywhere else, and a resource would re-query on every deletion.

Rules of the house:

- **Writable signals stay private, exposed read-only.** A store field is
  `private readonly _x = signal(...)` plus `readonly x = this._x.asReadonly()`. Every
  mutation therefore goes through a method, which stays the single entry point the day a
  write becomes more than a `set()`.
- **The view is a parameterised `resource()`.** Its params are the query criteria plus the
  current **local day** — not `clock.now()`, since only the day affects section boundaries.
  The exact instant is read `untracked` in the loader. `resource.value()` **throws** while
  the resource is in error; read it through a `hasValue()` guard.
- **That params `computed` needs its `equal` comparator.** `resource` compares its parameters
  by identity, and `queryParams` builds a fresh object literal that depends on `clock.now()`.
  Stabilising the day _value_ is not enough — without `sameQueryParams`, every 30 s tick
  produced a new object, a new request, and a full `query_notes` + SQLite round trip, hidden
  by the retained view and by `isLoading` staying false. A spec covers it: a clock tick must
  not increment `queryCount`.
- **The back-end is authoritative; writes are not optimistic.** A mutation persists, adopts
  the returned note, then reloads the view. Nothing is applied locally first, so there is
  nothing to roll back on failure — an `ErrorNotifier` message is raised and the screen still
  shows what is actually stored. A write can move a note between sections, which is precisely
  why the view is recomputed rather than patched.
- **Ids, timestamps and normalisation come from persistence**, never from the front-end.
- Derived state is `computed()`, never a manually maintained signal.
- Formatting logic that needs no injection lives beside its subject (relative time in `core/services/time/`) as pure functions taking
  `now: Date` as a parameter.

### Display sections

Sections are built in Rust (`src-tauri/src/notes/view.rs`) and arrive ready to render.
The front-end preserves the order it receives and never drops or merges a section.

The classification into `pinned`, `today`, `week` and `older` is **exhaustive**: apart from
pinned notes, each note falls into exactly one section — a note belonging to no section is
unreachable in the UI, search included. An unparseable `created_at` lands in `older` rather
than disappearing. The `week` section is always present because it hosts the "paste or
create" ghost card.

As soon as a search query, a tag selection or an opened folder is active, the view collapses
into a single flat `results` section. Spreading search results across date sections dilutes
them and hides matches at the bottom of the page. A quick filter (`pinned` / `untriaged`)
does **not** trigger this: it narrows a view that stays chronological.

⚠️ `build_sections` takes `Option<bool>` rather than a bare `is_filtering`: `None` is the
chronological sections, `Some(ghost)` the flat list, and `ghost` says whether it is somewhere
a note can be **created**. The inside of a folder is — a note made there arrives filed, and it
is the one place where creating one files it — so the flat view keeps the ghost when the
folder is the whole reason it is flat. A result list is a list of what already matched, not a
place, so a search or a facet takes it away.

Day boundaries are **local**, so the query carries `tzOffsetMinutes` alongside `now`. Without
it a note created at 23:00 would be filed under the wrong day. Beware the sign: JavaScript's
`getTimezoneOffset()` returns −120 for UTC+2, the opposite of what chrono's `FixedOffset`
expects.

A section's key **is** its translation key (`'sections.' + key`), which is why the Rust enum
serialises to `"pinned"` / `"today"` / … and no user-visible label ever crosses the bridge.

### What a card's footer shows

The footer carries one of three things, and which one is a **product rule**, so it is decided
in `notes::model` and arrives as a tagged `footer` field:

| variant  | when                         | rendered as                |
| -------- | ---------------------------- | -------------------------- |
| `expiry` | the note has a deadline      | `expiryRef(at, now)`       |
| `source` | pinned, and it has a context | the first path segment     |
| `age`    | everything else              | `relativeTimeRef(at, now)` |

The dated variants carry a **date, not a label**: formatting stays on the front so "4 min ago"
keeps ageing on screen without a round trip. That is the line — the back decides _what_ to
show, the front decides _how_.

`expiringSoon` comes with it, computed against a single threshold in `notes::model`. It
previously lived only on the front (`isExpiringSoon`, 3 days) while the back separately
computed `has_expiring_notes` — two definitions of "soon" behind a hint that reads "to triage
soon". The section flag now derives from the same per-note value.

### The two kinds of note

A note is a `snippet` or a `checklist`, and `notes::checklist::NoteKind` is a closed enum for
the same reason `Language` is: the front receives a generated union, so an unhandled variant
stops compiling rather than surfacing at runtime.

A checklist has **no body**. Its items replace `content` — they are not an addition to it —
and they live in `note_items`, keyed `(note_id, position)`. That key is the whole design: an
item has no identity beyond where it sits, so every write replaces the entire list, exactly
the way `note_tags` does. Nothing addresses a single item, and nothing generates an id for
one. Reordering is therefore an ordinary write, not a shuffle of rows under a primary key
that forbids duplicates halfway through.

Three behaviours would go quietly wrong without a thought for the kind, and each is handled
where its rule already lived:

- `notes::view::matches_search` scans the item texts as well. A checklist has no `content`,
  so it would otherwise be findable only by its title.
- `transfer::model::to_markdown` renders `- [x] …` lines instead of a fenced block. An empty
  ` ```txt ` block is not something anybody pastes into a ticket.
- Language detection is skipped, at creation and on patch alike. There is no body to read,
  and a format select over a note that shows no code is a control with nothing to do — which
  is why the editor hides it too.

Both fields carry `#[serde(default)]`. `transfer::Bundle` deserialises `Note` itself, so a
required key would have made every export file written before todo lists unreadable —
`FORMAT_VERSION` stays at 1 precisely because old files still read.

**A bundle from a newer version degrades, it does not fail.** `Note` carries two closed
enums, `language` and `kind`, so a file written once `Language` has grown a `rust` variant
used to fail serde outright and take the other 499 notes with it. `read_bundle` now walks
the raw JSON first and brings any value this build cannot name down to the default, counting
the notes it touched; `ImportReport.notes_degraded` reports them, and `merge` counts only
those actually inserted, so re-importing the same file says nothing a second time.

That settles an inconsistency the two read paths had by accident. The **database** read has
always degraded (`notes::store`, `TryFrom<NoteRow>`: `row.language.parse().unwrap_or_default()`);
the bundle read refused. A bundle is the same data through another door, so it degrades too.
Degrading also loses less than skipping the note would: the title, body, tags and deadline
all arrive, only the colouring is dropped. What stays strict is the **bridge** — a `language`
the front end cannot name is still a deserialisation failure there, which is what lets the
generated union be trusted.

⚠️ Consequences for `FORMAT_VERSION`: an added enum variant **does not** bump it. It is not a
format break, since the file still parses. It is bumped when a file written today would stop
being readable — and it is read off the raw JSON before the bundle is built, so a genuinely
future format answers with the designed message rather than with a serde error about a field.

Progress (`done`/`total`) is **not** on the wire. The items already travel with the note, and
a counter beside them would be the identity mapper this codebase refuses elsewhere; the card
and the editor each count in a `computed()`. That is the same line as relative-time
formatting: presenting data the front already holds is the front's job.

What _is_ on the wire is `DisplayNote.copy_text`: `Some(markdown)` for a checklist, `None` for
a snippet, whose `content` is already there. Counting an array is presentation; deciding that
a task list reads `- [x] …` is a rule, and it now has exactly one home —
`notes::checklist::to_markdown`, which sharing and exporting already used. The front end held
a second copy of that syntax, and one of the two was going to drift.

### A card is two layers, and the click surface is the lower one

A `<button>` may not contain another, and a card's header carries three — the selection tick,
the copy (or ⚡) and the `⋯`. While the card _was_ the button, those three had to be siblings
positioned absolutely over it: each a few pixels off the badge's line, each reserving a
hand-written width in the header so the title would not run under them. That is what made the
top of a card read as scattered glyphs rather than a toolbar.

So the card is **not** the button. `.card-open` is an empty `<button>` at `inset: 0`
underneath, carrying the click surface, the keyboard focus the canvas moves around and the
card's accessible name; `.card` is a `<div>` above it in `pointer-events: none`, and each
control takes its own events back. The checklist's tickable items are simply in the flow
instead of a floating layer of their own.

**One row holds the tick, the title and the marks.** The marks — a pin, a format badge, a ⚡,
a 📎 — used to have a band of their own above the title: a line that says nothing about the
note, on a card that is a fixed 150px. They are right-aligned on the title's row now, the
title gives way to them (`flex: 1` plus `min-width: 0`, without which a long title pushes
them off the card instead of clamping), and the body is what got the line back —
`SNIPPET_LINES` went from four to five.

⚠️ `MAX_VISIBLE_ITEMS` deliberately did **not** move. A todo list with nothing to mark never
drew that band, so it gave nothing up; and 21px recovered is less than the 24px a row needs
now that every row clears the hit target.

⚠️ **The tick is in the flow**, at the head of that row, and always drawn — quiet at rest,
amber once ticked. It used to float in a pill over the card's top-right corner, which is
precisely where the marks now are: anything floating there sits on top of them, and that
collision is what decided this layout. A selection affordance that exists only under the
pointer is also one nobody finds.

**The card's ⋯ menu is the complete one.** Open, pin, copy, file into…, move to…, delete —
a note's properties used to be spread over four surfaces with none of them complete: filing
one from the date view took three clicks through the selection bar plus a fourth to clear
it, and pinning with the mouse took a full-screen modal for a boolean that has its own
filter chip in the header. The editor gained the two placements it was missing, as
`placement-menu` used twice.

⚠️ **The two placement controls are shown together**, and that is not decoration: a folder
belongs to one space, so `NotePatch::apply` clears `folder_id` whenever `space_id` moves.
A folder control without the space control beside it would go on naming a folder the space
switcher can no longer reach.

⚠️ **Filing is not a `NotePatch` field.** `NotesStore.fileNote` is a **batch of one**
through `file_notes`, the same command the selection bar takes, because that command
answers the placements it actually changed and that answer is what the undo puts back. It
is also the one write with no note to adopt — `adoptFiling` refreshes the open note from
the filing that was accepted and the folder list the control offered, or the editor's own
control goes on naming the folder the note just left until it is closed and reopened.

⚠️ **The card's menu is `position: fixed`**, placed by the component from the trigger's own
box and flipped above it when that is where the room is. The card sits inside the canvas,
which scrolls, so an absolutely placed panel is clipped by it — and the menu became tall
enough for that to matter the day it became the complete one: it lost its last entry, the
delete. ⚠️ The move-to-space list is flat and grows with the library; past a handful of
spaces this wants the command palette rather than a longer menu.

⚠️ `placement-menu` swallows Escape while it is open, and it is the **only** menu in the
application that has to: it lives inside a dialog. `MenuTriggerDirective` lets Escape
bubble on purpose — a multi-level menu folds its panel before closing — but here the next
listener up is the editor's own, so one Escape closed the note along with the menu. It also
closes itself from that handler rather than from `escaped`, so the decision does not depend
on which listener was registered first.

⚠️ **The two actions hang outside the card**, in a pill over its **bottom**-right corner,
shown on hover and on focus. Outside rather than in the flow, so they cost the card no room
at all; at the bottom, because the top belongs to the marks. `.card-arming` — the red
"delete again?" band — owns the footer line inside the card, and the pill stands down while
it is asking: a copy button and a ⋯ beside that question are noise, and `:focus-within`
keeps the keyboard's way in. ⚠️ `.zone-body` scrolls, so it clips — its bottom padding is
what keeps the last row of cards from losing their pill on the board.

It is the same trick the editor uses for its body, where the code viewer sits under the
textarea; the card had it already, for its items alone.

Ticking from the card matters more than it looks: crossing tasks off is the gesture a todo
list exists for, and routing it through the editor would put a modal between the user and a
one-click action.

### The update prompt shows the release's notes

`update-prompt` has rendered `update.notes` since it was written, and `latest.json` never
carried the key — so the block was dead markup that looked like a shipped feature, and
`update.body` was `undefined` at every release.

The text already exists at that point in the release job: `scripts/release-notes.mjs extract`
writes it to `$RUNNER_TEMP/release-notes.md` for the GitHub release body, and the manifest
step reads the same file. ⚠️ `generate_release_notes` appends the raw pull-request list
inside `action-gh-release` and does not touch that file, so the manifest carries the
hand-written section alone.

⚠️ **Shown open, at a fixed height.** Behind a `<details>` it was installed without being
read — and worse, opening it _changed the dialog's height_: the body is bounded at 160px but
a collapsed summary is one line, so the panel grew under the cursor and moved "Install now"
out from under the mouse. The prompt is a `fitted` dialog, sized by its content, so a
**fixed** block is precisely what makes its geometry independent of a two-line release versus
a forty-line one. It scrolls inside.

It stays untranslated, like `CHANGELOG.md` and the "Nouveautés" panel that reads it: a
release note in one language beats no release note. The `<pre>` shows the section as
written, bullets and all.

### The theme is one gesture, where it was three

The theme and the language are the same kind of choice, and they sat three gestures apart:
the language in the titlebar, the theme behind File → Préférences → Apparence. There is a
theme control beside it now, and it is a **toggle between light and dark**. It used to cycle
through the three `THEME_CHOICES`, and a click on "system" usually changed nothing on screen
— the one thing a click there is expected to do (#323). Following the system is chosen in
the panel, which keeps its three rows.

⚠️ The control reads `resolvedTheme`, **what is on screen**, not the setting: on "system" it
shows what the machine resolved, and a press writes the explicit opposite. It writes
`SettingsStore.theme`, the **same signal** the panel writes, so the two cannot disagree.
⚠️ Icon and label both name the **action** — a moon on a light screen, "Passer au thème
sombre" — the convention a toggle follows. An icon rather than a word: the language beside
it is two letters, and a label would be the widest thing in a bar whose middle is the
application's name.

⚠️ Both switches sit in **one** group. `.titlebar` is `space-between` and the title takes
no part in that row, being absolutely positioned, so two loose groups put the theme in the
middle of the bar beside the application's name.

### One row, and the first card in the top half

Measured on the assembled application at 1100x720, fresh install, library rail open: the
titlebar, **two** rows of topbar, the tag rail, the language rail — and the first card at
**284px**, 39% of the way down. `+ Nouvelle note`, the primary action, sat on the second
row, and the search was squeezed below its own placeholder. Ticking a card inserted the
selection bar as a further band and pushed everything down 85px, so the canvas jumped on
every tick.

It is **151px** now, and the topbar does not wrap: `flex-wrap: nowrap`, with the search as
the only thing that gives way — it is the one control here that can be narrow and still be
itself. Four changes got it there:

- **The two facet rails went behind a disclosure** (`facets-panel/`). They were two
  permanent 44px bands, the single biggest saving. ⚠️ The disclosure is forced open
  whenever a facet is selected and **refuses to fold** while one is: a filter nobody can see
  is a filter nobody can undo, and a canvas silently showing a third of the corpus is worse
  than the bands were. Its trigger is in the topbar and the panel below it, so `notes-page`
  owns the state — a component cannot be in two rows at once.
- **The quick filters became one segmented control.** Three states of one thing, where four
  chips read as four filters that could be combined and took the width of four.
- **The trash left the filter chips**, and the six pixels it sat from the primary action. It
  is a **view** on the notes rather than a filter, so it is icon-only beside the rail toggle
  — still not in the File menu, for the reason it never was.
- **The selection bar floats** over the canvas region rather than being inserted above it.
  ⚠️ It has to be a child of `.canvas-region`, which is the only positioned ancestor: left
  where it was, it hung against the window and covered the search field.

⚠️ **The measurement is the spec**, in `06-search-and-filters`: at 1100x720 with the rail
open, every topbar child shares a row, the row does not overflow, the first card is in the
top half, and ticking one moves nothing. That is the thing that will regress, and the only
way to catch it is to measure the assembled application.

### Nothing wears the operating system's chrome, except one popup

The application draws every one of its own surfaces, and then **eight** controls came from
the operating system and brought its chrome with them: a different border, a different
radius, a different arrow, a different focus ring and, on Windows, a different font.

Two of them were worse than a mismatch — the selection bar's "Déplacer vers" and "Ranger
dans" were a `<select>` doing a **command's** job, resetting their own value to `''` after
every `change`. A screen reader announced a combobox whose current value was "Ranger dans",
and a keyboard user got a listbox where the rest of the application gives a menu.

**`notes/ui/choice-menu/` replaced seven of the eight**, on the existing menu directives:
the two selection-bar commands, the editor's language, the editor's space and folder, the
space editor's refuge and the preferences' language. ⚠️ `naming` is what tells the two
shapes apart — `'value'` names what is chosen and refuses to re-emit it, `'label'` names
what the control _does_ and has no current value at all. It lives in `notes/ui/` beside
`copy-button` because it crosses two zones of `notes/`, and it is the one menu that
**swallows Escape**: it is used inside dialogs, where the next listener up is the dialog's
own.

**Three options get a segmented control instead** (`shared/controls/segmented-choice/`):
Thème and Densité. Hiding two of three choices behind a click buys nothing, and
`role="radiogroup"` is what says exactly one of them is chosen at all times.

⚠️ **The date input stays native, deliberately.** An `<input type="date">` brings a
calendar, a localised format and keyboard entry that a hand-rolled picker has to earn back —
so its **box** is the application's and only its popup is the system's. Its picker icon is
painted by the engine and follows `color-scheme`, which `:root` sets per theme; the invert
filter is therefore dark-only, and it lives in `styles.scss` rather than in the editor's own
stylesheet because the rule has to open on `:root`, which emulated encapsulation rewrites
into a form that never matches `<html>`.

### Motion, and the one block that stops it

There were **four** `transition` declarations in the whole of `src/`, across roughly 225
style rules, and three of them had already drifted to three different durations. Nothing on
a card's hover moved, no menu or dialog opened, no transient bar arrived — every state
change was a hard cut. That is not a defect; it is the difference a reader registers between
"quick" and "finished".

**Two durations, and they are variables.** `--motion-fast` (120ms) is what follows the
pointer — hover fills, chip states, the card's action pill. `--motion` (160ms) is what
**enters**: a menu, a dialog, the selection bar, the undo banner, the status toast.

⚠️ **`transform` and `opacity` only.** A transition on `height`, `width` or a colour over
a list of cards is what makes a canvas feel heavy, and this canvas renders every note with
no virtualisation. `@mixin eases` therefore names its properties and never takes `all`.

⚠️ **An animation, not a transition, for what arrives.** Menus, dialogs and the bars are
`@if`-rendered: there is no previous state to transition _from_, because the element did
not exist a frame ago. `@mixin enters` picks one of three shared `@keyframes`
(`enter-rise`, `enter-lift`, `enter-fade`), which live in `styles.scss` because keyframe
names are global whatever the component encapsulation says.

⚠️ **`prefers-reduced-motion` is one block, and it has to stay the only one.** It reduces
every duration and every animation without naming any of them, which is the whole reason
the durations are variables and the entrances are three shared keyframes: motion added
later is turned off by it for free. `scripts/motion.test.mjs` holds all three rules
against the shipped stylesheets — no duration written out in a component, no
`transition: all`, and exactly one reduced-motion block.

⚠️ The card's action pill keeps its entrance to **opacity**. `note-card.component.spec.ts`
and two e2e scenarios read its box while it is at `opacity: 0`, which works because opacity
does not affect layout; a `transform` there would move what they measure. And ⚠️ its
`opacity` pair has to stay **after** `unstyled-control` wherever it is written — that mixin
expands to `all: unset`, which resets both.

### Reordering, and why not drag & drop

⚠️ **HTML5 drag & drop does not work in this WebView.** `dragDropEnabled` is Tauri's default
`true`, which is what makes a file dropped on the window reach `FileDropService` at all; with
it on, the WebView never sees `dragstart` or `drop`. Turning it off to get the DOM events
back would break attachments.

So `ChecklistEditorComponent` reorders with **pointer events** — `pointerdown`,
`setPointerCapture`, `pointermove`, `pointerup` — and the capture is what keeps a slightly
fast gesture from being lost the moment the cursor leaves the row. The call is optional
(`?.`): capture makes the gesture comfortable, it does not condition it.

`Alt+↑/↓` does the same thing from the keyboard, and it is not a bonus: the template
accessibility rules are errors here, and an interaction only the mouse can reach does not
ship. The visual preview during a drag is CSS `order`, so rows keep their place in the DOM —
and with it their focus and their caret — while only their position moves.

### "Untriaged" — the ephemeral note

In the product model, a note carrying a deadline is a note whose fate has not been decided
yet: that is what the `untriaged` quick filter selects, what the `⏳` lifecycle badge shows,
and what the "to triage soon" section hint counts. All of it hangs on one field, `lifecycle`.

The deadline is set from the editor's date field, next to the badge: an empty field means
permanent, a date means expiring. The value is turned into the **end of the local day**
(`endOfLocalDay`), not midnight — a note dated today would otherwise be expired the moment it
was set — and the reverse conversion is local too, or the field would show the previous day
west of Greenwich. This is the same class of exception as relative-time formatting: an
`<input type="date">` value is a UI representation, not a business rule.

Until that field existed, every note was created permanent and nothing could ever change it,
so the filter, the badge and the section hint were all reachable but permanently empty.

### The card actions menu

`NoteCardMenuComponent` is the `⋯` menu on a card: move the note to another space, or delete
it. It is a separate component from `NoteCardComponent` because it brings what the card has
none of — open/closed state, a document-click listener, focus management — leaving the card
purely derived from its note.

Two structural consequences:

- The card's root is a `.card-shell` wrapper, not the `<button>` itself. A `<button>` may not
  contain another `<button>`, and the menu trigger is one. The shell also anchors the menu
  (`position: relative`) and is what the trigger watches to appear on hover
  (`:host-context(.card-shell:hover)`).
- The pin indicator moved out of `.card.pinned::after` into the first row of the card, since
  the menu now occupies the top-right corner.

The trigger stops propagation: the whole card is a button, so without it a click on `⋯` would
open the editor at the same time as the menu. The trigger is `opacity: 0` rather than
`display: none` — hiding it would take it out of the tab order and make the menu unreachable
by keyboard. The menu emits no note id (it does not know one); the card attaches it, the same
way the editor lets the store decide which note is open.

### The library rail

Navigating used to take two dropdowns that knew nothing about each other: the space switcher
listed the spaces, and beside it the folder switcher listed the folders of whichever space was
active. The two lists are a **tree**, and they were drawn as two flat menus opened one after
the other. `notes/sidebar/library-tree/` draws that tree instead — every space, its folders
under it, the way an editor holds a project — and the toolbar keeps the toggle that shows and
hides it (`Ctrl+B`, and an entry in the shortcuts sheet, because the table that binds a key is
the table that documents it).

⚠️ **The rail replaces the two switchers while it is open**, and they come back when it is
closed. Two places to change space is how a tree and a dropdown drift apart; the `@if` in the
page's template is what keeps there being one.

**Nothing moved house.** A space row keeps its `⋯` — pin, rename, delete-with-refuge — and a
folder row keeps its own — rename, recolour, delete. Those panels are `space-editor/` and
`folder-editor/`, projected by whoever shows them; extracting the first one out of the
switcher is what let the rail have it without a copy. Creating a space sits at the head of the
rail, creating a folder under the active space alone — a folder is made in the space one is
in, and the row above is one click away from making that so.

**The rail asks for a destination; the page turns it into state.** `folderOpened` carries the
whole `Folder` rather than an id, because opening one in another space means switching space
_first_: `FoldersStore.activeFolder` resolves against the active space's folders, and the two
signals settle in that order. Choosing a space leaves whatever folder was open — the row means
the space itself.

⚠️ **`FoldersStore` holds every space's folders**, not the active space's. The rail needs the
whole library, and partitioning a list already in hand is what lets a folder in another space
be opened without a round trip in between; `folders()` narrows to the active space for the
switcher and the selection bar, `foldersOf(id)` for anyone who names one.

Shown or hidden is `AppSettings.showLibraryRail`, and how wide is `libraryRailWidth` — one
line each in `SettingsStore`, restored at launch like every other preference, and deliberately
absent from the preferences panel: they are states the window is in, not decisions to go and
make. ⚠️ The width is dragged from the rail's edge with **pointer events**, like every other
drag in this application — HTML5 drag and drop does not work in this WebView — and the edge is
a `role="separator"` so the arrow keys move it too. It is clamped on the way in _and_ on the
way out (`RAIL_WIDTH`), because a preferences file written by hand is an input like any other.

**The floor is the tree's own, not the panels'.** It stopped at 220px, and that number had
nothing to do with the rows: `space-editor` and `folder-editor` carry a `min-width` of 240px,
which is what gives a _dropdown_ a width at all. Projected inline between the rows they need no
such thing, so the minimum is handed down as `--editor-min-width` and the rail sets it to `0` —
a custom property, because a rule spelled in the rail's stylesheet is rewritten with the rail's
own `_ngcontent` and can never reach into a component's. The rows inside then **wrap rather than
squeeze**: under about 230px the submit drops below the field. The floor is 160, and so is the
default, the width being remembered like the window's own geometry — the default is a first
launch and nothing else.

⚠️ **A row's name is ellipsised by `.node-label`, a box of its own.** `text-overflow` is a
property of a block container and `.node-name` is a flex one, which ignores it outright: the
name was cut mid-letter. Nothing said so for as long as the rail could not be narrow enough to
cut one.

### Managing spaces from the switcher

The space switcher's dropdown has three mutually exclusive states: the menu, the creation
form, and the per-space edit panel (rename + delete). Each **replaces** the menu instead of
nesting inside it — a text field or a `<select>` inside a `role="menu"` is neither valid ARIA
nor navigable the way options are. Escape unwinds one level at a time.

The delete control only appears when another space exists to receive the notes; with a single
space the panel explains why rather than offering a button that could only fail. Each space
row is a `role="none"` wrapper holding the select button and the `⋯` trigger, so the menu
keeps its direct menuitem children. Arrow-key navigation stays on the select buttons only.

### Folders inside a space

A space is one flat pile ordered by when things were typed, which is the right default for
"what was I doing yesterday" and useless for "where do I keep the SQL I wrote about indexing".
A **folder** cuts a space into named, coloured regions, and a note belongs to exactly one or to
none.

`folders.rs` is a feature like any other, with its own model, store and commands. What it
holds is deliberately small:

| Piece                      | Where it lives                                              |
| -------------------------- | ----------------------------------------------------------- |
| name, colour, owning space | the `folders` table — domain, and it travels with an export |
| which folder a note is in  | `notes.folder_id`, nullable, `ON DELETE SET NULL`           |

⚠️ **`ON DELETE SET NULL`, never `CASCADE`.** Deleting a folder must not delete a note. It is
also why `delete_folder` takes no refuge argument where `delete_space(id, targetSpaceId)`
must: "no folder" is a legitimate state, so the notes simply come out loose. `spaces/`
cascades into `folders/`, and that cascade then fires the `SET NULL` — a space's notes reach
their refuge unfiled, which is correct, since a folder of the deleted space no longer exists.

⚠️ **A folder belongs to one space, so a note cannot be filed across one.** `file_many`
narrows to the folder's own space, and `NotePatch::apply` clears `folder_id` whenever
`space_id` moves — otherwise a card would show a chip the space switcher can never reach.
That rule lives in `notes/model.rs`, where a test reaches it without a database.

**The colour is assigned, not chosen.** `FolderColour` is a `closed_enum!` of the five theme
accents, and `create` rotates through them by the number of folders the space already holds,
so two made back to back differ. It is changed afterwards from the switcher's edit panel.
Counted rather than random: a deterministic colour is one a test can assert and a user can
predict. Every consumer reads it through the `folder-hues` mixin as `--folder-hue`, so the
five-colour map exists once.

**Filing is a batch with a command of its own**, not a `NotePatch` field. `file_notes` answers
the `NoteFiling`s it actually changed — which folder each note _left_, `null` for a loose one
— and `file_notes_back` undoes exactly those. That is the shape `move_notes` already has, and
for the same reason: rebuilding the set from the selection would unfile a note the batch never
touched. A note already in the target folder is not reported, so a batch that changed nothing
opens no undo window.

**The card says where a note lives, and the back end is what resolved it.**
`view::apply_folders` decorates `DisplayNote.folder` in a pass of its own, exactly as
`apply_attachment_counts` and `apply_global_defaults` do. The front end never joins a
`folderId` against a list it happens to hold. ⚠️ The chip is a **square swatch on a neutral
pill** — never a coloured pill, which is what a tag is. A note with no folder gets no chip at
all: the absence reads on its own, and an "unfiled" chip would soil every loose card.

Filing happens from the selection bar, beside "move to another space"; the same control both
ways, since taking a note out is a filing with no folder. The switcher in the header narrows
the canvas to one folder and is where folders are created, renamed, recoloured and deleted —
the same three-panel shape as the space switcher (see above). It offers no "unfiled" filter
entry, for the reason the chip does not exist: that would be a second way to say the same
thing.

⚠️ **The board geometry is not in any of this.** Where a folder's zone sits, and where a loose
note sits beside it, are local state that must never enter the `Note` or `Folder` models —
`transfer::Bundle` deserialises them itself, so a coordinate stored there would travel in
every export and land on top of the receiving machine's own arrangement.

### The board: the second view of a space

A switch in the header chooses between **Date** and **Tableau**. The date view is not
replaced and stays the default; the board is another way to look at the same notes, and it
is remembered **per space** — one key each, `devnotes.notes.view.<spaceId>`, so arranging one
space does not switch the others. ⚠️ It is unavailable on "all spaces", where a folder
belongs to no board: the switch disables rather than disappears, and falls back to the date
view.

**The board has a query of its own.** `board_view` answers folders and positions, not
sections — `build_sections` must never learn about a folder. It reads the whole space and
marks each note `matches`, because the search, the quick filter, the tag rail and the
language rail all **dim** on the board rather than narrow it. ⚠️ Reflowing the survivors
into a list would throw away the spatial memory the board exists for, which is the one
thing the date view cannot give.

**Cards flow inside a zone and sit freely outside it.** That is the whole geometry:

| What                 | Where it is stored                                   |
| -------------------- | ---------------------------------------------------- |
| a zone's frame       | `folders.x/y/w/h`, nullable — all four move together |
| a loose card's place | `note_positions`, one row per **unfiled** note       |
| a filed card's place | nowhere: it flows, in canvas order                   |

⚠️ A `note_positions` row means "this note is loose". `file_many` deletes the rows of the
notes it files, so the table's meaning stays exact and a zone never has two competing
notions of where a card is. It is also why `#133` can say the inside of a folder is not
spatial without contradicting anything.

**The first layout is materialised, not computed on the fly.** `store::board::geometry`
reads the frames and positions and writes one for anything that has never been laid out, in
a single transaction. ⚠️ A read that writes, deliberately: computing a place without storing
it would let the first drag land next to cards that have no stored place of their own, and
the board would look shuffled at every launch. It is idempotent — the second read of a space
writes nothing — and a folder added later lands in the slot reading order gives it rather
than on top of the first zone, because the arrangement is computed against every folder and
only the missing frames are written.

The layout rules live in `folders/board.rs`, which imports neither Diesel nor Tauri:
`arrange_zones` flows zones three across, each row clearing the tallest zone above it;
`arrange_loose` flows loose cards four across underneath; `surface` sizes the pannable area
from whatever reaches furthest, never under 960 × 540. ⚠️ That is how far the board **pans**, not how
much of it takes the pointer: `.board-surface` is `min-width` / `min-height: 100%` of the
visible ground, or every gesture stops at the content's edge on a large window while the
dotted ground beyond it looks exactly the same (#320). **Pan only, no zoom** — a zoom is a second thing to persist
and to reset, and full-size cards are what makes panning worth having.

⚠️ A card that has never been placed takes `free_slot` — the first seat of that flow nothing
is standing on — and never the seat its index in the list would give it. A note created now
is the most recently updated, so it arrives first and used to be written on top of whichever
card the board's very first read had put there. "Standing on" is a rectangle test against the
placed cards _and_ the zones, not an equality one: a card dragged by hand almost never sits
exactly on a seat, and one half over a seat hides what lands there just as well.

⚠️ **A zone grows to fit and never shrinks.** `store::board::grow_to_fit` runs from
`file_many` and from `restore_filings`, so both ways in are covered and so is the undo. A
frame is otherwise computed once and never again, which is how a card filed into a full zone
ended up out of sight behind its scrollbar. Rows are counted against the zone's **own** width
(`columns_in`), not the nominal two columns — a zone widened by hand fits more across.
Shrinking was refused: it would move the board under the pointer every time a card is taken
out, and a zone somebody stretched is a zone they chose the size of.

⚠️ **`columns_in` predicts the browser, so it measures the zone the browser’s way**: the
hairline off, the padding off, the `SCROLLBAR` allowance **not** off. That allowance belongs
to `default_zone_width`, which reserves it so the nominal zone still shows two cards across
when it is too short for them; subtracted from the count as well it made Rust disagree with
the flow. A zone dragged 14px narrower than nominal was told it held one column while
`.zone-body` kept flowing two, and the next card filed in bought a second, empty row — 162px
of dotted board under the cards (#284). Four numbers now live on both sides of that
prediction — the card’s width and height, the gap, the zone’s padding and its hairline —
and `scripts/board-geometry.test.mjs` reads them out of the stylesheets and out of `board.rs`
and fails when one moves alone, because nothing else would: both sides are valid on their
own and the symptom is a band of empty board. It pins `GRID_PX` against the dotted
background it snaps to for the same reason. ⚠️ `ZONE_HEADER` is **not** in it and cannot be:
the header’s height is its padding plus wherever the text lands, and no stylesheet says so —
so `19-board-gesture` measures the drawn header against the constant instead.

⚠️ **The hairlines cost height as well as width, and that half bites harder.** `.zone-body`
is the box that scrolls, so a zone one pixel short of its own rows shows a **vertical**
scrollbar, the scrollbar takes a slice off the row, the row wraps, and the taller content
keeps the scrollbar: two cards across become one and nothing gets them back. Measured in the
assembled application, a 374px zone gave its body 335px of client height where two rows need
336 — one pixel, and three cards came out in a single column. `zone_height` and
`MIN_ZONE_HEIGHT` pay for the two hairlines now, and the sweep checks both halves. The same
measurement put the scrollbar at **9px**, not the 18 `SCROLLBAR` reserves; that constant only
ever widens `default_zone_width`, and staying generous there costs a few pixels of board
where being exact would cost a column.

⚠️ **There is no "no folder · N" count any more, and its history is why.** It was drawn
above whichever loose card was highest, and climbed over the zones as soon as one was dragged
up — off the top of the board when the offset went negative. Pinning it to the board's own
corner cured that and bought the opposite: a chip anchored to the corner of a surface that
**pans** sits over whatever the surface puts there, and `arrange_zones` lays the first zone at
`BOARD_MARGIN` — in exactly that corner, so its title read "…s · 2" (#295). Pushing the layout
aside does not hold either: a zone dragged to `x=0` takes the corner back. Moving it into the
header would have worked, and it was built that way first; what the third look said is that a
marker nobody can find a place for is a marker nobody needs — the loose cards **are** the
count, sitting on the background where a filed one cannot be. Deleted rather than relocated
a third time.

⚠️ `apply_folders` deliberately does **not** run for the board: a chip naming the zone a
card already sits in is noise, and a loose card has no folder to name. The card component is
the same one the canvas draws, so it renders no chip simply because `folder` is `None`.

On the front end, `notes/canvas/` is the region and the two views are alternatives inside
it — `note-section/` for the date view, `board/` for the board, and `note-card/` risen to
their nearest common ancestor. ⚠️ The region keeps the `canvas` test hook whichever view
fills it: it is the address on screen, and the e2e helpers wait on it.

### Arranging the board

⚠️ **HTML5 drag and drop does not work in this WebView and cannot be turned on.**
`dragDropEnabled` has to stay `true` for the native file drop that feeds attachments, which
is exactly what stops the WebView from ever seeing `dragstart` and `drop`. Every gesture on
the board is therefore pointer events — `pointerdown` + `setPointerCapture` +
`pointermove` + `pointerup` — as checklist reordering already is.

Four gestures, one state machine (`board.component.ts`), and the arithmetic in plain
functions next to it (`board-gesture.ts`, which has no signals and no DOM):

| Gesture       | Started from         | Ends as                                     |
| ------------- | -------------------- | ------------------------------------------- |
| move a card   | anywhere on the card | `file_notes` — the zone under it, or `null` |
| move a zone   | the header's ⠿ grip  | a frame in the layout batch                 |
| resize a zone | the corner handle    | a frame in the layout batch                 |
| draw a zone   | the empty background | a folder, named on the spot, at that frame  |

**The card is its own handle.** There used to be a ⠿ grip in its corner, on the grounds that a
drag started on the card "would have to swallow its own click" — but pointer events already
tell the two apart by the distance travelled, so swallowing that click is one flag, not a
design. ⚠️ `onCardActivated` drops exactly the click a committed drag leaves behind, and
clears the flag on the next press: an abandoned drag must not eat a later click. ⚠️ A press on
one of the card's own controls (`isCardControl` — the tick, the copy, the ⋯, a checklist item)
starts nothing, because it is aimed at that control.

The grip's real cost was where it sat: in the corner the selection tick occupies, drawn over
it with a higher `z-index`, taking every pointer meant for it. The zone's own grip stays — a
zone has no click of its own to tell a drag from.

⚠️ **The gesture commits on `pointerup` and nowhere else.** A drag that never travelled
past `DRAG_THRESHOLD_PX` writes nothing — it was a click — and `pointercancel` throws the
whole thing away rather than leaving a card at coordinates nobody chose.

**A card being dragged is drawn on the surface, wherever it was grabbed from.** A loose card
has a position of its own, so `positionOf` simply moves it. ⚠️ A **filed** card has none — it
flows inside its zone — so a drag out of one used to carry nothing at all: the seat faded and
the pointer held empty air, with only the zone lighting up underneath. `travelling` draws that
card as a ghost at the pointer, transparent to it (the surface below is what decides where it
lands), starting from the seat `CardGrab` measured so it keeps its grab offset. The seat stays,
faded, until the pointer lifts: the drop can still be cancelled, and a hole opening mid-drag
would reflow the zone under the pointer.

**Moving a zone carries its notes, and resizing one captures and releases nothing.** Both
fall out of the flow rather than being coded: a filed card has no coordinates, so it is
carried by its zone for free, and a stretched frame has nothing to capture. ⚠️ Unreal's own
rule — a comment box owns whatever it overlaps — was considered and refused: it silently
refiles notes the day a frame is stretched. **Membership comes from the drop, in both
directions, and from nothing else.**

**Everything lands on the grid.** The surface draws a 20px dotted lattice and every gesture snaps to it (`GRID_PX`), so two zones dropped roughly side by side come out exactly aligned instead of three pixels off. The rounding lives in the three arithmetic functions and nowhere else: ⚠️ both corners of a drawn band are snapped rather than its size — rounding a width would leave the far edge between two dots whenever the near one moved — and a resize is snapped **before** it is clamped, because the minimum is `folders::board`'s and is not a multiple of the grid. ⚠️ Snapping cannot turn a click into a move: nothing is computed until the pointer has passed `DRAG_THRESHOLD_PX`, and a stored position is left alone until something is actually dragged.

**One write per gesture.** `BoardStore` stages what moved and writes it behind a 400 ms
debounce as a single `save_board_layout`, one transaction. ⚠️ The staged geometry is laid
_over_ the view rather than written into it, and it is let go of when **a view comes back
carrying it** — never when the write returns. `reload()` only _asks_: the view still on
screen is the one read before the drag, so clearing on the write uncovered it for a whole
round trip and the card was drawn back where it came from before settling. The two staged
maps are `linkedSignal`s sourced on the view, so a place the view agrees with is dropped and
anything else stays; a place the view says nothing about is **kept**, since a card dropped a
moment ago is exactly what the overlay is for. A failed write keeps the overlay too —
dropping it would snap every card back with nothing on screen saying why.

**There are three of those maps, and the third was the one missing.** A frame, a place —
and a **membership**: which folder a drop decided a card is in. Without it a card dropped
into a zone was drawn again in `loose`, at the place the last view gave it, until
`file_notes` returned and the reload landed; one dropped out snapped back into its zone for
exactly as long. `zones()` and `loose()` therefore repartition every card the view carries
rather than reading the view’s own two lists, and `sittingIn` asks `has` and not `??`,
because `null` is a value here — it is the background, and a staged `null` coalesced away is
a card that never leaves its zone. ⚠️ One difference from the other two, deliberate: a
**refused** file is dropped where a refused place is kept. A place the server would not take
is worth leaving on screen with a banner beside it; a membership it would not take is a lie
about which folder a note is in.

⚠️ `save_board_layout` skips a card that has been filed since the drag: a position row
means "this note is loose", and writing one back would undo what `file_many` just did.

**The keyboard twin is not optional**, and it is the path that already existed: tick cards
with `X`, then "Ranger dans" in the selection bar — the same batch command the drop uses,
so the two cannot drift. The gesture adds to it; it does not replace it.

**A right-button drag sweeps a selection band; the left one still draws a folder.** The
existing gesture does not move — it is the one that _creates_ something, and it had the
background first. ⚠️ Two things that settles and one it opened:

- **Zones are not selectable.** A band picks up cards only, so "delete the selection"
  cannot mean two different things. A zone keeps its own gestures — move, resize, its ⋯
  menu — and selecting a whole folder's notes is `checkFolder`'s job, from that menu.
- The existing selection bar acts on the result, so move / file / tag / copy / delete come
  for free, and `Ctrl+click` already added to a selection on both views.
- ⚠️ **The application has no context menu anywhere**, so `contextmenu` is suppressed on
  the **whole board** — without that, the browser's own menu opens at the end of every
  sweep, right where the pointer was lifted. The surface alone was not enough: the event
  goes to whatever is under the pointer when the button comes up, and pointer capture does
  not retarget it, so a sweep ending past the surface, over the tidy control or over the
  header opened it anyway (#321). A right-button press therefore arms a one-shot guard on
  `document`, released by the menu it swallows or by the next press anywhere — so a text
  field's own menu still opens afterwards. A right _click_ that never travels does nothing,
  as it did before.

⚠️ The band is the one gesture that runs while `editable` is false: it writes nothing to
the board, it only ticks what is already drawn. And ⚠️ which cards it swept is **measured
off the screen**, not computed — a card filed into a zone _flows_ and has no coordinates of
its own, so there is nothing in the model to test a rectangle against. `overlaps` is the
arithmetic, half-open on the far edges like `contains`, and touching is enough.

**A selection is resolved against whichever view is drawing the cards.**
`NoteSelectionStore.onScreen` reads `BoardStore.visibleNotes` while the board is showing
and `NotesQueryStore.visibleNotes` otherwise, and the ticks, the range and the keyboard
focus all go through it. ⚠️ The board **dims** where the canvas **narrows**, so a card the
search filtered out of the date view is still drawn on the board and still filed in its
folder — resolved against the canvas it left the selection the instant it was ticked, and
the bar said nothing was selected.

`checkFolder(folderId)` ticks every note on screen filed there, which is one rule serving
both surfaces: on the board it is the zone's cards, dimmed ones included; inside an opened
folder the whole view is that folder's contents anyway. It is offered by `folder-editor`,
so it lives once beside rename, recolour and delete — ⚠️ behind `selectableCount`, which
the zone menu and the breadcrumb pass and the **switcher** does not: the switcher lists
folders you are not in, and selecting the notes of one of those is a gesture with no visible
result. The entry names its count rather than saying "all".

⚠️ `BoardStore.isShowing` is `isBoard() && no folder open`, and it is the one place that
predicate is written. The inside of a folder is a flat grid, not a board, so an open folder
takes the canvas back whatever the switch says — and that condition had already been copied
into the page template, the page class and the canvas keyboard.

**Tidying up is the same two rules, run again — and it is two gestures, not one.**
`arrange_board` takes a `BoardScope`, because a single control confused two very different
things. What goes to pieces on a board is the cards **outside** the zones, and they cost
nothing to redo; a zone somebody positioned and sized by hand is the only manual work the
board holds. One button did both, so the click that repaired the cheap half destroyed the
expensive one — which is what stops anyone pressing it a second time.

- `looseCards` flows the unfiled cards under the zones **as they stand**
  (`arrange_loose_cards`, whose `zones` comes back empty). Often, and nothing anybody
  chose is lost — so it is the corner click, and the one with a key (`A`, in
  `CANVAS_KEYS`, which is also what puts it in the shortcuts sheet).
- `everything` adds `arrange_zones`: zones back in `created_at` order, three across,
  each at the height its contents need. Rarely, and it overwrites every frame set by hand —
  so it sits a notch further, behind the control's chevron, labelled with what it will
  touch (`Réorganiser 2 dossiers et 2 cartes`) rather than with a warning. That is the
  shape the tag manager and emptying the trash already use: name what you touch before
  touching it. A ghost preview on hover was refused — a second layout engine on the front
  for a tooltip.

⚠️ The command answers a `BoardArrangement`: `moved`, and the layout it **replaced**. The
new one arrives with the reload the front end does anyway, and this is the only moment the
old one still exists. `restore_board_layout` puts it back, and the record is the fourth
branch of `Reversible`. ⚠️ `moved` counts what came out somewhere other than where it
went in, never what was placed: a board already in order opens no undo window, because
`openUndoWindow` refuses a count of zero. A zone the board had never laid out is left out
of `previous` entirely — it had no place to go back to, and inventing one on the undo would
put it somewhere nobody chose.

⚠️ **The board pans home afterwards, and that is not a nicety.** The pan is a native scroll
on `.board` and nothing else resets it, so an arrangement that lands everything back at
the top left while the user is panned to the right produces its result **off screen**: empty
dotted ground under a banner announcing success, which is indistinguishable from an erasure.
`BoardStore` bumps `arrangements`, the component watches it and scrolls to the origin —
remembering where it was, because undoing puts the board back at the coordinates it was
dragged to while the pan is now at that origin, which is the same defect from the other end
(`restorations` is the counter for that half). Counters and not booleans: two arrangements
running have to pan twice.

⚠️ `BoardStore.arrange` also drops both staged maps rather than letting them expire. Every
place a gesture had staged has just been overwritten, and keeping the overlay would draw the
cards back where the drag left them until a view happened to agree. The undo window is
opened by `NotesStore`, which injects `BoardStore` — the dependency runs one way, so the
board cannot reach the undo itself.

### Descending into a folder

A space shows its folders; a folder shows its notes. **Choosing a folder is opening it** —
there is one piece of state, `FoldersStore.activeFolderId`, whether it was chosen from the
switcher or by clicking a zone title on the board.

While one is open, the space switcher, the view switch and the folder switcher all give way
to a `SQL / ● Perf` breadcrumb: from inside a folder there is one place to go, and it is
back. Leaving restores whichever view you came from, because the board's own mode is never
touched.

⚠️ **The inside of a folder is not spatial.** No zones, no coordinates, nothing to draw —
it is already sorted by the fact of being there, and a second board inside the first would
be a second set of positions to maintain for nothing.

**It is a flat grid, and that is decided in Rust.** `notes::view::build` counts an opened
folder as filtering, so the view comes back as a single `results` section. ⚠️
`build_sections` still knows nothing about a folder — this is the only place the two meet,
and the date view's own sections are untouched, because nothing sends a `folder_id` unless
a folder has actually been opened.

**A note created here arrives already filed.** That and a drop on the board are the only two
places that file a new note. ⚠️ The quick-paste palette deliberately does not: it is used
mid-task from another application, and a decision there would sit in the fastest path in the
product.

**Escape falls through**, in this order: the selection, then the search and the facets, then
out of the folder. Leaving is the biggest of the three, so it goes last. ⚠️ The middle rung
asks `hasUserFilters` and not `isFiltering` — the latter is the _view's_ answer and is true
inside an opened folder, so Escape would clear a search that is not there and never fall
through.

**The folder's three actions live in one component.** `folder-editor/` holds the rename, the
palette and the delete; the switcher, the breadcrumb and the zone menu on the board all
project it, so they cannot drift apart. Deleting from the breadcrumb goes back, with the
notes now loose.

### A folder travels; its coordinates do not

The folder is part of what a note is. Where its zone sat on a board is not.

**Out.** `Bundle` gains the folders actually cited, beside the spaces it already carries,
and each note carries its `folderId`. Only the cited ones travel, for the same reason only
the cited spaces do: exporting one space should not recreate a whole tree on the other side.
⚠️ `#[serde(default)]` on both new fields, exactly as `kind` and `items` carry it — an
export written before folders must stay readable, and there is **no `FORMAT_VERSION` bump**
because such a file still parses.

⚠️ **No coordinates, and nothing had to be stripped to achieve that.** `Folder` carries no
geometry: where a zone sits lives in columns only the board's own query reads. That is the
whole reason the geometry was kept off the model in the first place, and a test reads the
written file to prove no `x` ever reaches it.

**In.** A folder is matched **by name inside the destination space**, case-insensitively,
and created when absent — the rule spaces already follow. Spaces are merged first, because a
folder needs its space to exist.

⚠️ A note's `folderId` is the _sending_ library's, so it is remapped, and **dropped when the
file did not carry the folder**: a dangling id would be refused by the foreign key, losing
the whole import over a note that is merely unfiled. A folder whose space did not make it is
dropped for the same reason its notes were.

`ImportReport` gains `foldersCreated` next to `spacesCreated`, and a library that arrived
arranged says so — every library operation reports, including when it changed nothing.

⚠️ **An attachment's id is remapped too, and for a harder reason than a folder's.** It is
half of `attachments::model::stored_name`, which is joined onto the attachments directory to
decide a write path — and also handed to the sweeps that delete. An id read out of a file is
whatever the file said: a record claiming `../vault` made the import overwrite the wrapped
master key with bytes the file chose, and report success, locking the library for good. Two
things hold it now, and the second is what matters:

- `restore_attachments` gives every incoming record a fresh id before it writes. ⚠️ The
  bytes are taken from the archive **first**, since the archive is keyed by the _sending_
  library's `stored_name`.
- `stored_name` itself cannot return a traversing path any more. The id half contributes
  only `[A-Za-z0-9-_]`, the extension half was already filtered, so the result is always one
  path component — whoever calls it, with whatever. The guard belongs in the model and not
  at the import, because once a poisoned id is in the database every other
  `directory.join(attachment.stored_name())` inherits it.

### What a first launch teaches

A virgin database has no space, so `SampleNotesService` seeds one — and since the folders
milestone, **it arrives already arranged**: two folders, three of the four notes filed, and
one deliberately left loose. Without that the board opens empty on a fresh install, and the
one screen that explains what a folder is for shows no folder.

⚠️ **The folders are written in the same transaction as the space and the notes.** The two
guards in `seedIfFirstRun` only both say "virgin" for a database that has never been
written to; a seeding that created the space and the notes but not the folders would leave
a space standing, which both guards then read as "already seeded". It would never run again.

⚠️ **Each seeded row is stamped a millisecond apart**, and in opposite directions:
`folders::list` orders `created_at` **ascending**, the canvas orders `updated_at`
**descending**. Sharing one instant left both orders falling back to `id` — a random UUID —
so the zones and the sample cards came out arranged differently on each install.

⚠️ **`seed_samples` answers the space it made**, and the page selects it. With exactly one
space, "all spaces" is a distinction without a difference — and it is the one state in which
the board cannot be shown at all, so a first launch would have hidden the feature behind a
disabled button.

A note names its folder by **index** into the folders the same command creates: they have no
id until the transaction that writes them is under way.

The written guide ("À propos → Prise en main") has a chapter per subject, listed in
`getting-started-dialog.component.ts`. ⚠️ A chapter that names a key receives it as an
interpolation — Transloco replaces an unknown `{{name}}` with the empty string, so a body
cannot spell one out. `CHECK_KEY` is exported from the canvas keyboard directive for that:
the table that binds it and the chapter that names it read the same constant.

### Editing a note

The editor overlay is where every note mutation starts (title, body, language, tags, pin,
deletion). It stays presentational — it emits, the store persists — but it holds **local
drafts** for the title and the body, because persisting on every keystroke means one IPC
round-trip per character.

**It emits one output for all of them**, `patchRequested = output<NotePatch>()`, and the page
answers it with `store.applyPatch(note.id, $event)`. There were nine outputs, each wired to a
store method of its own, so adding a field to the editor meant editing four files. Whether a
value actually moved is not the editor's call either: the store holds what is stored, and it
is the one that drops an unchanged field. Drafts are confirmed on blur, and, crucially, on every closing path:
Escape, the backdrop and the close button all skip `blur`, so closing goes through a single
`requestClose()` that commits first.

Those drafts are `linkedSignal`s keyed on the note **id**, not on the note object: every save
refreshes `updatedAt` and produces a new object, which would otherwise wipe the in-flight
edit.

The body is **two stacked layers**, not a preview/edit toggle: `CodeViewerComponent` colours
the draft underneath, and a textarea sits on top with `color: transparent` and a visible
`caret-color`. The note stays highlighted while it is being typed, and there is no mode to
enter. Consequences worth knowing:

- The textarea's metrics must match the viewer's **exactly** or the caret drifts off the
  coloured text: same font, size and `line-height`, same `white-space: pre-wrap` /
  `word-break: break-word`, and a `padding-left` that adds the viewer's line-number gutter to
  its 24px padding — hence `padding: 20px 24px 20px 56px`. The gutter is **32px, not 48px**:
  `.line-no` is `width: 32px; padding-right: 16px`, and the global `* { box-sizing:
border-box }` folds that padding into the width. Getting this wrong shifts typing by two
  characters while looking perfectly aligned, because the caret is drawn by the textarea at
  its own — wrong — position.
- The **scroller is the wrapping `.overlay-body`**, never the textarea. The viewer, in normal
  flow, gives `.editor-stack` its height; the textarea is `position: absolute; inset: 0` over
  it and so never overflows internally. Both layers therefore scroll together with no
  `scrollTop` synchronisation to maintain. `.editor-stack` is `min-height: 100%` so a click in
  the empty space below a short note still reaches the field.
- The viewer is `aria-hidden` and `pointer-events: none`: the textarea carries the accessible
  text and every interaction, otherwise a screen reader reads the body twice.
- Escape leaves the body before closing the overlay (`onDismiss` blurs the textarea when it
  holds focus), so a keystroke aimed at the field does not dismiss the whole modal. Escape
  itself comes from the dialog shell, which only gives it to the modal in front — the editor
  no longer has to know whether the lightbox is open.

Deletion is a two-step confirm in the toolbar rather than a native `confirm()`, which would
freeze the whole WebView. The fullscreen toggle expands the panel to fill the backdrop and
persists through `PreferencesService`.

Where the note lives — its space, then its folder — is a **band of its own** under the meta
row, labelled like the attachments below it. It used to sit inside the meta row, between the
tags and the deadline, with the two choices stacked on two lines: neither metadata nor
header (#324). The two stay together, because moving the space clears the folder
(`NotePatch::apply`) and a folder control on its own would lie about that.

The attachment strip sits between that band and the body. It is fed by inputs and emits
outputs like everything else here: `AttachmentsStore` owns the state and follows the open note
itself, through an effect on `NotesStore.persistedNoteId()`. Attachments deliberately do
**not** travel inside `Note` — they have their own write cycle, and routing them through the
note would mean reloading the whole note on every add.

### The trash, and undoing a deletion

Deleting is **not** destroying. `delete_note` stamps `deleted_at` and the note leaves the
canvas; `notes::trash::RETENTION` (30 days) then decides when it really goes. Three
consequences:

- **Every read filters on `deleted_at IS NULL`** — `fetch`, `find`, both facet queries and
  the tag counts. A trashed note that resurfaced in a query would be editable without ever
  saying it is on borrowed time.
- **Purging is restricted to notes already in the trash** (`WHERE deleted_at IS NOT NULL`),
  so nothing can short-circuit the 30-day reprieve.
- **The retention is applied even if nobody opens the panel**: `lib.rs` sweeps at startup, and
  `list_trash` purges what expired before answering, so the panel never shows a note that a
  restart would erase.

`purgeAt` is **derived**, never stored: the retention can change between versions and a
deadline frozen in the database would not follow.

On the front, a reversible action records what it changed and shows `UndoBarComponent` for
`UNDO_WINDOW_MS` (8 s).

`Reversible` is a three-branch union — a deletion, a move, a tagging — and `NotesStore.reverse`
switches over it exhaustively, so a fourth kind of undoable action stops the front compiling
until it says how to put itself back. ⚠️ Each branch carries **what the back end answered**,
never what the front end guessed: `move_notes` returns the placements it actually changed and
`tag_notes` the `(note, tag)` pairs it actually added. Recomputing either from the selection
would mean holding a second copy of rules that live in Rust — and it goes wrong in a way that
destroys data, since undoing a tag a note already carried takes away something the batch never
gave. One key per kind on the bar, too: "3 notes deleted" and "3 notes moved" are not the same
sentence.

A batch that changed nothing opens no window at all. A bar offering to undo zero notes is
noise, not a safety net.

⚠️ The banner and the record are **two different things**: `undoBanner()` is what the timer
clears, `lastAction()` is what `Ctrl+Z` reads, and it survives. Hiding a suggestion is not
withdrawing it — the action stays undoable until another one replaces it or the user
dismisses the banner by hand, which _is_ an explicit refusal.

`Ctrl+Z` is handled by the page's keydown, ahead of the modifier guard that stops every other
canvas shortcut: it is the one gesture people make without looking at the screen.

⚠️ **Escape takes it back too, and only while the bar is up.** That difference is the whole
reason the two signals exist apart: the rung reads `undoBanner()`, so the key answers while
the offer is on screen and goes back to its other duties — the selection, the filters, the
folder — the moment it is not. It is the one rung of that chain that **writes**, and the
guard against a mis-press is that a banner is saying so at the time. The report it comes
from is worth keeping: the armed card names Escape, the second `Delete` sends the note to
the trash, and one keystroke later the key the card had just taught meant nothing at all
(#293). So the bar names it as well — each state says the key that undoes it, or teaching
the first one was worse than teaching neither.

### Keyboard navigation of the canvas

`CanvasKeyboardDirective` (`notes/`) drives the canvas from the keyboard whenever
the focus is neither in a field nor behind a modal (`DialogStack.hasOpenDialog()`, which also
disables the `Ctrl+K` search shortcut). Arrows move, `Enter` opens, `C` copies, `P` pins, `X`
checks, `Delete` trashes, `Escape` clears the selection. The keys are deliberately bare
letters: they only ever fire where no typing is happening.

⚠️ **`C` copies what the card's own control copies**, which is not `note.content`: a todo list
has none, and a snippet with `{{fields}}` asks for them first.
`PlaceholderFillStore.copyNote` holds that rule for both, and announces which note it took —
the card paints a tick on itself, the keyboard has no such surface, and the ring may be on a
card that is scrolled away.

⚠️ **`Delete` asks twice**, the way the card's menu asks for two clicks. The first press arms
the focused note, which the card then draws in red with words saying what one more press
would do; Escape, moving the focus or four seconds call it off. The armed note lives in
`NoteSelectionStore` and not on the card: the ring can move and a reload can rebuild the card,
and neither is a reason to forget what the first press said.

It is applied as a **host directive** of `NotesPageComponent`, so its element is the canvas
itself — which is how it measures the card grid without the page handing it a list of
sections.

**One table binds the keys and documents them.** `CANVAS_KEYS` gives each entry the caps the
shortcuts sheet draws (`keys`, `labelKey`) _and_, where there is one, the behaviour (`on`,
`ctrl`, `run`); `CANVAS_SHORTCUT_GROUP` is derived from it. An entry with no `run` is a key
documented here and handled elsewhere — `Ctrl+K` belongs to the search field, and a modifier
held during a click is not a key press at all. Before this, the sheet and the handler were two
lists and nothing kept them in step.

A `run` answers whether it acted, and only then is the browser's own behaviour cancelled: a
`Delete` with nothing focused has to stay a `Delete`.

**The number of columns is measured, not assumed.** It depends on the window width and each
section has its own card count, so `nextFocusIndex` (`ui/grid-navigation.util.ts`) takes the
cards' measured `top`/`left`, groups rows by `top` within a few pixels of tolerance, and picks
the nearest column in the adjacent row. Keeping it a pure function over rectangles is what
makes it testable without a DOM. Moving past an edge stops rather than wraps: wrapping in a
grid with no visible start or end makes it impossible to tell where you are.

The card is a real `<button>`, so the DOM focus follows the state through an effect —
otherwise the arrows would move an outline while the keyboard stayed behind.

### Multiple selection and bulk actions

`Ctrl`-click checks a card, `Shift`-click extends the range from the focused one, `X` toggles
it from the keyboard — the conventions of a file list, which is what the canvas becomes once
several notes are selected. `SelectionBarComponent` appears only when something is checked and
offers: move to a space, add a tag, share, move to trash.

Each action is **one command for the whole batch** (`move_notes`, `tag_notes`,
`delete_notes`), not a loop of single writes: a stale id in the selection produces a partial
result rather than failing the lot. Tagging **adds** without replacing — a bulk action
enriches the labelling, it does not overwrite it.

⚠️ `move_notes` and `tag_notes` answer **what they changed**, not how many notes they looked
at — the placements the notes left, the `(note, tag)` pairs actually inserted. That is what
`move_notes_back` and `untag_notes` undo, exactly and nothing wider. It also fixed a count
that had always overstated itself: `tag_many` used to report every live note in the selection,
including the ones already carrying the tag, which `INSERT OR IGNORE` had quietly skipped.
Those notes no longer have `updated_at` refreshed either, for the same reason — nothing about
them changed.

⚠️ Which notes "already carry the tag" is decided in SQL, before the insert, by the same
comparison the insert will make: `note_tags.tag` is `COLLATE NOCASE`, and SQLite's `NOCASE`
folds **ASCII only** — which is exactly what `eq_ignore_ascii_case` does in `tag_many`.

### `{{fields}}` in a snippet

A snippet like `psql -h {{host}} -p {{port=5432}}` is worth copying **filled in**. What counts
as a field is decided in `notes::placeholder` and nowhere else: the name is restricted to
`[A-Za-z0-9_-]`, so a note holding Angular template code (`{{ user.name }}`) does not turn
into a form on every copy. Values arrive parsed on `DisplayNote.placeholders`, and
`fill_placeholders` — a pure command, no database — does the substitution.

**The values are the note's, and they are kept.** `note_placeholders` stores them per note
(`(note_id, name)`, case-sensitive — `notes::placeholder::fill` tells `{{Host}}` from
`{{host}}`, where `note_tags` folds case), `set_placeholder_values` writes them, and
`placeholder::parse` merges them into the fields it finds in the text. Two consequences worth
stating: the **text** decides which fields exist, so a value whose token was renamed stays
stored but out of sight until the token comes back; and writing a value **does not touch
`updated_at`** — filling a field is not editing the note, and the canvas sorts on that column.
A field left empty is not stored (`normalize_values`): empty means "keep what the text
suggests", and storing it would freeze that answer the day the default changes.

**A value can also belong to no note at all.** The preferences panel's "Variables" page edits
`global_placeholders` (migration 7, `name` as the primary key, case-sensitive for the same
reason as `note_placeholders`): a `{{host}}` that means the same thing in every snippet is
worth saying once. Resolution order is **note value → global variable → default written in the
text**, which `notes::placeholder::resolve` builds by overlaying the non-empty typed values on
top of the globals. The text default comes last on purpose: `{{host=localhost}}` was a
suggestion noted the day the snippet was written, the variable was set for this machine.

Two things follow, and both matter:

- A global variable reaches a card as its field's **`default_value`**, never as `value`
  (`model::apply_global_defaults`). So the editor shows it in grey, as a suggestion — copying
  it into `value` would let `set_placeholder_values` freeze it in the note the day the
  variable changes. It is applied in a pass of its own, like the attachment counter, because
  `decorate` reads no database.
- `fill_placeholders` therefore **does** touch the database now, and returns a `Result`. It
  still takes no note id: the palette fills an unsaved draft as readily as a stored note.

Writing the set replaces it whole, like tags and checklist items: what is no longer sent is
what the user removed. No note is touched, `updated_at` included.

Three places offer the same single set of values. The editor carries `PlaceholderPanelComponent`,
a fold-away drawer between the metadata row and the body, mounted only for a note that has
fields — and its header is **three affordances rather than a chevron**: the whole bar is the
button (it lights up on hover), a single caret rotates instead of two glyphs swapping, and the
collapsed bar names its gesture ("Afficher") next to a summary of what it hides
(`host = db.internal · port = 5432 · +1`, capped at two). That is not decoration: with a small
caret and a monospace small-caps label — the vocabulary this app uses for inert section
headings — the bar read as a title, and nobody thought to click it; the card's ⚡ and the palette open `PlaceholderFormComponent`, seeded with the same
stored values. Both render the same rows (`PlaceholderFieldsComponent`) so the two paths cannot
drift on what an empty field means: it is a **suggestion** shown as the input's placeholder,
never a typed value.

The panel holds a local draft like the title and the body — reset on the note **id**, confirmed
on `focusout` (which bubbles, unlike `blur`) and by the editor before it closes. Its preview
toggle swaps the body for the filled text, read-only: the overlay asks (`fillPreviewRequested`),
the page fills through the back-end and hands the text back down (`filledContent`), the way
attachment previews already work. For a note with fields the toolbar's copy composes the text
**at the click** — `filledCopyRequested`, since a keystroke in the body or in a value would
make anything precomputed stale — and reports through `StatusNotifier` rather than the copy
button's tick, which would claim success before the bridge answered. "Copy as is" stays one
click away in the panel, for the note that only looks templated — the heuristic is careful,
not infallible.

### The quick-paste palette

`Ctrl+Alt+P` reveals the window and emits `devnotes:palette`; `PaletteStore` opens,
searches, and on `Enter` copies the highlighted snippet and **hides the window** so the user
lands back where they were pasting.

It searches **every space and ignores the canvas filters**: when you recall a snippet you do
not remember which space you filed it in. It also keeps its own state rather than reusing
`NotesStore`, whose search would otherwise change what the canvas shows behind it.

A snippet with fields goes through the fill form first — copying `psql -h {{host}}` verbatim
gives an unusable command. `Tab` opens the note instead of copying it, which is what makes the
palette double as a "find that note" shortcut.

⚠️ **A click on a row opens the note; it does not copy it.** Copying hides the window, and a
window that disappears on a click with nothing on screen saying why reads as the application
crashing. Nobody clicked their way here from another application — the window is already in
front — so the mouse gets the gesture a click on a note means everywhere else in the product,
and the paste path gets a control of its own beside the row (⧉, which copies and hides, like
`Enter`). The keyboard is untouched: it is the fast path and the muscle memory is the feature.

**It captures as much as it retrieves.** As soon as the query is non-empty, a "créer une note"
row is appended **after** the results — retrieving a snippet is the more frequent gesture and
keeps the first place, but a query that matches nothing highlights the create row by default,
which is exactly the quick-capture case. Choosing it turns what was typed into the note's
content, saved and opened straight away (no draft: there is nothing to wait for).

`PaletteStore` does not create the note itself — it does not know `NotesStore`, and the
reverse would be a cycle. `takeNewNoteContent()` hands the text over and closes; the page
chains `NotesStore.createWithContent`, the same path the clipboard capture uses.

The palette is an overlay in the main window rather than a second Tauri window: the window is
already warm behind the global shortcut, and a second one would mean a second Angular
bootstrap, its own CSP and its own lifecycle for the same result.

⚠️ **A copy from the palette is acknowledged on the desktop, not in the window**, because
the window is what the gesture takes away: `copyAndDismiss` copies, closes, hides, and only
then asks `DesktopNotifier` to say which note it took. Every other copy path in the
application answers through `StatusNotifier`, which draws under the titlebar — correct and
invisible here. Without a word from the desktop the window simply vanished, and it read as
the application crashing on a copy that had in fact worked (#285).

`DesktopNotifier` (`core/services/notifications/`) wraps `tauri-plugin-notification` behind a
token and an adapter, like `ClipboardService` and for the same reasons. ⚠️ It is **never
load-bearing**: whatever it is asked to say has already happened, so a refused permission, a
plugin that is not there and a desktop that drops the toast all come to the same thing —
`false`, and nothing thrown. It is also the one place that **translates imperatively**:
everywhere else code hands a `TranslationRef` to the `transloco` pipe, and there is no
template on the other side of this one. Permission is asked for **once** and remembered; a
prompt on every copy would be worse than no acknowledgement at all.

The field keeps the focus from start to finish and the list is walked with
`aria-activedescendant`; moving the real focus onto an option would lose the query being
typed. ⚠️ **Tab is an arrow here**, and every row button carries `tabindex="-1"` so that
the field is the only stop in the panel. Released to the DOM it walked a list of stops
nothing drew a ring on while the highlight stayed where the arrows had left it — two notions
of "the current row", one of them invisible (#283). The copy control is then out of reach of
the keyboard, and that is the accepted cost: `Ctrl+C` copies the current row.

⚠️ Taking the rows out of the order was not enough on its own. `FocusTrapDirective`'s
selector excluded `tabindex="-1"` on the bare `[tabindex]` clause only, so a
`<button tabindex="-1">` was still focusable as far as the trap was concerned and Shift+Tab
in the field wrapped onto the last row's copy control. The exclusion is on every candidate
now, and the trap leaves alone a Tab whose default a descendant has already prevented — the
same `defaultPrevented` guard `CanvasKeyboardDirective` carries, for the same reason.

### Global tag management

Free-text tags drift (`auth`, `authentication`, `Auth`), and nothing else in the app lets you
recollapse them. `list_tags` returns each tag of the corpus with the number of live notes
carrying it, and one operation covers both cases: renaming onto an existing tag **is** a
merge, because the database cannot carry the same tag twice on one note. Only the button label
changes with the number of selected tags.

Two subtleties in `notes::store::retag`:

- The target is **swept along with the sources and rewritten**, which is what makes a pure
  case correction (`auth` → `Auth`) effective. `INSERT OR IGNORE` alone would change nothing:
  the primary key is `NOCASE`, so both spellings are the same row.
- `updated_at` is left alone. A global retag would otherwise float the whole corpus to the top
  of a canvas that sorts on it, for notes nobody reopened.

**Nothing corpus-wide runs before it has said what it would touch.** These are the most
dangerous operations in the application and they act on the whole library rather than on a
selection, so a mis-click reaches them easily. `TagsStore` turns every one of them into a
`PendingTagChange` first — the kind, the tags, the target, and the number of notes — and only
`confirm()` writes.

⚠️ That number is counted by the back end (`count_notes_tagged`, a `COUNT(DISTINCT note_id)`
over live notes), not summed from the per-tag counts already on screen: a note carrying two of
the selected tags is one note, and a confirmation that overstates its blast radius teaches
people to dismiss it. If the count cannot be read, nothing is proposed and nothing runs —
a blast radius that cannot be shown is not a reason to go ahead blind.

⚠️ The confirmation replaces the actions rather than sitting beside them: the click that asked
for it is the one that would confirm it, and a second click landing on the same spot is the
accident a confirmation exists to stop. A **merge** says out loud that it cannot be undone,
because it is the only one that genuinely cannot — once `auth` and `Auth` are one row, nothing
knows which note carried which.

### Attachments

An attachment is a file **next to** a note: the database keeps a record, the bytes live in
`app_data_dir()/attachments/` under a name derived from the record's id — two screenshots both
called `capture.png` must not overwrite each other, and a name coming from outside has no
business deciding a write path.

The bytes cross the bridge only on demand, as a `data:` URI: the WebView's CSP forbids loading
a local file, and opening the `asset:` protocol would be a wide door for displaying a
screenshot. That encoding costs a third more than the file, which is why `read_attachment`
fetches **one** at a time and never the list.

**Three ways in, two ways back out.** A file arrives through the picker, through a drag-drop
onto the window (`FileDropService` — the drop is a _window_ event carrying real paths, a DOM
`drop` handler would receive nothing), or as an image pasted into the editor. It leaves
through `open_attachment` (the system's default application) or `save_attachment` (copied
where the user asks). A file you can only read the name of is not attached, it is stored.

All three go through `AttachmentsStore`, which owns the order of operations rather than
leaving it to the page: **attaching demands a note in the database**, so the draft is saved on
the way (`targetNote()`) — a note you attach a file to is no longer empty. ⚠️ The store
re-points itself there rather than waiting for its effect on `persistedNoteId`, which only
runs on the next detection cycle, after the write; the page used to chain those three steps by
hand, and forgetting the middle one attached nothing and said nothing.

The pasted image is the one worth explaining: the editor's `paste` handler reads only the
**type** of what was pasted and, for an image, calls `attach_clipboard_image`. The bytes never
cross the bridge — the native side reads the system clipboard, which hands it raw RGBA, and
encodes a PNG (`model::encode_png`). Sending them up to send them back down would cost two
conversions and several megabytes of JSON, and the body is a `<textarea>` that could not
display the image anyway.

The strip's inline preview is capped at 220 px so it cannot push the editor off screen, which
makes a screenshot of code unreadable — clicking it opens `ImageLightboxComponent`, bounded
only by the window. It **reuses the `data:` URI the preview already loaded**: a multi-megabyte
payload has no business crossing the bridge twice. The lightbox therefore only exists while a
preview does, and closing one closes the other (`togglePreview` clears the zoom). Escape
reaches it and not the editor underneath because it declares the `zoom` rung and `DialogStack`
gives the keystroke to whichever modal is in front.

Ordering matters on write: the file is copied **before** the record is inserted, and the
record is rolled back with the file if the insert fails. A record without a file shows a
broken thumbnail; a file without a record is swept at the next startup
(`attachments::sweep_orphan_files`). Purging a note collects its file names **before** the
`DELETE`, since the cascade takes the records with it.

### The "Fichier" menu, and where the rest lives

The titlebar carries a **File menu** next to "À propos" — the convention of a desktop
application. It holds import, export, "copy the selection as Markdown", the preferences and
quitting.

**The menu owns its entries**: `FileMenuComponent` declares them as an array and injects what
they need — `LibraryStore`, `NoteSelectionStore`, `SpacesStore`, `ClockService`. `disabled` is
a `Signal` because "Exporter la sélection" follows what is checked at that instant, and the
order on screen is the order in the array.

There used to be a contribution registry here — three of them, in fact, one per extension
point — so the chrome could stay ignorant of a feature it might not have. That indirection had
exactly one purpose, a second tool, and [#23](https://github.com/vmillet-dev/devnotes-rs/issues/23)
decided there would not be one. With a single feature it protected nothing and cost a real
detour: reading what a menu entry did meant opening the notes page. They are gone.

Two things deliberately did **not** go in that menu, because they are views on the notes and
not operations on a file:

- **The trash** sits next to the quick filters in the notes topbar — it is one more way of
  looking at the notes.
- **Tag management** sits at the end of the tag rail, which is exactly what it acts on. The
  rail disappears when no tag exists, and so does the button: there is nothing to manage.

### Preferences

"Préférences…" opens `SettingsDialogComponent` (`titlebar/file-menu/settings-dialog/`): a rail of pages on
the left, the chosen page on the right, and Annuler / Appliquer / OK at the bottom.

**Appliquer, Annuler, OK — and a draft under them.** Every control used to write straight
into `SettingsStore`, which was written down as a decision rather than an oversight. The
counter-argument turned out to be stronger: a shortcut is _captured_, not typed, so a
half-entered one was registered with the operating system for as long as it took to
finish; a theme flipped under the cursor; and there was no way back from a change other
than remembering what it was.

So the panel edits `SettingsDraftStore` (`core/services/settings/`), and Appliquer or OK
writes it through. ⚠️ It stages **both paths a preference can take**, because the panel
edits both: an `AppSettings` key, and a shortcut binding. A draft covering only the first
would make OK mean two different things on two pages.

⚠️ **The three services that push to the native side read `SettingsStore`, never the
draft.** That is the point of the layer: nothing reaches `set_global_shortcuts`,
`set_window_behavior` or the autostart plugin until the button is pressed.

**What "immediately" was buying is kept where it means something.** The theme and the
density show on screen while they are being chosen — nobody picks a theme without seeing
it — through `SettingsStore.preview`, which feeds `shownTheme` / `shownDensity` and
therefore the `data-theme` / `data-density` attributes. ⚠️ A preview is **not** a write:
nothing reaches `preferences.json`, and Annuler clears it with nothing to roll back.
Everything else waits for the button, because nothing else has a preview that means
anything.

⚠️ **Closing with unapplied changes has to say so**, or Escape and the backdrop become a
silent Annuler — the one outcome nobody would have chosen on purpose. Neither produces a
click, so the footer swaps its buttons for a sentence naming how many changes are waiting
and two answers: "Fermer sans appliquer" and "Appliquer et fermer". Every way out of the
panel passes through `finish()` after an apply or a cancel, so the draft is always empty
when it reopens.

⚠️ **The variables page is corpus data, and it is under the same footer anyway.**
`VariablesStore` writes through the IPC bridge rather than through `PreferencesService`,
so the draft does not hold it — `SettingsDialogComponent` does, reading
`VariablesStore.isDirty()` beside `SettingsDraftStore.isDirty()` and committing both.
A panel where one page wrote on blur and three waited for a button would be worse than
either rule on its own. The store's `load()` refuses to run over edits in hand, because
the page is recreated every time the rail changes section.

⚠️ **One rule, two readers.** `refuseBinding` and `conflictsAmong` (in
`shortcut-bindings.store.ts`) take a `BindingReader`, so the store answers from what is
stored and the draft from what is staged. A check living in only one of them would let
the panel offer a key the store is about to refuse.

**The pages are a list in the panel**: "Général", "Raccourcis", "Sécurité" and "Variables",
each in its own folder beside the dialog, rendered through `NgComponentOutlet` so the rail
stays one loop over one array. The order on screen is the order of the list.

The split is the shape of the subject rather than of the file it came from. "Général" keeps
what the application looks like and how it behaves; the keys moved out because there are
eleven of them and they needed room; and "Sécurité" holds the two things that decide who can
read the library — the passphrase that unwraps its key, and whether a copy is taken at launch.
⚠️ Those two belong on **one** page: the copies are wrapped under the phrase too, so a page
carrying only one of them would let someone change the phrase without meeting what the change
reaches.

⚠️ Each page's stylesheet is `@include settings-page;` and its own extras. A component's SCSS
is out of reach of its neighbours, so the groups, their titles and the four things a row is
made of live in a mixin — otherwise the three pages would carry the same thirty lines three
times, which is also what the duplication gate would have said.

`SettingsStore` holds one signal per setting — the interface language, the theme, the density,
the tray behaviour, the three global accelerators — backed by `PreferencesService`: one key per
setting, not one serialised object, so a setting added later cannot make a file written by the
previous version unreadable. `restore()` runs from the app initializer, after
`PreferencesService.hydrate()` and before the first render.

**Three services read those signals and push them to the native side**, rather than the store
reaching for the IPC itself — a preferences store has to stay readable outside Tauri:

| Service                  | Effect                                         | Native side                         |
| ------------------------ | ---------------------------------------------- | ----------------------------------- |
| `GlobalShortcutsService` | `set_global_shortcuts` on every change         | re-registers, returns what is taken |
| `WindowBehaviorService`  | `set_window_behavior`                          | what close and minimise do          |
| `AutostartService`       | `tauri-plugin-autostart` (`autostart:default`) | the system's own startup entry      |

Each starts from the app initializer, and each builds its `effect` with an **explicit
injector**: they are started outside a constructor, where `effect()` would have nothing to
attach to. For the same reason the initializer injects **everything before its first `await`** —
an `inject()` after one is outside the injection context, and the whole bootstrap fails with
NG0203 and a black window.

⚠️ The native side has shortcut defaults of its own, and must: it takes the shortcuts before
the front has started, and without them `Ctrl+Alt+P` would be dead for the length of the first
render — exactly the second one uses it from another application. They are declared once,
though: `ShortcutBindings::defaults()` crosses as the `DEFAULT_SHORTCUTS` constant, next to
`GLOBAL_ACTION_EVENT` and `APP_METADATA`, and the front re-exports it rather than retyping it.

The same treatment closes the other mirror: `notes::placeholder` exports `FIELD_NAME_PATTERN`,
and `isVariableName` builds its `RegExp` from it. ⚠️ Rust checks characters rather than
matching a regex, so the pattern and `is_field_name` are held together by one test —
`the_pattern_and_the_rule_agree` — and by nothing else.

`AutostartService` reads the system **first** and aligns the preference on what it finds:
turning the entry off from the task manager has to uncheck the box, not see DevNotes put it back.

**What the settings actually change**

- **Theme** — `system` / `dark` / `light`, resolved into a `data-theme` attribute on
  `<html>` (see _Theming_). `system` follows `prefers-color-scheme` live.
- **Density** — `data-density`, which swaps four spacing variables.
- **Start with the system, minimise to tray, close to tray** — the last one was the application's fixed
  behaviour and stays the default; both tray settings are still refused when there is no tray
  (`desktop::hides_on_close` / `hides_on_minimize`), since hiding a window nothing can call
  back is worse than closing it.
- **Quick paste** — "show pinned first", the only consumer of `NotesQuery.pinnedFirst` that
  ever sends `false`. The palette's accelerator used to sit here and now lives on the
  Shortcuts page, with the ten others.
- **Copy confirmation** — the acknowledgement lives in `ClipboardService` itself rather than in
  its five callers, four of which show nothing today: copying from the canvas with `Ctrl+C`
  said not a word. Callers with something better to say — "3 notes copied as Markdown" — speak
  after, and the banner keeps the last message.

### Shortcuts: two vocabularies, two storage paths

Eleven actions can be moved: the three **global** ones registered with the operating system,
and the eight **canvas** keys. Everything else on the read-only sheet is documented and fixed —
the arrows are the grid's own navigation, Escape is the way out of every other thing on
screen, Tab is how the window is crossed, and the Ctrl/Shift clicks are not keys at all.
`isCanvasAccelerator` refuses those by name, which is what keeps a library you can still get
out of.

**A canvas key is declared where it is bound.** `CANVAS_KEYS` in `CanvasKeyboardDirective`
grew an `id` and an `accelerator` on the entries that can move; the caps the sheet draws are
derived from that accelerator, so a key is spelled once. Two constants come out of the same
table — `CANVAS_SHORTCUT_GROUP` for the sheet, `CANVAS_ACTIONS` for the panel — and a key
cannot be documented, bound or made movable without the other two following.

**`ShortcutBindingsStore` (`core/services/shortcuts/`) reads both paths as one table.** The
global three live in `AppSettings`, because the native command takes them as a block; a canvas
key is one preference of its own, `devnotes.shortcut.<id>`. ⚠️ The store holds **no list**: a
`Rebindable` carries its own fallback, so nothing has to register and there is no third place
to add a key to. Reading both halves together is also what makes a collision _between_ them
something the panel can say out loud.

⚠️ **The two halves read the keyboard differently, and it is a deliberate fork.**

|        | reads                                   | may be bare | why                                                                                                                                     |
| ------ | --------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| global | `event.code` — the key's **position**   | no          | the native parser reads it back by position, and a shortcut with no modifier would swallow that key in every application on the machine |
| canvas | `event.key` — the **printed** character | yes         | `C` is read off the keycap by someone looking at this window, and the key answers only while the canvas has the keyboard                |

On AZERTY the position `KeyA` is the key labelled `Q`. Capturing both halves the same way puts
one of the two on the wrong key the moment the layout is not QWERTY — which is why
`acceleratorFromEvent` and `canvasKeystrokeFromEvent` are two functions and the panel picks
per row. `canvasKeystrokeFromEvent` upper-cases a single character, so a caps-locked keyboard
answers the same key as a bare one, and folds the Command key into `Ctrl`: `Super` means
something of its own only where a shortcut leaves the window.

**A refusal is a reason, not a boolean.** `rebind` answers `null`, `{ kind: 'illegal' }` or
`{ kind: 'taken', by }`, and the page turns that into the sentence under the field. Deciding it
twice — once in the store, once in the panel — is how the two drift. `conflicts(among)` exists
for what `rebind` cannot refuse: a preferences file edited by hand, and a shipped default
landing on a key someone had already taken.

⚠️ `aliases` on an entry fire it whatever it is bound to. Backspace has trashed a note since
before the key could be moved, and making the key movable is no reason to take that away.

The read-only sheet in the About menu keeps existing — it is the one reachable with the
keyboard while working — and every row that can be moved now resolves its caps through the
store, so it shows what is bound rather than what shipped.

### Help: what's new, getting started, shortcuts

**The written guide is a walk, not a wall.** It was ten chapters of prose in one scrolling
panel — roughly 3 400 characters, all of it true and none of it looked at, which is the
worst return a help surface can have. One chapter is on screen at a time now: a schematic of
the screen it is about, and two sentences beside it. The bodies came down to about 2 000
characters in total, because the figure carries the arrangement and the words only have to
say what it is _for_.

⚠️ **Inline SVG and not an image.** The CSP is `script-src 'self'` with nothing remote, and
a PNG would need one file per theme; the figures are drawn from the palette's own custom
properties, so they follow the theme for free. ⚠️ Schematic on purpose — boxes where the
cards are, a bar where the rail is: a screenshot would be a fourth thing to keep in step
with the interface, and it is the _arrangement_ they have to show. They are `aria-hidden`,
and every chapter says in words what its figure shows.

⚠️ **It opens at a chapter**, and that is what `core/services/help/` exists for. An empty
canvas offers the chapter about notes and an empty board the one about folders — reaching
help from the thing it explains, neither of which is anywhere near the titlebar. `HelpStore`
holds which chapter is open, the About menu reads it, and `GUIDE_CHAPTERS` is the one list
of ids so a link cannot point at nothing.

⚠️ "Fermer" is on **every** chapter, not only the last: a walk you can only leave by
finishing it is a wall with extra steps. ⚠️ The figure band is a fixed height, or a chapter
two lines shorter than the last would make the dialog jump under the cursor on every step —
the same trap the update prompt's notes block had.

The titlebar's **"À propos" menu** carries the update check, three help panels and the card
itself, separated into those three groups. They share one signal (`AboutMenuComponent.panel`)
rather than a boolean each: they sit on the same backdrop rung (55), only one is ever wanted at
a time, and four flags would allow a state where two of them are stacked and the focus trap of
the loser keeps the keyboard.

- **"Nouveautés"** renders `CHANGELOG.md`, the repository's own, **embedded in the binary** by
  `include_str!` (`src-tauri/src/changelog.rs`) and read through `app_changelog`. The Markdown
  is parsed **in Rust** (`changelog/model.rs`) into releases, categories and entries, so the
  front renders a typed structure with the components it already has: no Markdown renderer to
  pull in, no `innerHTML`, nothing for the CSP to forbid. The grammar is deliberately thin —
  `## ` a release, `### ` a category, `- ` an entry, an indented line continues the one above —
  and the file is written to match it; a shipped file that no longer parses fails a test rather
  than emptying the panel in silence.
  An entry is not a string but a list of **typed runs** (`ChangelogSpan`: plain, strong, code),
  which the panel draws with a `switch` and three elements — still no `innerHTML`. That grammar
  is two markers, `**` and a backtick, paired and on one line, and an unclosed one is text, so
  an entry about `5 * 3` stays readable. ⚠️ A continuation line is re-cut with the line it
  continues, or a marker opened on the first would never find its close. The trailing `(#123)`
  a squashed pull request leaves behind is dropped when the entry is **read**, never when the
  file is written: the number is what makes the release on github.com navigable, and noise on
  a panel with no links in it. The newest section is **generated** by the release
  workflow from the pull requests merged since the last tag (see "Releasing"), which is also
  where a category's icon comes from: `### ✨ Added` is the heading in the file, so nothing on
  the front end decides what a category looks like and a new one needs no code. The changelog
  is **not translated**, on purpose and like
  the release notes the updater hands over: one changelog, written once, rather than two that
  drift apart. The panel marks the release the running binary is, and its footer opens the
  repository's releases page through the `opener` plugin.
- **"Prise en main"** is nine chapters of prose, keyed by translation
  (`gettingStarted.chapters.<id>`). The list of chapters lives in the component. The bodies are
  handed the live key bindings as
  interpolation parameters — a guide quoting the combination that shipped would be wrong for
  anyone who changed it.
- **"Raccourcis clavier"** is a read-only sheet. Read-only because the one shortcut that can be
  changed is changed in the preferences, and a second editor for it would be a second place to
  keep in step.

The groups of the notes — the canvas arrows, `X` to check a card, `Alt+↑` to reorder a
checklist item — come from `core/constantes/notes-shortcuts.ts`, which the sheet imports.
They live with the notes rather than in the sheet because the canvas group is **derived from
the key table that binds them** (`CANVAS_SHORTCUT_GROUP`, from `CanvasKeyboardDirective`): a
key documented but not bound, or the reverse, is not possible.

The **global** group is the exception and is built by the dialog itself: those three
combinations are the application's, they work with the window closed, and the quick-paste one
follows a preference — the sheet reads `SettingsStore` so it shows the key that is really bound
rather than the one that shipped. A shortcut is spelled as one `<kbd>` per key
(`acceleratorKeys`), the `+` drawn between the caps and `aria-hidden`: inside a cap it reads as
a key to look for on the keyboard. What is not a key press — `Ctrl` + click to check a card — is
said in the label rather than drawn as a cap.

The three panels share their frame through mixins in `src/styles/_mixins.scss`
(`help-panel`, `help-header`, `help-body`, `help-actions`, `key-cap`) rather than through a
common component: a component's SCSS is out of reach of its neighbours, and three panels that do
not look alike would read as three unrelated windows.

### The first launch

A brand-new installation opens on **sample notes**, in a space of their own
(`SampleNotesService`, `core/state/`). This is not decoration: a virgin database has
no space, and creating a note with nowhere to file it is refused on purpose — so without them
the first screen is empty, silent, and offers a "+ Nouvelle note" button that does nothing.

Four notes, one feature each: a pinned welcome note (so the first screen is not an empty
"pinned" heading), a shell snippet carrying `{{fields}}`, a checklist, and a code snippet with a
deadline — which is what lights the ⏳ badge and gives the "à trier" filter something to find.
They are ordinary notes: editing or trashing them is the point, and the written guide behind
"Prise en main" is what survives the day they go.

**The content comes from the front end**, not from a seed in Rust, for the reason no
user-facing string ever comes out of the back end: it would ship French into an English
interface. It also means the samples arrive in the language the application starts in. The two
snippet bodies are the exception and are hard-coded in the service — they are _code_, so they
are not translated, and they could not be: Transloco reads `{{name}}` as an interpolation and
would replace a snippet's fields with empty strings on the way out.

⚠️ **Two guards decide a first launch, not one.** A preference marker
(`devnotes.notes.samplesSeeded`) alone would re-seed anyone whose preferences file went missing;
"no space at all" alone would re-seed the day the last space disappears. Together they only ever
match a database that has never been written to. The marker is written **after** the seeding,
because the seeding is one write: `seed_samples` creates the space and the four notes in a
single transaction, so there is no halfway. ⚠️ It used to be six round trips, and a process
killed between any two left a space standing with nothing in it — which both guards then read
as "already seeded", leaving that install with an empty canvas for good. An installation that
predates the samples is marked as skipped so the check stops running on every launch. A failure is silent: the canvas reports its own, and a second banner
about samples nobody asked for would only add noise.

### Import, export and copying out

- **Export writes an archive**, `.devnotes`, which is a zip: `bundle.json` at the root
  (`transfer::model::Bundle` — a version, an instant, the spaces cited, the notes and the
  attachment records) and one entry per attachment under `attachments/`, named by its
  `stored_name`. The bundle is deflated, being repetitive text; the attachments are stored as
  they are, a PNG being compressed already. The format reuses the domain types rather than
  duplicating them, so a field added to `Note` is exported without anyone thinking about it.
  Only the spaces actually cited travel: exporting one space should not recreate a whole tree
  on the other side.
- **⚠️ An export can be sealed, and with a key of its own.** The library key is derived
  from the passphrase typed at launch and never leaves the machine; an export is the one
  file meant to reach another one, so `transfer/protect.rs` derives a second key from a
  phrase the user gives that file, and writes the recipe (version, algorithm, cost, salt)
  in the clear beside the payload — a salt is not a secret, and a reader has to know how
  to derive before it can ask anything else. The sealed `bundle.sealed` **replaces**
  `bundle.json` rather than sitting beside it, or a reader that could not open it would
  quietly fall back on a bundle in the clear. Attachments are opened under the library key
  and resealed under the export’s, so the file is openable by whoever was given the
  phrase and by nobody else.
- **⚠️ An unprotected export is still written, and is still plaintext.** Refusing one would
  break the portability the exchange format exists for. What the interface owes the user
  instead is to ask which of the two it is about to write (`PassphrasePromptComponent`,
  with the warning beside the button) and to say which one it wrote — `ExportReport.protected`
  picks the message. On the way in, `export_is_protected` is what lets the prompt appear
  before the import starts rather than as a failure after it; a refused phrase asks again
  rather than failing, since it is the ordinary answer to a typo.
- **⚠️ Base64 inside the JSON was the obvious alternative and was refused.** It costs a third
  more bytes, and the import path holds the file as a `String`, then a `serde_json::Value`,
  then a `Bundle` — three copies of every screenshot in memory, which a library of a hundred
  captures turns into a gigabyte. `file::Payload` hands entries over one at a time instead.
- **⚠️ A new DevNotes reads an old file; an old DevNotes does not read a new one.** `file::read`
  sniffs the zip magic and falls back to parsing the whole file as JSON, so every `.json`
  export written before the archive still imports. The picker keeps `json` among its
  extensions on the way in for exactly that reason, and offers only `devnotes` on the way out.
  `FORMAT_VERSION` is untouched: the container changed, the data shape did not.
- **Import merges, it never replaces.** Spaces are matched by name, case-insensitively, and a
  note whose id is already taken is counted as skipped rather than overwritten — so the same
  file can be imported twice without duplicating anything. A bundle from a newer format
  version is refused outright rather than half-read.
- **An attachment comes back only with a note that actually arrived.** One belonging to a
  skipped note is already in the library. ⚠️ The file is written **before** the record, the
  rule `attachments.rs` already holds — a record without a file is a broken thumbnail, where
  a file without a record is swept at the next startup, which is also what collects these if
  the transaction rolls back. A record the archive names but does not carry is counted in
  `attachments_missing` rather than swallowed: the note arrives with a preview that will stay
  empty, and the report is the only thing that explains it.
- **Copying out stops at the clipboard.** `share_notes` renders the selection as Markdown
  (heading, space, context, tags, then a fenced block). The fence is longer than the longest
  run of backticks in the content, otherwise a note that already contains a Markdown block
  would cut its own in half. The menu entry names the format — "Copier la sélection en
  Markdown" — because a format nobody asked for is a surprise, not a feature.

⚠️ **Every one of these reports, including when it changed nothing.** Exporting then
re-importing at once is the first thing anyone tries, and it legitimately imports zero notes:
every id is already there. Without a message that outcome is indistinguishable from a
failure, so `LibraryStore` pushes a distinct `file.importedNothing` for it, and an export
names the file it wrote. The report goes to `StatusNotifier` (`core/services/notifications/`), rendered
under the titlebar by `StatusToastComponent` — not inside the menu, which closes on the click
and which a native file dialog covers anyway.

`transfer::collect` and `transfer::merge` take a `&mut SqliteConnection` rather than living
inside the commands, which is what makes the round trip testable (`tests/transfer.rs`) without
a Tauri runtime.

The file itself is written and read **in Rust**: the serialisation is the domain's, and
sending it across the bridge to be reassembled in TypeScript would mean a second format to
keep in step. The front-end only picks a path, through `FileDialogService`.

### Creating a note writes nothing

Opening the editor on "+ Nouvelle note" produces a **local draft** (`DRAFT_ID`), not a row: a
blank note per opening turns the canvas into a pile of things to tidy up. The draft is
persisted on the first change that makes it worth keeping — a title, a body, a tag, a pin, a
deadline (`isWorthSaving`) — and closing it untouched simply drops it.

⚠️ `draftMaterialisedAs` is not optional. Closing the editor commits the title **then** the
content with no change detection in between, so the second call still carries `DRAFT_ID` while
the note already exists; `resolve()` redirects it. Without that, the second commit would write
into nothing.

Attaching a file needs a real row, so `materialiseDraft()` saves the draft first — a note you
attach a file to is not empty either.

### Data access

Stores never talk to a data source directly. They inject `NotesRepository` /
`SpacesRepository` — ordinary `providedIn: 'root'` classes, not an interface behind an
`InjectionToken`. There is one implementation and there has only ever been one; the triad
described a variation that does not exist, and `app.config.ts` binds nothing for them.

Substituting them is still a one-liner: `provideAppTesting()` overrides each class with
`{ provide: NotesRepository, useValue: fake }`, exactly as it already does for
`UpdaterService` and `AppInfoService`. What the interface really bought was the _compile-time
check on the doubles_, and the fakes keep it with
`implements Pick<NotesRepository, keyof NotesRepository>`: `keyof` on a class type yields only
its public members, which both drops the private `ipc` (nominal, hence unimplementable) and
keeps the method list in sync by construction. Rename a method on the real class and
`src/testing/` fails to compile.

The notes contract is `query` / `create` / `update` / `delete`, plus the batch and trash
operations (`deleteMany`, `restore`, `loadTrash`, `purge`, `emptyTrash`, `moveMany`,
`tagMany`), the corpus-wide tag operations (`loadTags`, `renameTag`, `mergeTags`,
`deleteTag`) and `fillPlaceholders`. Spaces expose `loadAll` / `create` / `rename` / `delete`.
`AttachmentsRepository` covers `loadFor` / `attach` / `read` / `delete`, and
`TransferRepository` covers `export` / `import` / `share`.

There is deliberately **no method returning the raw list of notes** — offering one would invite
a caller to filter it again. That holds even for export: `export_notes` returns a _count_, and
the note list it assembled never leaves Rust.

`SpacesRepository.delete` takes a refuge (`delete(id, targetSpaceId)`) rather than an id
alone: a one-argument signature would have made data loss the default, since the schema
cascades. Moving a single note needs no dedicated method — `spaceId` is part of `NotePatch`,
and `NotesStore.moveNote` is a thin wrapper over the ordinary update path.

`SpacesStore.deleteSpace` returns a boolean and does **not** reload the notes: it does not
know `NotesStore` (the reverse dependency already exists, and closing the loop would be an
injection cycle). `NotesPageComponent` chains the reload, which matters when the current
query does not mention the deleted space and would otherwise show nothing new.

Keep this seam intact: no component or store calls a command, the repositories do.

## IPC boundary (Angular ↔ Rust)

**The boundary is generated, not written.** `src/app/core/ipc/bindings.ts` is produced by
[tauri-specta](https://docs.rs/tauri-specta) from the Rust signatures: one typed function per
command, plus a TypeScript type for every struct and enum that crosses the bridge. It is
committed — the front-end does not compile without it — and regenerated by

- `npm run tauri dev`, which rewrites it at every launch (`export_bindings()` in `lib.rs`,
  behind `debug_assertions`), or
- `npm run bindings`, which runs the `export-bindings` binary alone when a Rust signature
  changed and starting the whole app is not worth it.

It deliberately is **not** a `#[test]`: on Windows the test executable lives in
`target/debug/deps/`, where the `WebView2Loader.dll` that `tauri-build` drops is absent, and
merely linking `Builder::export` there stops the binary from starting at all.

This replaces the hand-written `IpcContract` table and the `tauri::generate_handler![…]`
list, which were two mirrors of the same thing kept in step by review. `collect_commands![…]`
in `lib.rs` is now the single list: it both registers the commands with Tauri and decides
what `bindings.ts` contains. Tauri matches arguments **by name** and renames them to
camelCase; nobody spells `targetSpaceId` by hand any more.

Only `core/data/` and `core/ipc/` call a generated **command**: everything above the
boundary speaks the **model**, which `note.mapper.ts` converts to and from. The generated file
stays behind the same boundary the hand-written types were behind. `core/services/app-info/` is the
third and last file to import `bindings.ts`, and it reads a **constant** and not a command —
see below.

### Calling a command

`commands.queryNotes(query)` returns a **discriminated result**, not a promise that rejects:
`{ status: 'ok', data }` or `{ status: 'error', error }`. Repositories pass it through
`unwrap()` (`core/ipc/ipc.error.ts`), which returns the data or throws an `IpcError`. Stores
and components therefore keep the `try`/`catch` they already had, and `ErrorNotifier` stays
the one place that branches on a cause.

### Error contract

Commands return `Result<T, AppError>`, never `Result<T, String>`. An `AppError`
(`src-tauri/src/error.rs`) carries a stable **code**, its interpolation **params**
and a technical **detail**:

```json
{ "code": "duplicateSpaceName", "params": { "name": "Perso" }, "detail": "Un espace nommé …" }
```

This exists because business rules live in Rust. A message written there would be French in
an English UI, and branching on a cause would mean parsing a sentence that breaks at the
first rewording.

The mapping lives in **one** place, `core/services/errors/error-notifier.service.ts`: `ipcNotice(error, fallback)`
turns a failure into the message that helps most. A named cause wins over the attempted
action — "this note no longer exists" beats "could not save the note", which would leave the
user retrying something that can never succeed. `fallback` is used when the cause adds
nothing actionable (a generic SQLite failure) or when there is no code at all.

`IpcErrorCode` is a plain alias of the **generated** `ErrorCode` union, so it is no longer a
mirror at all — adding a Rust variant makes it appear on the front at the next generation.
Two tables then fail to compile until the new case is handled: `CODE_KEYS`, typed
`Record<IpcErrorCode, string | null>`, and `IPC_ERROR_CODES` in `ipc.error.ts`.

That second table is a runtime guard, and it still earns its place: `bindings.ts` _declares_
the error branch as an `AppError`, but Tauri itself rejects with a plain **string** for an
unknown command or an argument that fails to deserialise, and that value lands in the same
branch. `IpcError.code` is `null` in exactly those cases, and the message falls back to the
raw cause so the failure stays readable.

`ErrorCode` has no variant for a too-recent schema: that failure is only produced by the
migration during Tauri's `setup()`, where it aborts startup. No command can return it, so
giving it a code would advertise a case the front can never handle.

Three codes exist for what happens **outside** the database: `attachmentNotFound`,
`fileAccess` (reading, copying or writing a file — attachments, export and import all land
there) and `importFormat` (a file offered as a bundle that is not one, or one written by a
newer version). They are codes rather than a generic storage failure because each has a
different thing to tell the user, and only `importFormat` means "choose another file".

### Serialisation contract

**The wire types are generated; the conversion is not.** `model/` is the vocabulary the
application reasons in — what stores, components and templates manipulate; `data/` is the
boundary — the shape that crosses the bridge, the repository that crosses it, and the
conversion between the two. There are **two** shapes and not three: the wire types are the
generated ones, imported under a `Wire*` name where a model type carries the same name, and
`note.mapper.ts` holds the conversion. Nothing re-declares a wire shape by hand.

Where the two shapes coincide, the model type travels as it is: a space still has no mapper.
An identity mapper is not symmetry, it is one more name for one type.

What generation does **not** remove, and why `core/data/note.mapper.ts` is still
the biggest file in `data/`:

- **JSON has no date type.** Rust types every timestamp as a `String`, so the bindings do too.
  The mapper parses it into a `Date` and throws a `ContractError` on an unparseable value,
  rather than letting an `Invalid Date` propagate and resurface as `NaN` in a relative-time
  label. The reverse direction (`toIsoString`) guards the same way.
- **The front no longer narrows the language.** It was a free `String` in the model;
  the front restricts it to a `LanguageTag`.
- **A patch omits what it does not touch.** The Rust fields carry `#[specta(optional)]`, so
  the generated `NotePatch` has optional keys and `toNotePatchDto` can copy field by field —
  an explicit `undefined` would serialise to `null` and overwrite the stored value.

The serde attributes are still load-bearing (`rename_all = "camelCase"` on the structs,
`tag = "kind"` on the data-carrying enums), but they no longer need to be mirrored by hand:
specta reads them and the generated types follow. The tests in `notes/model.rs` that pin the
JSON shape are now a second line of defence rather than the only one.

An unknown `language` value degrades to `txt` instead of failing the load, and an unknown
entry in `availableLanguages` is dropped from the rail: a newer backend may know a language
this front-end build does not — and since Rust types it as a plain string, the bindings cannot
rule it out. A section key needs no such guard any more: `NoteSectionKey` is generated, so a
variant added in Rust breaks the assignment at compile time instead of throwing at runtime.

The known list is `notes/language.rs` (`Language`), aliased by `core/model/language.model.ts`
(`LanguageTag` + `LANGUAGE_LABELS`). Adding a language means editing both, plus a `.lang-*`
rule in `language-badge.component.scss` and, if it should be coloured, an entry in `GRAMMARS`.
The typed tables drift no further than the next build; the stylesheet is the one that can, and
`scripts/language-hues.test.mjs` compares it against the enum for exactly that reason.

**The language is detected, not asked for.** `notes/language.rs` reads the content and returns
one of `LANGUAGES`; without it every note is born `txt` and the format rail only serves people
who remember to touch the select. Three things keep it honest:

- `txt` doubles as **"nothing chosen"**, and detection runs on exactly two moments, both of
  which are a note acquiring its first content:
  - `with_detected_language` on a draft that reaches `create_note` as `txt` — the capture
    shortcut, which pastes and creates in one go;
  - `language_after_patch` when a patch gives a **still-empty** note its content — the ordinary
    "+ New note, then paste", where creation sees no content at all. Applied from
    `notes::store::update`, which calls into the model for the rule the same way it calls
    `notes::model::normalize_tags`.
- It **never replays afterwards**. Once a note has content, or carries a language other than
  `txt`, or the patch sets a language itself, nothing is guessed: re-detecting on every write
  would take the select back from the user, and there would be no way to overrule a bad guess.
- The heuristics are cheap and **allowed to be wrong**: the result is a starting value the
  editor can change. A miss costs one click.
- Order runs from the most discriminating signal to the vaguest (a wrapping brace beats a
  `key: value`), so each new rule goes in at the position its confidence earns.
- The editor commits a **paste** immediately rather than on blur (`onBodyInput` tests
  `inputType === 'insertFromPaste'`). The language is only known once the content is persisted,
  so waiting for the blur would leave the badge on TXT — which reads as a failed detection.
  Plain typing stays deferred: that is what avoids one round trip per character.

### Rules

- A new command needs **one** registration: `collect_commands![…]` in `src-tauri/src/lib.rs`.
  Annotate it `#[tauri::command(async)]` — see the threading note under
  [Persistence](#persistence-rust) — **and** `#[specta::specta]`, then regenerate; an
  unannotated function will not compile inside `collect_commands!`.
- A module holding commands is `pub`, and that is not decoration: `#[specta::specta]`
  generates a macro per command that `collect_commands!` resolves from the crate root.
  Everything else is `pub(crate)` or narrower, so that `dead_code` and `unreachable_pub` —
  both denied in `Cargo.toml` — still have something to say. A blanket `pub` silenced them
  across the whole back end, and three unused items had accumulated behind it.
- Every type crossing the bridge must derive `specta::Type` alongside its serde derives.
- Specta refuses to export `usize`, `isize` and the 64-bit-and-wider integers, since JSON
  cannot carry them without precision loss. Use a sized type the wire can hold — `NotesView.matched`
  is a `u32` for exactly this reason.
- Commands are **adapters only**: validate the input, lock the shared connection, delegate,
  translate the error. A command that grows is a sign a rule was written in the wrong place.
- `bindings.ts` is excluded from ESLint and Prettier: its shape belongs to the generator, and
  reformatting it would make every regeneration a diff.

### The downward direction: events

`bindings.ts` covers the front asking the back a question. The reverse — the back telling the
front something happened — goes through **`AppEventsService`** (`core/ipc/app-events.service.ts`),
which wraps `listen` from `@tauri-apps/api/event`.

**One topic, carrying a closed action.** There used to be three (`devnotes:capture`,
`devnotes:new-note`, `devnotes:palette`), spelled out on both sides, where a typo produced a
subscription that was silently inert and that nothing reported. Now `desktop::GlobalAction` is a
`closed_enum!` and the topic is a single constant, and **both are generated**: `lib.rs` exports
them with `.typ::<GlobalAction>()` and `.constant("GLOBAL_ACTION_EVENT", …)`, neither of which
needs a command to hang off. The front imports both from `bindings.ts`, and its `switch` over
the action is exhaustive — a variant added in Rust stops the front compiling.

**Application metadata travels the same way.** `app_info::METADATA` is exported with
`.constant("APP_METADATA", …)` and holds what the front used to spell out: the display name,
the repository URL, the author and their handle, plus the Rust toolchain the project pins —
`build.rs` reads the `channel` out of `rust-toolchain.toml`, the same file rustup resolves for
every build. The about card completes the line with Angular's own `VERSION.full` and with
`getTauriVersion()`: that one is **asked of the running framework**, not baked in, for the
same reason as the application's own version — a committed `bindings.ts` can lag behind a
dependency bump, an answer from the bridge cannot. Every value comes from `Cargo.toml` — the
standard fields through `CARGO_PKG_*`, and what Cargo has no field for through
`[package.metadata.devnotes]`, which `build.rs` hands to the crate as environment variables
read with `env!`. It is a constant and not a command on purpose: the titlebar reads the name
**synchronously**, where a round trip would leave it empty for a frame. `core/services/app-info/`
re-exports it once as `APP_INFO`, so nothing else imports `bindings.ts` for it.

⚠️ The **version** is not in it. It is read from the running binary with `getVersion()`, which
cannot go stale the way a committed `bindings.ts` can, and `tauri.conf.json` no longer declares
one either — without the key, Tauri takes the version from `Cargo.toml`. The release workflow's
`prepare` job reads that version and refuses to go on unless `package.json`, `package-lock.json`
and `Cargo.lock` say the same thing — **before** the tag exists, so it prevents the drift rather
than reporting it afterwards.

That is deliberately **not** `collect_events![…]`: it would generate a `listen` call per event,
imported straight from `@tauri-apps/api/event`, and the `EVENT_SUBSCRIBER` token every spec
substitutes would have nothing left to stand in front of.

Today it carries the desktop integration, which lives in `src-tauri/src/desktop.rs` — global
shortcuts and the system tray. It is native glue rather than a feature, so it sits beside
`notes/` and `spaces/` rather than inside either. None of it needs a
capability: capabilities gate the API the **WebView** calls, not what the native side does on
its own.

Two producers, **the same three actions**, so the front wires them once:

- `Ctrl+Alt+V` / `Ctrl+Alt+N` / `Ctrl+Alt+P`, registered at startup;
- the tray menu's "new note", "paste from clipboard" and "quick paste" items.

Each reveals the window and emits `GlobalAction::Capture`, `NewNote` or `Palette`;
`NotesPageComponent` listens once, reads the clipboard, creates the note or opens the palette.

⚠️ **A global shortcut is first-come, first-served across the whole machine**, and the loser
gets no error — the key simply does nothing. `Ctrl+Alt+Space` was the palette's first choice
and lost it to a widely installed application, which is why it is now `Ctrl+Alt+P`. Losing one
is still possible, so `set_global_shortcuts` **returns** what it could not take and the front
says so. A log line is not an interface. The same command re-registers the three from scratch
whenever the preference changes — everything is released first, or an abandoned combination
would keep answering.

- **Rust does not create the note.** Keeping creation on the front means one creation path
  (`create_note`), so a captured note gets language detection without a second implementation,
  and the adapter stays thin.
- Neither the topic nor the action set is spelled twice any more: both are generated, so a
  typo cannot produce the silently inert subscription this used to risk.
- A shortcut already taken by another application is **logged and ignored**, never fatal:
  DevNotes has to start without it.
- `AppEventsService.on()` returns an unsubscribe immediately although the subscription only
  lands a tick later; a component destroyed in between would otherwise stay subscribed for the
  whole session.

### Window geometry

`tauri-plugin-window-state` remembers the size, the position and whether the window was
maximized, in `.window-state.json` beside the database. `tauri.conf.json` still declares a
size, and it is only a **first** launch: 1100×720, with a 640×480 floor so a window cannot be
restored — or dragged — to something nothing fits in.

⚠️ The flags are explicit (`WINDOW_STATE_FLAGS`, `lib.rs`) and that is the whole subtlety.
The plugin's default is `all()`, which includes `VISIBLE`; quitting from the tray saves a
window that is **hidden**, and the next launch would restore it hidden — an application that
starts with nothing on screen and only a tray icon to be found by. A unit test holds the flag
out. `DECORATIONS` and `FULLSCREEN` are left out for the opposite reason: nothing here
changes either, so saving them stores noise.

A minimized window needs no guard of ours — the plugin's `Moved` and `Resized` handlers
both skip one, which on Windows reports itself at -32000. And the file is written on
`RunEvent::Exit`, not on every move: the tray's "Quitter" is `app.exit(0)`, so it goes
through, while a force-kill saves nothing and leaves the previous geometry standing.

⚠️ **The window is declared `"visible": false` and shown from `setup`.** The plugin restores
the geometry from `on_webview_ready`, which runs _after_ the window is on screen: created
visible, the window appeared at the config's size and then jumped to the remembered one.
Measured by polling the window rectangle through startup — 1116×759 at +172 ms, 900×600 at
+359 ms, so nearly 200 ms of the wrong window. Created hidden, there is one rectangle and no
jump. `setup` is also the right place rather than the front end: a front end that fails to
boot would otherwise leave a process with no window at all.

`backgroundColor` is the dark `--bg-0`, for the same reason the dark palette is the base
one — the WebView paints white before the first frame, and the theme preference cannot be
read before Angular boots.

### System tray

DevNotes stays resident in the notification area, and **the window's close button only hides it**
— quitting goes through the tray menu. An app made to be one shortcut away would be pointless
if closing it killed the shortcut. It is a preference now (see _Preferences_), still on by
default; minimising to the tray is the same idea, off by default. Tauri emits nothing for
"minimised", so `lib.rs` watches `Resized` and asks the window where it stands.

- **The front creates the tray, not the native startup.** `TrayService` (`core/services/tray/`) pushes
  the menu labels through `sync_tray`, and Rust holds **no user-visible string at all**: the
  interface language is a front-end preference, and a translation table in Rust would be a
  second source to keep in step. The subscription re-emits on every language change, so the
  menu re-translates itself.
- **Closing only hides when a tray exists** (`desktop::tray_exists`). Without that guard, a
  desktop with no notification area would leave a hidden window and a process nothing could
  bring back.
- `sync_tray` is the one command that returns no `Result`. A missing tray is not a failure the
  front can act on, and giving it an `ErrorCode` would add a branch no UI would ever render —
  it is logged natively, like an unavailable global shortcut.

The commands, grouped by the feature that owns them:

| Feature       | Commands                                                                                                                                                                                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `notes`       | `query_notes`, `create_note`, `update_note`, `delete_note`, `delete_notes`, `restore_notes`, `list_trash`, `purge_notes`, `empty_trash`, `move_notes`, `tag_notes`, `list_tags`, `rename_tag`, `merge_tags`, `delete_tag`, `fill_placeholders`, `set_placeholder_values` |
| `spaces`      | `list_spaces`, `create_space`, `rename_space`, `pin_space`, `delete_space`                                                                                                                                                                                               |
| `attachments` | `attach_file`, `attach_clipboard_image`, `list_attachments`, `read_attachment`, `open_attachment`, `save_attachment`, `delete_attachment`                                                                                                                                |
| `transfer`    | `export_notes`, `export_selection`, `import_notes`, `share_notes`                                                                                                                                                                                                        |
| `desktop`     | `sync_tray`, `unavailable_shortcuts`                                                                                                                                                                                                                                     |

The guarantees the front-end relies on (persisted value returned, `Err` on an unknown id,
"absent field means unchanged" for patches) are implemented in each feature, and tested there.
The batch commands return a **count** rather than a `Result` per note: a selection can hold an
id that has gone stale, and failing the whole batch for one of them would be worse than a
partial result.

`fill_placeholders` is the one command with no `State<Db>`: it is a pure function, so the
palette can fill an unsaved draft as easily as the note it just opened.

`delete_space` takes a **refuge** (`targetSpaceId`) and is the one command whose argument is
multi-word, so it is the first to actually exercise Tauri's camelCase renaming. The refuge is
not optional: `notes.space_id` carries an `ON DELETE CASCADE`, so a bare delete would take
the notes with it. `spaces::store::delete` moves them and drops the space in one
transaction, in that order, and deliberately leaves `updated_at` alone — the canvas orders on
that column, and refreshing it would float the whole absorbed space to the top as if every
note had just been edited. A space cannot be its own refuge (`spaces::model::validate_move_target`);
the cascade would take the notes back out one statement after the move.

`query_notes` takes a `NotesQuery` (space, search, quick filter, tags, languages, `now`,
`tzOffsetMinutes`) and returns a `NotesView` (sections, `availableTags`, `availableLanguages`,
`isFiltering`, `matched`). The pair `isFiltering` + `matched` is what lets the UI distinguish
"no results" from "this space is empty" without recomputing anything. Its two steps are
visible in the command body: `notes::store::fetch` runs the indexed SQL, `notes::view::build`
applies the rules to what came back. `fetch` returns the notes plus a `Facets { tags,
languages }` — the two rails ask the same question of two columns, and passing two bare
`Vec<String>` side by side would be indistinguishable at the call site.

**Tags and languages are both facet rails**, and behave identically: union semantics (a note
passes if it carries _at least one_ of the selected values), facets scoped to the space rather
than to the current filter, and a selection counts as `is_filtering` — which collapses the
canvas into a single flat `results` section. The quick filters (pinned / untriaged) do not:
they narrow a view that stays chronological. One asymmetry: selected tags go through
`notes::model::normalize_tags` before hitting SQL, selected languages do not — a language is picked
from a closed list, not typed, and `notes::language` compares it exactly.

The serialisation contract is pinned by tests in `notes/model.rs`, `notes/view.rs` and
`spaces/model.rs` rather than left to review: they assert the emitted
JSON keys are camelCase, that a lifecycle serialises to `{"kind":"expires","at":…}`, that a
section key serialises to `"older"`, that a decorated note serialises **flat**, and that an
error code serialises to `"noteNotFound"`. A serde attribute deleted by accident fails
`cargo test` instead of silently breaking the UI.

### Input validation

The back validates what the front already constrains, because a rule held only by a form is
not held at all. `error.rs` defines a `ValidationError` carrying the offending
`field`; commands call `draft.validate()` / `draft.validated_name()` before touching the
connection, and `AppError` turns the refusal into `invalidInput` with `{{field}}`.

What is checked: `language` against the known list (an arbitrary value would be unreadable by
any front build), a space name trimmed and non-empty (`COLLATE NOCASE` folds case but not
whitespace, so `"Personal "` would otherwise sit beside `"Personal"`, identical on screen), and
`NotesQuery.now` as a parseable instant — falling back to the server clock would silently
re-cut every section on a different day.

## Encryption at rest

The library is sealed with a key derived from a passphrase typed once per launch. Nothing
is kept of it: no keychain, no "remember me", no recovery — the semantics of a KeePass file,
and the same consequence.

### Why not SQLCipher

Measured rather than assumed. SQLCipher encrypts every 4 KiB page with AES-256-CBC and
authenticates it with HMAC-SHA512; on the 8000-note benchmark corpus that costs ~112 ms on
`query_notes`, 88% of it in the HMAC. It also has to be built — the crate wants a vendored
OpenSSL, which means a C toolchain in CI on every platform, for every build.

Sealing values instead costs ~28 ms on the same corpus, four times less, because it seals
what a reader would want rather than every byte the file system moves. The crates are pure
Rust (`aes-gcm`, `argon2`, `getrandom`, `zeroize`): no `build.rs`, no C, nothing added to a
CI build. On a realistic library (~2 MB) the sealing costs well under a millisecond.

### What is sealed, and what is not

Sealed: note titles, bodies and sources, checklist item texts, space names, `{{field}}`
values and their global defaults, attachment file names — and the attachment files
themselves, bytes and all.

Not sealed, deliberately: tags, instants, ids, `kind`, `language` and the foreign keys.
They are what SQL filters, sorts, groups and joins on, and sealing them would move every
query into Rust over the whole corpus. ⚠️ Tag names are the visible cost of that line, and
the one thing a reader of the raw file learns.
`a_note_is_not_readable_in_the_file_it_was_written_to` (`tests/notes.rs`) greps a freshly
written database and asserts exactly that split — the only test here that reads the file
rather than the API.

### What this protects against, and what it does not

The threat is a file read at rest: a stolen laptop, a copied profile directory, a backup
that ended up somewhere it should not have. Against that, a value is unreadable and a
tampered one is refused rather than decrypted into nonsense.

⚠️ It is **not** a defence against someone who can write to the file while you are away.
A sealed value is authenticated on its own and not bound to the row it sits in — no
associated data — so a value could be moved from one row to another and its tag would
still verify. Binding it would mean threading the row identity through every seal and open
call in the stores; it buys nothing against the threat above, where the attacker reads the
file rather than edits it and hands it back.

⚠️ Nor is it a defence against a machine already compromised while DevNotes runs: the key is
in this process’s memory for the length of the session, and there is no idle re-lock.

### The pieces

- **`vault/key.rs`** — `Vault`: the key, and the two operations. Argon2id (`Cost`: 64 MiB,
  3 passes, 1 lane — 1.16 s on the development machine, which is the point of it) and
  AES-256-GCM with a fresh 96-bit nonce per write, laid out `nonce || ciphertext || tag`,
  base64 for a TEXT column and raw bytes for a file. The key is a `Zeroizing<[u8; 32]>` and
  the hand-written `Debug` prints `Vault(…)`, so it cannot reach a log line.
- **`vault/file.rs`** — `vault.json` beside the database: format version, KDF parameters,
  salt, and **the library's key sealed under the phrase**. ⚠️ The phrase does not derive the
  key the notes are sealed with, it wraps it: the key is random, and a new phrase re-wraps
  the same one. That is what makes a passphrase change a hundred bytes of rewriting rather
  than re-encrypting every note and every attachment — an operation that could not be
  atomic across the database and the files, and would leave a half-readable library if it
  stopped halfway. Opening the wrapped key is also the check: a phrase that fails to open
  it is the wrong phrase, said by the authentication tag, so there is no separate check
  value to attack. Written staged-then-renamed, and `create` refuses to overwrite one.

  ⚠️ The other side of that coin, and the reason it is worth writing down: changing the
  passphrase answers a phrase somebody **learned**, never a key somebody **took**. Whoever
  got hold of the unwrapped key keeps it, exactly as with LUKS or KeePass.

- **`vault/migrate.rs`** — `seal_existing`, one transaction that seals a library written
  before any of this. ⚠️ It runs from `create_vault` **after** the library is open and
  **before** the startup sweeps: the orphan-attachment sweep reads stored file names and
  would meet them in the clear if it ran first.
- **`db.rs`** — `Library` carries the `SqliteConnection` **and** the `Vault`, and derefs to
  the connection so the store functions did not have to grow an argument. `split()` hands
  the two fields over separately where the borrow checker needs both at once, and
  `Db = Mutex<Option<Library>>` is empty until `unlock_vault` fills it — a command that
  runs before the unlock answers `StorageError::Locked` rather than reading a database
  nobody opened.

### The gate

`vault_state` answers `absent` / `locked` / `unlocked`; `app.config.ts` awaits it before the
first render and `app.component.html` puts `VaultGateComponent` in front of the outlet. ⚠️ It
does not hide the outlet, it never creates it — which is what keeps every store free of a
"locked" branch: the canvas queries notes the moment it mounts, and nothing would be there
to answer.

⚠️ The unlocked state lives in Rust and not in the front end: a page reload must not ask
again for a library this process already has open. That is also what keeps `reopenSession()`
working in the end-to-end suite, where the front end reboots and the process does not.

### Changing the passphrase

`change_passphrase` (from Préférences → Sécurité) unlocks with the current phrase, then
rewrites the key file with a fresh salt and the same library key wrapped under the new
one. ⚠️ It refuses before writing anything when the current phrase is wrong — a change
that took it on trust would lock the library behind a phrase nobody chose. The open
session is untouched: the key in memory is the one that was already there.

⚠️ The command asks for the connection only to check that the library is open, then
releases it: holding the mutex across the derivations would freeze every other command.

⚠️ **Rewriting the live key file alone revoked nothing**, which is the whole of #157. A
change is the gesture somebody makes when they think the old phrase leaked — and every
retained backup kept a key file of its own, still wrapped under it, inside the same profile
directory the library is in. Whoever copied the profile copied every phrase the user had
ever retired, and the envelope means **one master key for the life of the library**, so any
key file ever written is a permanent escrow for it. A note written _after_ the change opened
under a phrase abandoned before it.

So `backup::rewrap` is the second half, and `vault::change_with` is the one place that holds
both. Three things decide its shape:

- It writes the key it is **given** rather than opening each copy with the phrase being
  retired. A backup's file wraps that same master key whatever phrase was current when it
  was taken, so this retires **every** abandoned phrase at once — including one from three
  changes ago, which could never have been opened with the current phrase to be rewrapped
  the obvious way.
- **The live file first.** If it cannot be written the whole change fails having touched
  nothing; it is the only one whose loss is fatal. Each copy then goes through the same
  staged-then-renamed write, so a copy is atomic for itself even though the set is not.
- A copy it cannot rewrite is **counted, not fatal**. Refusing to rotate the phrase because
  one backup's file is locked would block the revocation at the moment it is asked for.
  `PassphraseChange { backupsRewrapped, backupsLeft }` crosses the bridge and `VaultStore`
  turns it into one of three sentences — the report comes from the store and not from the
  dialog, which closes on the click.

⚠️ `damaged/` is deliberately absent: `recovery::set_aside` leaves `vault.json` where it is,
so a set-aside library has no wrapping of its own to retire and opens under the new phrase
like the live one. And the dialog says the part the application cannot act on — a key file
the user copied elsewhere still opens with the old phrase, and only the user knows about it.

### Revisions: the body before the edit

The trash protects a deletion and nothing protected an edit. You adjust a command that
worked, it stops working, and the version that worked is gone. ⚠️ The point is not the
restoring — it is the **ease**: a text you know is recoverable is a text you edit freely,
and touching a snippet that works stops costing nerve every time.

`note_revisions` (migration 12) follows `notes::trash`'s shape rather than inventing one:
a retention constant, a prune that runs on write, and a table the cascade takes with the
note.

**What is kept, and what is deliberately not.**

|                      |         | why                                                                                                                                          |
| -------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| the **body**         | kept    | the 90% case and by far the cheapest; title, tags and language are almost never what anyone wants back                                       |
| **snippets** only    | kept    | a checklist's items live in `note_items` — a second table to snapshot and a two-step restore                                                 |
| the last `KEEP` (20) | kept    | ⚠️ a **count**, not a time window: a body runs to tens of kilobytes, so a cap is the only bound that is predictable for storage              |
| exports              | **out** | revisions in the bundle would inflate it by a factor of the cap; a note restored elsewhere arriving without its history is the accepted cost |

⚠️ Both omissions are held by a test rather than left to be noticed —
`a_checklist_keeps_nothing_yet` and `a_bundle_carries_no_history_of_the_bodies_it_holds`.
Half the note kinds get nothing from this first version, and that is said out loud rather
than shipped silently.

**When one is taken.** Inside `notes::store::update`, in the same transaction, **before**
the row moves: what is worth keeping is the body as it _was_. `update` already computes
`before` for its own change detection, so the signal costs nothing.

⚠️ `record` skips the write when the newest kept body already holds that exact text. The
editor commits on blur _and_ on every closing path, so one editing session produces
several writes of the same body — without the skip, each of them would push a duplicate
and rotate a genuinely different version out of the cap.

⚠️ `content` is **sealed**, like the column it copies. A history of every body in
plaintext beside a sealed library would undo the encryption entirely, and
`a_kept_body_is_not_readable_in_the_column_it_sits_in` holds that against the raw column.

⚠️ The listing orders by **`rowid`**, not by `taken_at`. The editor commits the title, the
source and the body back to back, so two revisions can share a millisecond — and the
tiebreak was a random UUID, which put the older one first about half the time. Insertion
order is what "newest" means here, and `rowid` _is_ insertion order. (The same trap the
seeding met, from the other end.)

**Restoring.** ⚠️ `restore_revision` does **not** touch `updated_at`. Putting something
back is not editing it — the fifth place in the codebase holding that line, after
`restore_many`, `restore_placements`, `untag_many` and `set_placeholder_values` — and the
canvas sorts on that column.

⚠️ **Restoring is going back to that point, and it is irreversible.** The note takes the
kept body, and that body **and every one kept after it** leave the history
(`revisions::discard_from`, by `rowid` like the listing); nothing is kept of the text it
replaced. A → B → C, back to B, and C is gone. It used to keep the replaced body first, so
a restore added a row — and the panel then showed two, one of them identical to the text
on screen, with nothing to say what either meant (#325).

⚠️ So **a row opens a preview**, and only the preview restores. `revision_diff` answers the
kept body against the current text as `DiffLine`s — `kept`, `restored`, `dropped`, and
`skipped` for unchanged runs folded to a count, three lines of context either side —
computed in Rust with the `similar` crate (Myers, a 300 ms deadline past which it
approximates). The preview says what goes ("the current text and N newer versions") and
carries the restore button, drawn destructive: opening the preview is the first step,
which a double click on a row cannot defeat — the shape emptying the trash has.

⚠️ `content_of` narrows on the **note id as well as** the revision id: an id comes from
the front end, and one note's history must not be reachable through another note.

**On screen**, a fold-away under the editor's body, shown only when there is something to
show — "a header always present and always empty would be a feature nobody uses" is the
editor's own rule, applied again. ⚠️ `NoteRevisionsStore.openFor` reloads even for the
note it is already on, and must: the editor is **destroyed** when it closes, so nothing
ever calls it with `null` on the way out. Skipping the work for a matching id left the
panel showing what the _first_ open found — an empty history that never came back.

### Several libraries, and the registry beside them

There used to be exactly one library and its address was compiled in. `app_data_dir()` for
the directory, `devnotes.sqlite3` for the database, `attachments/` and `backups/` beside
it — four names that are the **address of a library** rather than decoration.

```
data_dir()/com.devnotes.app/
  libraries.json          ← the registry: the list, and which one is open
  preferences.json        ← the application's preferences
  open/                   ← decrypted attachment copies, every library's
  libraries/
    <id>/
      devnotes.sqlite3    vault.json      preferences.json
      attachments/  backups/  damaged/  archived/  replaced/
```

⚠️ **The registry is beside the libraries and no library owns it.** One that is deleted,
moved by hand or sealed under a forgotten passphrase takes nothing else with it — which is
exactly the situation #252 is about, seen from the other end.

⚠️ **Every library has the same shape**, including the one that was already there:
`libraries::gather` moves it — database, sidecars, key file, attachments and the four
directories a library accumulates — into `libraries/<id>/` the first time the registry is
read. That move is best effort and never fatal, and the same move runs again over whatever
is left, so a half-finished one is picked up by the next launch. Uniformity is what makes
the first library deletable like any other and keeps the profile root down to two files.

**`libraries::open_directory(app)` replaced `app.path().app_data_dir()` at a dozen call
sites** — the vault's five, the backups' two, the attachments, the recovery. ⚠️ A module
that reaches for the profile directly writes into whichever library happens to be first,
whatever is open. The one that stayed is `open/`, the decrypted attachment copies: they
are ephemeral and swept wholesale, and one directory means one sweep catches every
library's leftovers.

**Switching is a full teardown, and the front end is reloaded for it.** `open_library`
empties the connection `Mutex` under the same lock every other command takes, then
`LibrariesStore` reloads the page (`AppWindowService.reload()`). The vault state lives in
Rust, so the front end comes back on the gate of the library just opened. ⚠️ The other
library has its own passphrase, and asking for it is the only proof the right one is open.

⚠️ **A reload, not a gate over the same stores.** Destroying the outlet is not enough: every
store is `providedIn: 'root'` and outlives it. `SpacesStore` loads once and nothing
reloaded it on a switch, so going back to a library that was already seeded left the rail
listing the other library's spaces and the board asking this database about a space it had
never held (#318) — with the selection, the retained views and the undo record behind them.
Resetting each store by hand was the alternative and was refused: a dozen stores to wire,
and the next one to forget brings the bug back. The cost is the reload itself, and the dark
base showing for as long as it does at every launch.

⚠️ **The gate says which library it asks for, and offers the others.** The File menu is
gated on an open library, so before this the gate asked for _a_ phrase and the only way to
another library was to open the wrong one first. A line at the head of every panel — form,
forgotten phrase, damaged file — names it; with several, that line is a choice menu calling
`openLibrary`, which reloads like any other switch. The one library that predates names
gets no line: "Library: Library" says nothing.

⚠️ **Creating opens.** One gesture rather than two: you have just named it, so you want to
be in it, and the gate then asks for a phrase exactly as a first launch does. The surprise
would be staying where you were.

⚠️ **Neither the open library nor the last one can be deleted**, refused in Rust _and_ in
the store — a command is reachable from more than the interface. Deleting files under a
live connection is how a library that was merely unwanted takes the process down; deleting
the last leaves the gate with nothing to offer, and the next read of the registry would
adopt an empty profile as a library nobody asked for. The deletion itself gets the
treatment emptying the trash gets: a sentence naming what goes, and a confirm somewhere
other than the button that fired it.

⚠️ The registry is **staged and renamed**, like an export file. `fs::write` truncates
first, so a disk that fills mid-write would leave a registry naming no libraries at all,
with every one of them still on disk and nothing pointing at them.

### Two scopes of preference, one prefix apart

A preference belongs to the **application** or to the **library**, and the line is one
prefix: ⚠️ **`devnotes.notes.*` is the library's, everything else is the application's.**

|                             | file                                        | holds                                                                                   |
| --------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------- |
| `PreferencesService`        | `preferences.json`, at the profile root     | theme, language, density, the keys, the tray, autostart, the window, `automaticBackups` |
| `LibraryPreferencesService` | `preferences.json`, inside the open library | the samples marker, and which view each space was left on                               |

Both extend `KeyValueStore`, which holds the synchronous cache and the plugin's own
loading — the API stays synchronous where the plugin's is not, because a preference is read
when a component is constructed and an async read would show the interface in one state
then the other.

⚠️ `automaticBackups` stays with the application deliberately: "copy my libraries at
launch" is a habit rather than a property of one corpus, and it is the one key Rust reads
out of that file before the front end has booted (`backup::wanted`).

⚠️ `LibraryPreferencesService` is re-opened on **every** switch, where the application's is
opened once. A space id means nothing in another library, and a samples marker carried
across would leave a fresh library empty with no way to create a note — a note needs a
space.

⚠️ It also **adopts** the library-scoped keys out of the application's file, once. Every
install before the registry kept both scopes in one, and without the adoption the first
launch after the upgrade re-seeds a library that is full.

⚠️ The word "library" was already taken on screen: the sidebar rail was labelled
_Bibliothèque_ while listing **spaces**. It now says _Espaces_, which is what it lists, and
the name is free for what the whole codebase already calls a library — the database, the
vault, "the library is locked".

⚠️ `25-libraries.e2e.ts` is **last**, after `24-forgotten-passphrase`: that one archives
the library and leaves the gate asking for a phrase on a fresh one, which is the state this
file needs. It ends on a gate too, so nothing may be filed after it.

### The copies, and putting one back

`backup::rotate` takes one at unlock, before the sweeps, into `backups/<stamp>/`: a
`VACUUM INTO` of the database plus its key file, at most one a day, the last `KEEP` kept.
None of that used to be visible. There was no list, no date, no size, no way to put one
back, and the switch that controls it stood with nothing beside it — a safety net nobody
can see is one nobody trusts, and one nobody can use.

The **Sécurité** page now says it where the application can be read from: what the copies
are, where they live, how many are kept, and what they do **not** cover — a copy inside
the profile answers an emptied trash, a botched update or a slip of the hand, not a dead
disk, and it carries the notes and the key but **not the attached files**.

`list_backups` answers `Backup { id, takenAt, bytes, openable }`, newest first. ⚠️
`openable` is the key file: a copy without one opens for nobody, so it is **listed and
never offered** — proposing it would be proposing to lose the library for nothing. ⚠️
`bytes` crosses as `f64`, which specta types as nullable because JSON cannot carry `NaN`;
`backups.repository.ts` puts that back, alongside the ISO instant.

**Restoring is the one gesture in the application that replaces a whole corpus**, and it
is built like it.

- ⚠️ `restore_backup` empties the connection `Mutex` **before a single file moves**, under
  the same lock every other command takes. Renaming a database out from under a live
  connection is how a working library becomes a lost one.
- ⚠️ `backup::replace` moves the live database **and** `vault.json` into
  `replaced/<timestamp>/` before copying the chosen pair in, and moves them back if the
  copy fails. A failed restore must never leave no library at all. Nothing is deleted: a
  folder with a date on it is the difference between a mistake and a loss.
- ⚠️ `vault.json` travels, unlike `recovery::set_aside` where it deliberately stays. The
  copy brings its own wrapping, and a database from one wrapping with a key from another
  opens nothing.
- ⚠️ `attachments/` **stays**, also unlike `set_aside`. The copies do not carry it — it is
  the bulk of a profile — so moving it aside would point every restored record at a file
  that left. The next launch's orphan sweep collects whatever the restored library no
  longer names, which is right: those files belong to notes it does not have.
- ⚠️ The id is matched against the listing rather than joined onto `backups/`. It comes
  from the front end, and `../2026-01-01_00-00-00` joins to a path outside the directory
  whose file name still parses as a stamp.

Afterwards every command answers `Locked`, `VaultStore.load()` sees it, and the outlet is
destroyed — with the File menu the panel was opened from, which is gated on the same
signal. The restored copy needs a passphrase, and asking for it is the only proof the
right file is in place.

⚠️ `BackupsStore` follows the shape the tag manager and the trash already use: `ask()`
only **proposes**, `pending()` names which copy and what it would cost, and `confirm()` is
what writes. The trigger is replaced by that sentence and the confirm sits somewhere else
— a second click on the button that fired it is the guard a double click defeats. The
strip scrolls itself into view, because it replaces a trigger further down a panel that
scrolls and would otherwise appear below the fold.

⚠️ `23-backups.e2e.ts` runs **second to last**: its final scenario really replaces the
library and leaves it locked, which every other file would meet as a gate it was not
written for. The numeric prefix is the run order, and nothing may be filed after it.

### Attachments, and the one plaintext copy

The bytes are sealed on the way in (`attachments::copy_within_limit`) and opened in memory
on the way out: `read_attachment` decrypts into the `data:` URI the preview already used,
and `save_attachment` writes plaintext where the user chose to put it.

⚠️ `open_attachment` is the exception, and a deliberate one: the program that opens a
document reads it from disk, so DevNotes writes a decrypted copy under `app_data_dir()/open/`
and opens that — one click, as before. Those copies are swept on the way out
(`RunEvent::Exit`) and again at every launch, which is what covers one another application
still held, and a crash. ⚠️ The profile and **not** the OS temporary directory: that one is
shared with every account on the machine, where the copy would be readable by all of them
and a directory somebody else created first would be theirs rather than ours.

## Persistence (Rust)

Storage is **SQLite**, queried through **Diesel** and embedded via `libsqlite3-sys` with the
`bundled` feature — SQLite is compiled from source and statically linked, so nothing has to be
installed or shipped alongside the executable. The database file lives in Tauri's
`app_data_dir()`.

- **The store is plain functions.** `notes::store` and `spaces::store` take a
  `&mut SqliteConnection`; the `#[tauri::command]`s sit on top. That is what makes persistence
  testable against `SqliteConnection::establish(":memory:")` without launching Tauri. A store
  holds **no business rule** — it reads and writes the model defined in its sibling
  `model.rs`, which it depends on.
- **`db/schema.rs` is the typed mirror of the schema**, written by hand rather than
  produced by `diesel print-schema`, which would make `cargo check` depend on an up-to-date
  database sitting outside the repository. What it deliberately does not model — `CHECK`
  constraints, `ON DELETE CASCADE`, and the `NOCASE` collation on `note_tags.tag` — stays in
  the migration SQL. Diesel obeys those; it does not own them.
- **Concurrency.** A `SqliteConnection` is not `Sync`, and Diesel takes it exclusively for
  every query, reads included. A single connection is shared as `tauri::State<Db>`
  (`Db = Mutex<Option<Library>>`, the connection and the key together), registered with
  `.manage()` in `lib.rs` — never a global, and empty until the library is unlocked.
  Overlapping commands serialize on that mutex, and each command holds `db::lock` for its
  whole body, so a check and the write that depends on it cannot be interleaved.
- **⚠️ Commands that touch the database or the disk are `#[tauri::command(async)]`.** A plain
  `#[tauri::command]` is compiled as `ExecutionContext::Blocking` and its body runs **inline
  in the WebView's IPC handler** — on the main thread, where it freezes the window for as long
  as it takes. Exporting a library, importing one, reading a 10 MB attachment into a `data:`
  URI or copying a file all did exactly that. `(async)` on the same synchronous function moves
  the body off that thread; no signature changes and `bindings.ts` is unaffected, since the
  generated TypeScript was always promise-based.

  ⚠️ Where it moves it to is worth being precise about, because the name suggests otherwise.
  `tauri-macros` emits `respond_async_serialized(async move { … })`, which hands the task to
  `async_runtime::spawn` — and Tauri's default runtime is Tokio's **multi-threaded** one. A
  synchronous body therefore occupies a Tokio _worker_ for its whole duration; it is not
  `spawn_blocking`, whatever the `sync_threadpool` label on the macro's tracing span implies.
  The window is freed, which is the whole point, but the workers are a bounded pool the size of
  the core count, shared with the updater and the plugins. It holds because the connection
  mutex already serializes the database work behind it, and because no command here is
  long-running by design. A genuinely long one would need `async fn` plus an explicit
  `async_runtime::spawn_blocking`.

  The exception is `desktop.rs`: `sync_tray`, `set_global_shortcuts` and
  `set_window_behavior` stay blocking, because the tray and shortcut registration want the
  main thread. That is also why every piece of native state is managed in `desktop::init`
  before any command can run — a command creating its own on first call would race another
  doing the same.

- **Migrations.** They live as SQL files in `src-tauri/migrations/`, are compiled into the
  binary by `embed_migrations!`, and are tracked in the `__diesel_schema_migrations` table.
  Evolving the model means adding a `YYYY-MM-DD-HHMMSS_name/` directory — never editing a
  shipped migration, it has already run on user machines. Each migration is atomic. A database
  carrying a migration this binary does not know is refused rather than misread.
- **The legacy `PRAGMA user_version` history is adopted, not replayed.** The schema used to be
  versioned by that pragma (values 1 to 3). `db::migration::adopt_legacy_history` marks the matching
  embedded migrations as already applied and zeroes the pragma, so an existing install neither
  re-runs `CREATE TABLE spaces` nor keeps a second, drifting source of truth. A pre-Diesel
  binary reopening such a database now fails loudly at startup instead of writing into a schema
  it believes it understands.
- **The board geometry is columns nothing else reads.** `folders.x/y/w/h` are nullable and
  move together — `NULL` means "never laid out", which every folder made before the board
  existed is. `note_positions` is keyed on `note_id` alone and holds a row only for an
  **unfiled** note; filing one deletes its row. ⚠️ None of it is on the `Note` or `Folder`
  model, and that is what makes `#134` free: `transfer::Bundle` deserialises both, so a
  coordinate there would travel in every export and land on top of the arrangement the
  receiving machine already has.
- **A folder is a row, and membership is a column.** `folders` holds `(id, space_id, name,
colour, created_at)` and `notes.folder_id` points into it. ⚠️ The column was added with no
  `DEFAULT`, because SQLite only accepts an added `REFERENCES` column whose default is `NULL`
  — which is what every existing note needs anyway. `folders.name` is sealed like
  `spaces.name`, so uniqueness leaves SQL exactly as it did there: two seals of the same name
  differ, and a unique index on ciphertext catches nothing. The order does not move —
  `created_at` stays in the clear, and reading order is the order a board lays its zones out
  in. `colour` carries no `CHECK`, following `language`: the list lives in the domain, and a
  value a build cannot name degrades to the default rather than failing the read.
- **A checklist's items are a child table, not a serialised column.** `note_items` is keyed
  `(note_id, position)` and written by wiping the note's rows and re-inserting them in order —
  the same shape as `note_tags`, for the same reason. It is _not_ read back after writing, and
  that asymmetry with tags is deliberate: `position` orders numerically, which the insertion
  order reproduces exactly, whereas `note_tags.tag` is `COLLATE NOCASE` and only a read gives
  its order. `notes.kind` carries no `CHECK`, following `language` rather than `lifecycle_kind`:
  the list of kinds lives in the domain and can move between versions. Its `DEFAULT 'snippet'`
  is not a convenience either — SQLite refuses an `ADD COLUMN NOT NULL` without one, and it is
  what gives every note already in the database its value.
- **Filled `{{fields}}` are a child table too.** `note_placeholders` is keyed `(note_id, name)`
  and rewritten whole, like `note_tags` and `note_items` — what is no longer sent is what the
  user cleared, and a partial write would leave an emptied value still filling the text. Its
  key is **case-sensitive**, unlike `note_tags.tag`: `notes::placeholder` distinguishes
  `{{Host}}` from `{{host}}` in the text, and folding here would fill one with the other's value.
- **Schema choices that made filtering movable to the back-end.** `lifecycle` is split into
  `lifecycle_kind` + `lifecycle_expires_at` columns rather than stored as JSON, and tags live
  in their own `note_tags` table rather than in a serialised column. Both exist so that
  filtering by tag, or querying what expires before a date, is a `WHERE` clause instead of a
  full re-read — which is why `query_notes` needed no migration. `PRAGMA foreign_keys` is set
  per connection, which is what makes the `ON DELETE CASCADE` on notes and tags actually fire.
- **The four pragmas in `db::configure` are each a decision, and each has a test.**
  `foreign_keys` because the cascades are inert without it; `journal_mode = WAL` so a reader
  never blocks the writer; `busy_timeout = 5000` because SQLite's own default is **zero**,
  which turns a file another process holds for twenty milliseconds — a checkpoint, an
  antivirus, a second instance — into a storage error on the first write; and
  ⚠️ `synchronous = NORMAL`, WAL's default, written down because it is a durability choice:
  a power cut can cost the last committed transaction and cannot corrupt the file. `FULL`
  would fsync every commit to protect a note the user can retype.
- **Every open runs `PRAGMA quick_check`, and a damaged file is its own answer.** Nothing
  checked the database was still readable, so the first symptom was a query failing somewhere
  in the interface, reported as a storage error — "try again" about a file that will never get
  better on its own. `db::quick_check` runs in `db::open` ⚠️ **before the migrations**: they
  write, and running them over a damaged file is how a salvageable database becomes an
  unsalvageable one. `quick_check` and not `integrity_check` — the quick one skips the most
  expensive cross-checks and reads the file once, milliseconds at this size; the full check
  belongs behind a button, never on the path to a window. SQLite answers a single `ok`, or one
  row per problem, and those rows become `StorageError::Damaged` → `ErrorCode::LibraryDamaged`.
- **Detecting it without a way out would be a loop.** An application that refuses to start and
  explains why, every launch, leaves deleting a file by hand as the only move — so
  `recovery::set_aside` is the other half of the check. It tries `VACUUM INTO` first (best
  effort, and deliberately so: a partly readable database usually gives most of itself back,
  and a failure there must not stop the user getting a working application), then moves the
  database, its `-wal` / `-shm` sidecars and ⚠️ **`attachments/`** into `damaged/<timestamp>/`.
  The attachments are not a detail: a fresh library calls every file there an orphan, so the
  next launch's sweep would delete the pictures of the notes just set aside — the one way this
  recovery could destroy what it exists to save. ⚠️ `vault.json` stays where it is. The
  passphrase does not change, the rescued copy needs that exact key, and asking someone to
  choose a new passphrase in the middle of losing their library would be its own small cruelty.
  `set_aside_damaged_library` refuses while the library is open — moving the file under a live
  connection is how a damaged database becomes a lost one — and answers the folder it wrote,
  because "set aside" is only true if the user can be told where.
- **A forgotten passphrase is final, and the gate now says so with something to do about
  it.** It used to offer nothing but the field: the phrase wraps the library's key,
  opening `vault.json` is the only check there is, and the only way past was finding the
  profile directory in `%APPDATA%` and moving files by hand. `recovery::Reason` is what
  splits the two cases, because ⚠️ **the answers are opposite and getting either wrong
  loses the library for good**:

  |             | `vault.json`                                                                 | rescue                                                                  | goes to                 |
  | ----------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ----------------------- |
  | `Damaged`   | **stays** — the phrase still works and the rescued copy needs that exact key | `VACUUM INTO`, best effort                                              | `damaged/<timestamp>/`  |
  | `Forgotten` | **travels** — it is the only thing that phrase would ever open again         | none: the file is sealed and SQLite has nothing to give without the key | `archived/<timestamp>/` |

  `attachments/` goes along either way, for the same reason as before. `archive_locked_library`
  refuses on an open library like its neighbour, and answers where it wrote.

  ⚠️ With the key file gone, `vault_state` answers `absent` rather than `locked`, which is
  what turns the gate into the one that asks for a **new** phrase — a gate still asking for
  the old one would be the same dead end with extra steps. `VaultStore.moveAside` re-reads
  the state rather than assuming it, and forgets `devnotes.notes.samplesSeeded` so the fresh
  library seeds.

  On screen it is a small, quiet link under the unlock button, ⚠️ shown only on a library
  that exists — a fresh one has no phrase to have forgotten. It opens a panel **instead of**
  the form, never beside it: the field is the one thing that cannot help, and an offer to
  give up standing next to it reads as a shortcut. The panel names what is lost — the notes
  are not recovered, they leave sealed and unreadable — and ⚠️ its confirm is `destructive`
  rather than the amber fill, which is the button you are _meant_ to press.

  ⚠️ `24-forgotten-passphrase.e2e.ts` runs **after `23-backups`** and depends on it: that
  one ends by restoring a copy, which closes the library and leaves the gate on screen.
  There is no other way to meet a locked gate inside one run — the unlocked state lives in
  the process, and the process outlives every page reload. Nothing may be filed after it.

- **Ordering is the back-end's call.** `notes::store::fetch` orders by `updated_at DESC, id`;
  the front-end preserves the order it receives, so this one query decides what the user sees
  first. Note the deliberate asymmetry: the order is by `updated_at` while sections group by
  `created_at`. The section answers "when was this note born", the order within it answers
  "which did I touch last", so an old note reopened today tops the "older" section.
- **Querying splits the work by what each tool does well.** SQL handles what it indexes —
  space, pin state, lifecycle, language, and tag membership through a `note_tags` subquery
  (`notes::id.eq_any(...)`). The conditional criteria are assembled on a Diesel `into_boxed()`
  query, which is what replaced hand-numbered `?N` placeholders and their bound-parameter
  bookkeeping.
  Full-text
  matching is done **in Rust** (`notes::view`), because SQLite's `LOWER()` only folds ASCII
  without ICU, so `Étape` would not match `étape`. Grouping is `notes::view`, which
  touches no connection and is therefore testable without a database.
- **The search fold drops the accents as well as the case, on both sides.** `view::fold`
  lowercases and strips the combining marks, so `etape` finds `Étape` and `Étape` finds
  `etape` — the needle goes through the same function as every haystack. It decomposes one
  character at a time (`unicode_normalization::char::decompose_canonical`) rather than
  streaming the whole string through `nfd()`: on 800 notes of 13 kB of accented text, in
  release, that is 15 ms against 76 ms, and against 27 ms for the `to_lowercase()` it
  replaces — which folded no accent at all. Pure ASCII, which is what a snippet of code is,
  never leaves the fast path. Only what a **canonical** decomposition separates is folded:
  `ø` and `ß` are letters in their own right and stay. ⚠️ Tag normalisation
  (`notes::model::normalize_tags`) deliberately does **not** fold accents — `Étape` and
  `etape` are two tags, and merging them would lose one.
- **Tag normalisation lives in `notes::model::normalize_tags`, and only there.** Trimming,
  stripping leading `#`, dropping blanks and collapsing case-insensitive duplicates (first
  spelling wins) all happen on write, so the front sends what the user typed. The returned
  tags are sorted to match what a read gives back — otherwise a note's tags would reorder
  themselves on the next reload.
- **Tag case folds at the storage level too.** `note_tags.tag` is `COLLATE NOCASE`
  (the `fold_tag_case` migration). Without it `normalize` only deduplicated _within_ one note: `Urgent` and
  `urgent` carried by two different notes produced two facets in the rail, of which
  `tag IN (…)` — running in BINARY — matched only one, while the text search confused them.
  Three behaviours for one concept.
- **`notes.language` is indexed** (the `index_language` migration), since it became a filtering facet: both
  `language IN (…)` and the `SELECT DISTINCT language` that feeds the rail would otherwise
  scan the table on every query. No `CHECK` constraint on the column, though — the list of
  known languages lives in `notes::language` and moves between versions; freezing it in the
  schema would mean a migration per addition.
- **Timestamps are injected, not read.** `notes::store` takes `now` as a parameter and the
  command passes `Utc::now()` — the same reason `ClockService` exists on the front.
  Millisecond precision is deliberate: two notes saved within one second would otherwise be
  impossible to order.
- **Deletion is a column, not a `DELETE`.** `notes.deleted_at` (the `trash_and_attachments`
  migration) is `NULL` for a live note, which lets the index be partial and every read filter
  on `deleted_at IS NULL`. See [The trash](#the-trash-and-undoing-a-deletion) for what that
  costs and buys.
- **Attachment bytes are not in the database.** The `attachments` table holds a record; the
  file sits in `app_data_dir()/attachments/`. A base growing by 10 MB per screenshot would
  make every note read slower, for data no query ever looks inside.
- **`updated_at` is not touched by operations the user did not aim at a note.** Deleting a
  space moves its notes, a global retag rewrites their tags, restoring pulls one back out of
  the trash, filling a `{{field}}` records a value — none of the four refreshes it. The canvas
  sorts on that column, and touching it would float notes nobody reopened to the top. It is
  also why the field values are a command of their own rather than a `NotePatch` field: the
  patch path exists to refresh that column.

## Cross-cutting services

- **`ClockService`** (`core/services/time/`) exposes `now` as a signal ticking every 30 s. Relative
  time computed with `new Date()` inside a `computed()` freezes: the computed depends on no
  signal representing time, so it never re-evaluates and a card shows "4 min ago" forever.
  Injecting `now()` makes those computeds both pure and self-refreshing.
- **`PreferencesService`** (`core/services/preferences/`) stores UI preferences in a real file through
  `tauri-plugin-store` (`preferences.json` in `app_data_dir()`, next to the database — the plugin
  resolves against `BaseDirectory::AppData`; ⚠️ that is indistinguishable from `app_config_dir()`
  on Windows, where both are `%APPDATA%\<identifier>`), readable from Rust and
  immune to a WebView cache wipe — unlike the `localStorage` it replaced. Two consumers:
  `LocaleService`, and the editor overlay's two display toggles — fullscreen
  (`devnotes.editorFullscreen`) and the fields drawer (`devnotes.editorFieldsPanel`, open by
  default: a drawer folded on first sight hides the feature from whoever does not know it yet).
  - **The API stays synchronous** although the plugin's is not: both consumers read at
    construction time, and an async read would show the interface in one state then the
    other. The file is loaded **once** by `hydrate()` from an app initializer, into an
    in-memory cache; writes hit the cache immediately and are pushed without being awaited.
  - The plugin is reached through the `PREFERENCES_STORE_LOADER` token rather than by calling
    `load` directly. Beyond the usual seam argument, it is a practical necessity: the Angular
    builder bundles modules before Vitest sees them, so `vi.mock` on an external package
    intercepts only intermittently. Outside Tauri the loader rejects and the service degrades
    to a memory-only cache, which is how every other spec runs.
  - `hydrate()` adopts any `devnotes.*` key left in `localStorage` by an earlier version, then
    clears it. Without that, updating the app would silently reset the interface language.
  - Adding a plugin also means declaring its permission (`store:default`) in
    `src-tauri/capabilities/default.json`, or the call is refused at runtime.
- **`SettingsStore`** (`core/services/settings/`) is the application's own settings, on top of
  `PreferencesService`. It writes as it is read — there is no draft to validate — and it talks
  to nobody: `GlobalShortcutsService`, `WindowBehaviorService` and `AutostartService` read its
  signals and carry each change to the native side. See _Preferences_.
- **`ErrorNotifier` + `AppErrorHandler`** (`core/services/errors/`) surface failures on screen through
  `ErrorBannerComponent`. On a desktop app the console is not an interface: an uncaught
  exception or a failed write has to be visible, or the app just looks unresponsive.
- **`ClipboardService`** (`core/services/clipboard/`) is the system clipboard. The CSP locks the WebView
  to `'self'` and `navigator.clipboard` is unusable there, so everything goes through
  `tauri-plugin-clipboard-manager` (permissions `clipboard-manager:allow-read-text` and
  `allow-write-text`). Same `CLIPBOARD_ADAPTER` token and same degradation as
  `PreferencesService`: outside Tauri the plugin rejects, and the service reports a `false`
  rather than throwing — a copy that failed only has a visual acknowledgement to withhold.
  It is in `core/` by the usual test: a hashing tool would inject it verbatim.
- **`FileDialogService`** (`core/services/dialogs/`) is the native file picker, behind
  `tauri-plugin-dialog` (permissions `dialog:allow-open` and `dialog:allow-save`). Same
  `FILE_DIALOG_ADAPTER` token and same degradation as above, with one addition: `null` covers
  both a cancelled dialog **and** an unavailable plugin. An exception would force every caller
  to tell two non-choices apart, and there is nothing to open either way. It also flattens the
  plugin's `string | string[]` union, which stays a union even with `multiple: false`.
- **`AppWindowService`** (`core/services/window/`) hides the window and quits the app
  (`core:window:allow-hide`, `process:allow-exit`). The two are and stay distinct: the window's
  close button **hides** (`lib.rs` intercepts `CloseRequested` while there is a tray), the
  palette hides after copying, and `quit()` is the only path that really ends the process. The
  adapter is substituted in **every** spec — a real `exit()` would take the test runner down
  with the application.

## i18n

UI strings live in `src/app/core/i18n/translations/{fr,en}.json` and render through
Transloco's `transloco` pipe. French is the fallback locale.

- Translations are `import`ed and bundled at build time rather than fetched over HTTP — a
  small desktop binary with two locales gains nothing from `HttpClient` and a round-trip.
  They deliberately sit **outside** `src/assets`, where the assets glob would copy them into
  `dist` a second time, never to be read.
- **The choice belongs to `SettingsStore`** (`locale`: `system` / `fr` / `en`, default
  `system`), exactly like the theme. `LocaleService` is what _resolves_ it and pushes it to
  Transloco and `<html lang>` — the same shape as the three services that carry a preference
  down to the native side. Two controls write that one setting: the preferences panel and the
  `FR` / `EN` buttons in the titlebar, which set an explicit language.
- `system` resolves through `resolveSystemLocale()` (`core/services/i18n/locale.model.ts`), which reads
  `navigator.languages` — the WebView takes it from the OS — and falls back to
  `SYSTEM_FALLBACK_LOCALE` (English) when the machine speaks neither language. Nothing is
  persisted while the preference stays on `system`, so DevNotes keeps following the OS.
- ⚠️ `DEFAULT_LOCALE` is a different thing: Transloco's fallback _bundle_, the file that
  answers when a key is missing from the other one.
- `LocaleService.restore()` runs from the app initializer, **after** `SettingsStore.restore()`
  and before `TrayService.start()`, which builds the tray with translated labels. The effect
  alone would only flush after the first render, showing the interface in one language then
  the other.
- Code that produces user-visible text returns a **`TranslationRef`** (`{ key, params }`)
  instead of a formatted string, so translation always happens in the template. This applies
  to error messages too.
- **The application's name is never written in a translation.** A string carries `{{app}}`, and
  `AppTranslocoLoader` adds one key to every language: `app: APP_INFO.name`. No interceptor and
  no transpiler — Transloco's own transpiler resolves an interpolation it cannot find in the
  params against a **sibling key of the same translation**, so a key is all the mechanism
  needed. It costs one line in the loader, no call site passes a parameter, and a string that
  carries parameters of its own (`{{palette}}`, `{{version}}`) still resolves both. A spec on
  the shipped locale files refuses a string that spells the name out.
- A new string means adding it to **both** locale files.
- Nothing user-visible is hard-coded in the Rust back-end. A new note is created with an
  empty title and source, and the UI renders translated placeholders — storing
  "Nouvelle note" would freeze French into the data.

## Accessibility

Treated as part of the definition of done, and partly enforced by
`angular.configs.templateAccessibility` in the ESLint config.

- **Decorative pictograms carry `aria-hidden="true"`.** The app uses emoji as icons; unmuted,
  each one is announced ("pushpin", "hourglass").
- **An icon-only control draws an `<app-icon>`, not a glyph** (`shared/icon/`: a handful of
  Lucide paths, ISC, stroked in `currentColor` at 16 px). `🗑` is drawn by the system's
  colour font and looks different on every machine, `▤` barely renders, neither follows
  the theme's `color` — and `⚙` opened the **filters**, the one thing a gear is never for
  (#322). The icon is `aria-hidden` by construction: the control names itself, through
  `aria-label` and `title`. A glyph beside a word, as in the editor's toolbar, reads fine
  and stays.
- **Information conveyed only graphically is duplicated as text.** A pinned card renders a
  `.visually-hidden` label, because the pin itself is a CSS pseudo-element.
- **Toggles expose `aria-pressed`**, not just a CSS class: tag pills, filter chips, the pin
  button, the locale switcher. A non-interactive tag pill renders a `<span>`, not a button —
  announcing a button would advertise an action that does not exist.
- **Every modal is a real dialog**: `role="dialog"`, `aria-modal`, `aria-labelledby` and a
  focus trap that confines Tab and restores focus on close. All of it comes from
  `DialogComponent` (`shared/dialog/`), written once — an accessibility fix here used to be
  a twelve-file change. Written by hand rather than pulling in `@angular/cdk` for it.
- **The space switcher is a real menu**: `aria-expanded`, `aria-haspopup`, focus moved into
  the menu on open, arrow/Home/End navigation, Escape closing and restoring focus. Creating a
  space _replaces_ the menu with a form instead of nesting a text field inside `role="menu"`,
  which is neither valid ARIA nor navigable the same way; Escape then steps back to the menu
  before closing the dropdown.
- **Controls that wrap decorations get an explicit `aria-label`.** The search input sits
  inside a `<label>` that also holds the magnifier and the shortcut hint; without one, the
  field would be announced as "🔍 Ctrl+K".
- A `<button>` contains only phrasing content — nested `<div>` is invalid HTML with
  undefined accessibility behaviour.

## Theming

All colors, fonts and shadows are CSS custom properties defined on `:root` in the global
`src/styles/styles.scss`; components only consume them via `var(…)`.

Those variables **must** stay in the global stylesheet. Angular's emulated encapsulation
rewrites a `:root` selector written inside a `*.component.scss` into a form that never
matches `<html>`, silently invalidating every variable.

Recurring style patterns (unstyled control, card surface, accent state, focus ring, tinted
badge) are SCSS mixins in `src/styles/_mixins.scss`, imported as `@use 'mixins' as *;` —
resolved via `stylePreprocessorOptions.includePaths` in `angular.json`. Colors needing
translucency are also exposed as RGB triplets (e.g. `--amber-rgb`, which is the **fill**) so
`rgba()` never hard-codes a hex value.

**Three sweeps read the shipped files**, all `node --test` rather than `*.spec.ts` because the
Angular builder compiles its specs for a browser, where `node:fs` does not exist:
`palette.test.mjs` for contrast, `focus-rings.test.mjs` for a control styled with no
`:focus-visible` — the linter cannot see a missing one, and four had accumulated — and
`language-hues.test.mjs` for a `Language` variant with no `.lang-*` rule, whose badge is then
bare text. ⚠️ Each is named in `test:scripts` one by one; Node 24 will not expand a directory.

`styles.scss` also carries the `.visually-hidden` utility and a `prefers-reduced-motion`
block.

**Light theme.** `:root` stays the dark palette and `:root[data-theme='light']` redefines the
colours only — fonts, shadows and spacing are shared. Dark stays the base on purpose: the
preference lives in a file nothing can read before Angular has booted, so any other order would
flash white at launch. `color-scheme` switches with the palette, which is what repaints the
native `<select>`s, scrollbars and autofill. The syntax-highlighting theme needs nothing: it
only ever consumed these variables.

**The accent is three variables, one per job.** `#e8a33d` on white is 2.16:1, and what that
rules out depends entirely on what it is being used for — a surface only has to carry the ink
drawn on it, a hairline has to be found against the page, and a label has to be read.

|                | job                            | bar                   | dark                | light                    |
| -------------- | ------------------------------ | --------------------- | ------------------- | ------------------------ |
| `--amber-fill` | a solid button, a tick, a tint | 3:1 for the ink on it | `#e8a33d`           | **the same declaration** |
| `--amber-edge` | a border, a focus ring         | 3:1 on the surface    | `var(--amber-fill)` | `#cf6a08`                |
| `--amber-text` | a label                        | 4.5:1 on the surface  | `var(--amber-fill)` | `#a8500f`                |

⚠️ `--amber-fill` is declared **once**, in the dark base, and the light block does not
redefine it — which is what makes the dark theme provably unmoved by the split, and what lets
the light theme show the same amber everywhere it is a surface. Only the line and the word step
down. One token doing all three meant drawing the whole accent at the _label's_ darkness, which
is where a warm hue lands at 4.5:1 on white, and which read as mustard.

`--amber-dim` is the softer hairline a hover draws, and `--amber-ink` the text laid on the
fill; with the same fill in both themes there is one of each. `scripts/palette.test.mjs` holds
each of the three to its own bar, in both themes, and fails if the fill is ever declared twice.

**Density.** `:root[data-density='compact']` tightens four variables — `--space-card`,
`--space-grid`, `--space-section`, `--space-canvas` — and nothing else. Typography is
untouched: shrinking the text would be a zoom, not a density. Four named gaps rather than a
global factor, because these four are what decide how many cards fit on screen; everywhere else
the spacing stays hard-coded, since compressing all of it would cost legibility without buying
a line.

Fonts are self-hosted through the `@fontsource` packages listed in `angular.json`'s `styles`
array. They used to come from Google Fonts, which on a desktop app meant degraded typography
offline and a CSP that could not be locked down.

## Application updates

Built on `tauri-plugin-updater`. Each release bundle is signed at build time with a minisign
key; the matching public key is compiled into the binary, so a compromised release host
cannot push a payload the app will accept — only the private key can produce an installable
update.

- **The user decides.** `check()` only produces an offer; `UpdateStore.accept()` is the only
  path that downloads. A silent update would restart the app mid-keystroke, and the editor
  only commits its drafts on blur.
- **`UpdaterService`** (`core/services/updates/`) is the seam, for the same reason the repositories are
  one: no component or store imports `@tauri-apps/plugin-updater`, which needs a Tauri bridge
  that jsdom does not have. These are plugin commands, not ours, so they never appear in
  `bindings.ts`. The service also holds the plugin's `Update` object — a **native resource**
  with a Rust-side id that must be closed if the offer is declined, hence
  `UpdaterService.discard()`.
- **A failed check is silent; a failed install is not.** Offline, behind a proxy, or on a dev
  build whose public key is still the placeholder, `check()` fails on every launch — a banner
  there would be a daily reproach about something the user cannot act on. An install failure
  follows an explicit click, so it reaches `ErrorNotifier` and leaves the prompt open for a
  retry.
- **A manual check speaks where the startup one stays quiet.** `checkNow()`, behind the
  titlebar's About menu, reports all three outcomes — including "nothing to do", which the
  silent path has no way to express. Its failures also reach `ErrorNotifier`: the user clicked
  and is owed an answer, and the banner outlives the menu, whereas the in-menu status line
  disappears with it. Both paths share one private `runCheck({ silent })`.
- **`CheckState` is deliberately separate from `UpdateStatus`.** The latter is the install
  lifecycle that `UpdatePromptComponent.busy()` reads; the former is only what the menu has
  left to announce. Its `idle` covers both "not checked yet" and "found something" — in the
  second case the prompt is doing the talking.
- **"Later" can be made to stick, and what is written down is a version.** The prompt
  carries a checkbox, because the decision belongs where the interruption happens.
  `AppSettings.skippedUpdate` holds the version that was set aside — storing "the user said
  no" would silence the release after it too, and a newer one is a new offer that needs no
  gesture to become one again. `AppSettings.updateNotifications` is the blunter switch next
  to it, and the preferences panel names the silenced version with a way to take it back,
  since waiting for the next release is not one.
  ⚠️ Only the **silent** startup check is silenced. `checkNow()` is a question asked out
  loud and always answers, which is also what stops the feature from being a trap.
- **The dot is a third thing, next to `UpdateStatus` and `CheckState`.** `hasPendingUpdate`
  is "an update exists and is not installed", and the About menu wears it. It cannot read
  `status()`: `dismiss()` returns that to `idle` — which is what closes the prompt — so a dot
  driven by it would vanish with the dialog it exists to outlive. For the same reason
  `dismiss()` leaves `_update` standing and moves only the status; `offered` is the computed
  the prompt reads, and it is `null` exactly when the status is `idle`, so the dialog stays
  up through the install it would otherwise disappear from.
- **The download does not cross the CSP.** It runs in Rust through the plugin's HTTP client,
  not in the WebView, so pointing `endpoints` at GitHub needs no widening of `connect-src`.
- `bundle.createUpdaterArtifacts` makes the bundler emit a `.sig` beside **every** bundle it
  produces, `.deb` and `.rpm` included — but the updater can only install the NSIS installer
  and the AppImage. System packages are updated by their package manager, by design, so the
  manifest step ignores their signatures instead of choking on them.
- The manifest (`latest.json`) is assembled by the `publish` job from those `.sig` files
  rather than by `tauri-action`, which only writes one when it creates the release itself —
  something the build matrix deliberately avoids. Windows is the one platform whose absence
  fails the job; a missing Linux artifact only logs a warning, so a Linux bundling problem
  cannot hold back an otherwise sound Windows release. The release is published straight
  away, which is what keeps `releases/latest/download/latest.json` — the endpoint compiled
  into the binary — resolvable.
- Building a bundle now requires `TAURI_SIGNING_PRIVATE_KEY` (and its password) in the
  environment. Without it `tauri build` fails, instead of shipping binaries the updater would
  later refuse. Both secrets live in the `release` **environment** rather than in the
  repository, so no other workflow and no other branch can reach them.

## Releasing

One button: **Actions → Release → Run workflow**, with a `dry_run` checkbox. `release.yml`
carries the whole path — `prepare`, `release-build`, `publish` — and `ci.yml` is only CI.

**Two platforms, deliberately: Windows and Linux.** The release matrix builds
`windows-latest` and `ubuntu-22.04`, the CI matrix tests the same two, and that is the whole
list. macOS is out of scope and stays out: nothing here can test it, and Gatekeeper wants a
paid Apple Developer account with no free bypass on recent versions — a build that can be
neither tested nor distributed is a liability wearing the costume of a feature. The bundle
therefore declares no `.icns`, and the README says which platforms it ships. ⚠️ One macOS
name survives and is meant to: `tauri_plugin_autostart::init` takes a `MacosLauncher` on
every platform and ignores it off macOS, so it is a required argument, not a dead branch.

Two facts shape that, and neither is negotiable:

- **`CHANGELOG.md` is baked in at compile time.** `include_str!` reads it when the crate is
  built, so the section for the release being cut has to be written, committed **and tagged
  before** `release-build` starts. A workflow that generated notes after the tag would ship a
  binary whose "Nouveautés" panel does not know its own version.
- **A tag pushed with the default `GITHUB_TOKEN` triggers no workflow.** GitHub's anti-loop
  guard means the tag `prepare` creates cannot wake a second run, so the build and the
  publication have to live in the same one. That is the whole reason the release jobs moved
  out of `ci.yml` — not a taste for tidiness.

The same property has a price worth stating: **the changelog commit is never seen by the CI**.
`prepare` is therefore its own gate — it runs `npm run test:scripts` and `prettier --check` on
the file it just wrote, and `scripts/release-notes.mjs` mirrors, in `assertRenderable`, the
three invariants `changelog::tests::the_shipped_file_parses_into_something_to_show` asserts.

What `prepare` does **not** do is rerun the suites. It checks that `ci.yml` concluded `success`
on the very commit being released and stops if it did not; the work was already done, and a
release that replayed it would pay for it twice.

- **The notes come from the merged pull requests.** `scripts/release-notes.mjs` reads the
  squash subjects between the last tag and `HEAD`, pulls the `(#NN)` out of each, then asks
  GraphQL for the title, the labels **and the labels of the issues the pull request closes**
  (`closingIssuesReferences`, which REST does not expose). GitHub propagates nothing from an
  issue to its pull request, and in this repository the issues are the labelled half — hence
  the chain: label on the pull request, then label on the closed ticket, then the `(feat)` /
  `(fix)` prefix of the title, then "Under the hood". Nothing is dropped for want of a class,
  and the dry run prints which link of the chain filed each entry.
- **Asked by number, never listed.** Several pull requests here were merged into intermediate
  branches and never landed as a commit on `main`; any listing by date reports work the
  release does not carry.
- **The body of the GitHub release is re-extracted from the committed file** at the tag, rather
  than passed along as a job output. The file baked into the binary and the page on github.com
  then cannot say two different things.
- **Restricted to `main` by an environment, not by an `if`.** The branch selector of "Run
  workflow" cannot be filtered, so the three jobs declare `environment: release`, whose
  deployment branch policy refuses to start them from any other ref. The `if` on the first
  step stays as a readable fallback.
- **`prepare` is deliberately not idempotent.** After a failure, use "Re-run failed jobs":
  successful jobs are not replayed and their outputs survive. "Re-run all jobs" will stop on
  the "tag already exists" guard — safe, but surprising.

## Tauri configuration

- `src-tauri/tauri.conf.json` wires the pipeline to Angular: `beforeDevCommand` /
  `beforeBuildCommand` run the npm scripts, `devUrl` must match the Angular dev server port
  (1420, fixed in `angular.json`), and `frontendDist` must match Angular's build output path.
- `src-tauri/capabilities/default.json` is the v2 permission manifest for the main window.
  Any new plugin or restricted API needs its permission listed there, or the call is denied
  at runtime — that is where `updater:default` and `process:allow-restart` come from.
- **`opener:allow-open-url` carries a scope**, not the bare permission: only
  `https://github.com/vmillet-dev/*` may be opened. `opener:default` would let any URL through
  the WebView's only escape hatch to the system browser. The About dialog needs the plugin
  precisely because the CSP is locked to `'self'` — a plain `<a href>` leads nowhere — and
  `AppInfoService` (`core/services/app-info/`) is its seam, alongside `getVersion()`. That one needs no
  permission of its own: `core:app:allow-version` already ships inside `core:default`.
- `serde_json` is a **runtime** dependency, not just a dev one: `generate_context!` embeds the
  `plugins` section of `tauri.conf.json` as JSON, and drops the section without it.
- **CSP is enabled.** `csp` locks production down to same-origin resources; `devCsp`
  additionally allows the dev server's websocket and inline scripts for hot reload. Both
  keep `ipc:` and `http://ipc.localhost` in `connect-src` — without them `invoke()` is
  blocked. Loading anything remote means widening these, deliberately.

## Testing

### Benchmarks

`cargo bench` from `src-tauri/`. They are **not in CI**: a timing assertion on a shared
runner flaps, and a suite that flaps is a suite everyone learns to ignore. They run
locally, on demand.

```bash
cargo bench -- --save-baseline main   # before
cargo bench -- --baseline main        # after
```

That comparison is the whole reason the harness is **criterion** and not divan, which is
lighter and pleasanter but does not offer it out of the box. ⚠️ The baseline it writes lives
in `src-tauri/target/criterion/`, which is gitignored: it is local to one machine and dies
with `cargo clean`. The table below is the durable record.

⚠️ **They run below the command boundary, not through Tauri.** A command is four lines —
validate, lock, delegate, translate the error — so `store::*` plus `view::*` plus the serde
round-trip captures nearly all of the cost. `tauri::test::mock_app` would drag the whole app
lifecycle in and buy only the IPC transport, which this codebase does not control. So these
numbers are **not** "the IPC is fast": they are what the work behind a command costs.
Serialisation is included on purpose — a `NotesView` over 8000 notes is a real `serde_json`
cost paid on every keystroke.

⚠️ **The corpus is file-backed, never `open_in_memory`.** An in-memory database has no pager
behind a file, no page cache doing real work and no I/O at all — it measures something the
application never does. `benches/corpus.rs` writes 8000 notes of ~13 kB into a temporary file
database — about 104 MB, seeded once per group — and its bodies are accented on purpose: a
pure-ASCII corpus would exercise only `fold`'s fast path.

⚠️ **`Corpus` does not implement `Drop`; its `TempDir` field does, and is declared last.**
Fields drop in declaration order but a `Drop` on the struct runs before all of them, so the
erasure fired while SQLite still held the file open — which Windows refuses to delete over,
and `let _ =` swallowed the error. Six groups leaked a corpus each, 600 MB a run.

Two Cargo details exist solely to make this work, and they are documented here rather than in
`Cargo.toml`: `autobenches = false` (or the shared corpus module is discovered as a benchmark
of its own and reported as entirely unused) and `bench = false` on the lib and both bins (or
cargo runs their built-in harness first, which rejects criterion's own flags).

#### The baseline

8000 notes, Windows, release profile with `lto = true`. The last column is the same benchmark
against the 800-note corpus this suite started on, which is what says whether a cost is linear
in the corpus or worse.

| Command                                        | Cost            | 800 → 8000    |
| ---------------------------------------------- | --------------- | ------------- |
| `query_notes`, search matching nothing         | 406 ms          | ×15.1         |
| `query_notes`, unfiltered                      | **403 ms**      | ×14.7         |
| `query_notes`, search folding accents          | 392 ms          | ×15.2         |
| `export_notes` / `import_notes`                | 377 ms / 98 ms  | ×14.1 / ×11.8 |
| `list_tags`                                    | 37.9 ms         | **×64**       |
| `delete_notes` then `restore_notes`, 100 notes | 10.4 ms         | ×1.2          |
| `rename_tag` across the corpus                 | 8.2 ms          | ×2.2          |
| `move_notes` / `tag_notes`, 100 notes          | 5.1 ms / 4.2 ms | ×1.9 / ×1.8   |
| `list_trash`                                   | 4.5 ms          | ×10.5         |
| `update_note`                                  | 1.5 ms          | ×1.0          |
| `list_global_placeholders`                     | 1.5 µs          | ×1.0          |

One row does not belong to that shape and is measured for a different reason:

| Bench                                 | Cost        | Corpus |
| ------------------------------------- | ----------- | ------ |
| `Vault::derive` at the shipped `Cost` | **52.6 ms** | none   |

⚠️ It seeds no corpus — deriving a key touches no database — and it is here rather than in a
comment because **that number had been taken in a debug build**, where Argon2 is unoptimised:
`vault::key::Cost::default` claimed ~1.2 s, twenty times what the shipped binary pays, and
every argument about the margin rested on it. Criterion builds in release, so a benchmark
cannot be read off the wrong profile by accident, which a hand-run measurement could. The
parameters were left where they are: 64 MiB is three times OWASP's floor and it is the memory,
not the time, that bounds an attacker with a GPU.

Three things worth reading off the first table.

**`query_notes` has gone past the debounce.** It runs on every keystroke behind a 150 ms
debounce, and at 800 notes its 27 ms sat comfortably inside it. At 8000 it costs 403 ms: the
query fired for one keystroke is still running when the third one after it arrives. That is
what #21 is about, and this is the number that says the problem has stopped being theoretical.

**The search is still not what costs.** Folding accents is the _cheapest_ of the three
variants (392 ms against 403 ms unfiltered) — fewer notes survive to be serialised. Fetching
8000 × 13 kB out of SQLite and turning the view into JSON is the whole cost, and work aimed at
making the match faster would be aimed at the wrong half.

**`list_tags` is the one that degrades faster than the corpus.** ×64 for ×10 the data, the only
entry on the table that is markedly super-linear, and invisible at 800 notes where it cost
591 µs. It joins `note_tags` to `notes` to read one nullable column, so each of the 16 000 tag
rows dereferences a ~13 kB note row; the notes table was 10 MB at 800 notes and is ~104 MB at 8000. ⚠️ That is the likely cause and it is **not confirmed** — no query plan was taken.

Unit tests run with Vitest through the `@angular/build:unit-test` builder in a jsdom
environment (configured in `angular.json`'s `test` target and `vitest-base.config.ts`), so
no browser is needed. Specs sit next to the file they cover. Coverage thresholds are set at
80% and enforced by `npm run test:coverage`.

Test descriptions and comments are written in **English**, like the rest of the repository.

Shared helpers live in `src/testing/`, which is outside the `**/*.spec.ts` include and so
never collected as tests: `Note` and `NoteSection` fixture builders, in-memory repository
doubles, and `provideAppTesting()` — one call providing both repositories and Transloco, so
a new data seam does not have to be added to a dozen spec files by hand.

`createNotesHarness()` builds the three canvas stores together onto one fake repository,
because they _are_ one object graph: the selection reads what the query holds, and a write
reloads it. Testing any of them against fakes of the other two would test the fakes.

`FakeNotesRepository` and `FakeSpacesRepository` behave like real persistence (they own the
list and assign ids and timestamps) and expose `failNext`, which is what makes the stores'
failure paths testable at all. `FakeAttachmentsRepository` and `FakeTransferRepository` follow
the same pattern; `FakeFileDialog` and `FakeAppWindow` stand in for the two native services.

`FakeNotesRepository`'s deletion is a **soft** one, like the real back-end's: a deleted note
moves to its trash list, which is what makes undo and the trash panel observable at all.
`FakeAppWindow` is provided in **every** spec, not only those that need it: a real `exit()`
would take the test runner down with the application.

`FakeNotesRepository` deliberately **does not** reimplement filtering, grouping or tag
normalisation: those live in Rust and are tested there. Duplicating them in the double would
let a front-end spec pass against rules the real back-end does not apply. It wraps its notes
in a trivial single-section view, and a spec needing a specific shape (search results, empty
results, several sections) pins one with `setView`. `lastQuery` and `queryCount` expose what
the store asked for — which is the part of querying the front-end still owns.

That split also decides where a test belongs: assertions about _what is shown_ (which notes
match, which section they land in, how tags are cleaned) go in `src-tauri/`, while the
front-end specs cover assembling the query, pacing it, and reacting to what comes back.

Component specs follow one consistent pattern:

- Render the component with its **real** children — they're standalone and already declared
  in the component's own `imports`, so no extra wiring is needed. Don't stub children or
  mock Angular's DI.
- Assert against a child's public contract only: read its input signals, and call or
  subscribe to its output emitters. Don't reach into a child's rendered DOM — that child's
  behavior is covered by its own spec. Note that a `model()` exposes its change output
  through the signal itself, not as a separate `xChange` property.
- Do `TestBed.createComponent(...)` and set required inputs in `beforeEach`, not in a
  per-test helper, so every spec shares one setup path.
- Call `fixture.autoDetectChanges()` once in `beforeEach` (it also performs the initial
  render), then `await fixture.whenStable()` after any state change before asserting on the
  DOM. This lets Angular's own scheduler decide when to re-render, as it would in
  production, instead of forcing synchronous checks.
- For date-dependent output (relative time, expiry), use
  `vi.useFakeTimers({ toFake: ['Date'] })` with `vi.setSystemTime(...)`; for the search
  debounce, `toFake: ['setTimeout', 'clearTimeout']`. **Never** call `vi.useFakeTimers()`
  without a `toFake` list here: it also fakes `requestAnimationFrame`, which the zoneless
  scheduler relies on, and `await fixture.whenStable()` will hang forever.
  `clock.service.spec.ts` is the one exception — it asserts on the interval itself.
- Anything asserting on `document.activeElement` must attach `fixture.nativeElement` to the
  document; jsdom does not track focus for detached elements.
- Assert on text through a whitespace-normalising helper. Two templates deliberately keep
  their interpolations on a single line (and carry a `prettier-ignore`) because Angular does
  not fully collapse the whitespace a line break would introduce.
- For anything that transitively needs a store, use `provideAppTesting()` and spy on the real
  store's methods rather than re-implementing a fake store — stores have their own specs.

### Rust

`cargo test` from `src-tauri/` runs everything. `cargo clippy -- -D warnings` and
`cargo fmt --check` gate the code; `Cargo.toml` sets `unsafe_code = "forbid"` and
`deny(clippy::all)`.

Unit tests are inline `#[cfg(test)] mod tests` blocks at the bottom of the file they cover —
the idiomatic Rust form, and the one that keeps a test next to what it asserts. Shared
fixtures for the notes tests live in `notes::fixtures`, a `#[cfg(test)]` module in `notes.rs`.
`src-tauri/tests/` holds the integration binaries, which see only the crate's public API.

The tests split by what they need in order to run:

- **Model, view and language** — pure, no database, milliseconds to run. Section placement and
  exhaustiveness, local-midnight boundaries, absurd timezone offsets, tag normalisation,
  Unicode search folding, footer choice, expiry thresholds, the refusal of an unreadable
  `now`, and the JSON wire shape.
- **The stores** — against `open_in_memory()`, which applies the **real** migrations, so the
  tests exercise the actual schema, constraints and cascades rather than a stand-in. They pass
  timestamps explicitly instead of reading the clock, which makes assertions on `created_at` /
  `updated_at` deterministic. A `query` helper in the test module recomposes
  `fetch` + `notes::view::build` so the whole read path stays covered end to end.
- **`db` and `error`** — that a poisoned mutex reports `storageUnavailable` instead of
  panicking a second time, and that each error variant maps to the right code and params.

Test names and comments are in English, like the front-end specs. `notes::store::list`
survives only as a `#[cfg(test)]` helper — no command returns a raw list.

### End to end

`npm run test:e2e` drives the **assembled application**: the real binary, a real WebView, the
real IPC bridge and a real SQLite file. It is the only suite that can see a missing capability,
a command registered but unreachable, an argument renamed on one side only, a CSP that blocks
what it should not, or a front end that fails to boot at all — none of which is visible to
jsdom or to `open_in_memory()`.

The runner is WebdriverIO with `@wdio/tauri-service`, configured in `e2e/wdio.conf.ts`. Specs
live in `e2e/specs/`, one file per scenario; every selector is held in `e2e/pageobjects/`, one
file per area of the screen as `src/app/` draws it (`sidebar`, `header`, `canvas`, `board` and
`board-gestures`, `editor`, `overlays`, `titlebar`), and `e2e/support/` holds the bridge helper,
the application helpers and the profile paths.

**Nothing of the harness ships.** Three seams keep it out of the release binary, and each one
is checkable:

- The `e2e` Cargo feature, off by default, gates the two `tauri-plugin-wdio*` plugins.
- `src-tauri/tauri.e2e.conf.json`, merged at build time by `npm run e2e:build`, adds
  `withGlobalTauri`, declares the `wdio` capability **inline** so no file under
  `capabilities/` can leak into a release, and changes the identifier.
- The `e2e` Angular configuration in `angular.json`, whose only difference is
  `"polyfills": ["@wdio/tauri-plugin"]` — no production source file mentions the harness.

⚠️ The Rust half and the npm half go **together**. With the crates but no polyfill the runner
never sees `window.wdioTauri` and hangs before opening a session; with the polyfill but no
crates the front end invokes `plugin:wdio|…` commands nothing answers, and the error banner
comes up on launch.

#### One application, fifteen spec files

**⚠️ `driverProvider: 'embedded'`, and that decides the shape of every scenario.** The
WebDriver server lives _inside_ the application, reached through
`tauri-plugin-wdio-webdriver`, so there is no `tauri-driver`, no msedgedriver and no Chrome
DevTools protocol. That is what made the suite runnable on CI at all: the external chain drives
the WebView through msedgedriver, which launches the binary expecting Chromium's handshake and
gives up on `DevToolsActivePort file doesn't exist`.

The cost is that the provider spawns the application **once**, from its own `onPrepare`, and
never again. Reading `@wdio/tauri-service`: `startEmbeddedDriver` is called there and from
`restartEmbeddedServer`, which only fires when a health check finds the server already dead;
`onWorkerStart` — the per-spec-file hook — does nothing else. Raising `maxInstances` would not
change it either, because the service skips its per-worker spawn for this provider
(`if (this.perWorkerMode && !instanceId && !this.isEmbeddedMode)`).

So **every spec file shares one process, one SQLite file and one `preferences.json`**. Three
consequences, and they are rules rather than observations:

- **The profile is wiped once per run, before the runner starts** — `npm run test:e2e` is
  `tsx e2e/reset-profile.ts && wdio run …`. Not from a hook: nothing orders a wdio hook against
  the service's own `onPrepare`, and a wipe from inside meets a living process, a locked
  database and an open WAL. A separate process beforehand has no ordering to get wrong.
  ⚠️ The profile is **two** directories, not one: `tauri-plugin-window-state` writes
  `.window-state.json` under `app_config_dir()` while everything else the application
  writes lives under `app_data_dir()`. Windows cannot tell them apart, so wiping only the
  data directory passed there and left the window geometry behind on Linux — where a run
  then opened on the window the previous one closed with.
- **`before()` buys each file a fresh front end and nothing more.** `browser.refresh()` reboots
  Angular and every store over the same database; it resets no data.
- **A spec file establishes its own preconditions.** It seeds what it needs, and it does not
  assume a clean corpus. `05-spaces` deletes the spaces an earlier file left before asserting
  on "the only space there is", and puts the canvas back on "all spaces" in its `after`.

The numeric prefix on each file is therefore load-bearing: it is the run order.
`01-first-launch` is the only file that meets a virgin profile, which is why it is also the one
that resolves the seeded space — `homeSpaceId()` records it while exactly one exists and writes
it to a marker file, because WebdriverIO gives each spec file its own worker process and a
module-level cache would be empty again in the next one. ⚠️ It cannot be `listSpaces()[0]`:
`list_spaces` orders pinned first and then by `name COLLATE NOCASE`, so after another file
creates `Ops` — or pins anything — the first row is no longer the seeded space.

**⚠️ There is no restart, and no spec may claim one.** `reopenSession()` is a
`browser.reloadSession()`: it tears the session down and opens a new one against the same
living process — which has to stay up, since it _is_ the server. The Rust side, its SQLite
connection and `tauri-plugin-store`'s in-memory map all survive. What it proves is that the
interface was rebuilt from what the commands answer rather than from a signal it was still
holding; what it cannot prove is that anything reached the disk. For preferences specifically
the distinction matters: the store plugin caches in the Rust process and flushes on a 300 ms
debounce, so a reloaded page reads the map, not the file. `15-preferences-on-disk` reads
`preferences.json` from Node for that, which is outside the application entirely.

**Seeding goes through the bridge.** `e2e/support/bridge.ts` calls the real commands with the
types generated in `bindings.ts`, so a Rust signature that moves stops the harness compiling.
It goes through `window.__TAURI__` rather than `browser.tauri.execute`, which the service
resolves through an HTTP endpoint it loses after a `reloadSession`. Writing the SQLite file
directly from Node would bypass the migrations and the model rules, and would let a test pass
against a state the application cannot produce.

#### Driving the interface

**Selectors are `data-testid`, and page objects own them.** Every `aria-label` and every
visible string goes through `transloco`, and the default locale follows the machine — a text
selector would depend on the runner. Repeated elements carry the identifying value beside the
hook (`data-testid="note-card" data-note-id="…"`), because picking a card by position is the
brittleness the attribute exists to remove. The preference controls are the exception: they are
addressed by the `id` their own `<label for>` needs, which cannot be renamed without breaking
the association.

Three things the embedded WebDriver server will not do, each with a helper in `support/app.ts`:

- **Keyboard.** It answers `POST /session/:id/actions` with a 200 and dispatches nothing, so
  `browser.keys` silently did nothing at all. `press()` dispatches a synthetic `KeyboardEvent`,
  which reaches the same handlers — they all listen in the DOM. It fills `code` as carefully as
  `key`, because the shortcut field reads the physical key.
- **`<select>` and `<input type="date">`.** `selectByAttribute` moves the selection without the
  `change` the components listen to, so the model kept the old value while the control showed
  the new one. `setNativeValue()` assigns and dispatches. A date input is the same helper for a
  different reason: typed keystrokes go in the _display_ format, which follows the WebView's
  locale.
- **Implicit form submission.** Enter in a text input submits its form natively, and the
  browser reserves that for real user input. `submitFormOf()` calls `requestSubmit()`.

**What the suite deliberately does not cover**, because a WebView cannot reach it. In each
case the control is asserted on — it exists, it is labelled — and never clicked:

- The OS-level global accelerator. WebDriver types into the WebView, not into the machine, so
  the palette is opened by emitting the same `devnotes:action` event the accelerator sends.
- The native file picker. `window.__TAURI_INTERNALS__.invoke` — the funnel every `invoke` goes
  through — is `writable: false, configurable: false`, so nothing can stand in front of it and
  a picker opened by a click would block the application until a human clicked it. Import,
  export and attaching a file are exercised through their commands, which take a path.
- Handing a file to the desktop (`open_attachment`), and `Quit` — one launches whatever the
  runner has registered, the other takes the application down mid-run.
- The tray menu, which is native.
- The pointer-drag handle on a checklist item. `Alt+↑/↓` is its keyboard twin, it has to work
  anyway for the linter, and it is the testable one.
- The system clipboard where the machine will not release it: on Windows a clipboard manager
  can hold the lock indefinitely, and a headless Linux runner may have no selection owner at
  all. `clipboardText()` answers `null` and the scenario calls `this.skip()` — skipped rather
  than passed, because a bare `return` is a green test that asserted nothing. What DevNotes owns
  is asserted anyway, through `DisplayNote.copyText`.

#### In CI

The suite is a job of its own, on a matrix of `windows-latest` and `ubuntu-22.04`, and it
**blocks**. It did not always: `continue-on-error: true` stood on it while it earned its
keep, on the reasoning that a flaky E2E job everybody ignores is worse than no job. That
reasoning turned out to have a worse failure mode than the one it was avoiding. On `main`
at `e4ed5d9` the Ubuntu leg failed on a real bug — a note keeping its title and losing the
body typed after it — and the run was reported `success` regardless: `gh run list` said
success, the badge said passing, and the failure sat there across three merges until it was
looked for by hand. ⚠️ A job whose failure reads as a pass is not a weak signal, it is a
false one. The flag is gone, and the price it was paying — a flaky run blocking a merge
until it is re-run — is the one worth paying.

The two platforms do not break the same way, and **Linux is the one that
can tell paths apart**: the WebView is WebView2 on one and WebKitGTK on the other, and
`dirs::data_dir()` and `dirs::config_dir()` are the same `%APPDATA%\<identifier>` on Windows
but `~/.local/share` against `~/.config` on Linux.

That is not theoretical. `preferences.json` goes to `app_data_dir()` — `tauri-plugin-store`
resolves against `BaseDirectory::AppData` — and both the service's own comment and the first
version of `15-preferences-on-disk` said `app_config_dir()`. Windows agreed with the mistake
because the two resolve to one folder there; the Linux job is what produced a
`no preferences file at /home/runner/.config/…` and settled it.

On Linux the runner is wrapped in `xvfb-run`: WebKitGTK needs an X server, and it is the wdio
service that spawns the application, so the `DISPLAY` has to exist for the whole process rather
than for one command. `logLevel` defaults to `warn`; a session that refuses to open is
diagnosed by re-running the job with `E2E_LOG_LEVEL=trace`.
