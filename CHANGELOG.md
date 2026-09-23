# Changelog

Notable changes to DevNotes, newest first.

**The newest section is generated.** `.github/workflows/release.yml` writes it from the pull
requests merged since the last tag, then commits it **before** creating the tag — this file is
shipped inside the binary (`src-tauri/src/changelog.rs` embeds it with `include_str!`) and read
by "À propos → Nouveautés", so a release whose section came after the tag would ship a binary
missing its own entry. Sections already written are never touched: edit them by hand freely.

Keep the shape — a `## ` heading per release, `### ` headings for the categories, one `- `
bullet per entry — or the entry will not be rendered. It is deliberately untranslated, like
the release notes the updater hands over.

## [0.4.1] - 2026-09-23

### ✨ Added

- Name the library on the gate, and offer the others there (#327)

### 🔧 Changed

- Go back to a version rather than adding one, and show it first (#335)
- Toggle the titlebar's theme between light and dark (#334)
- Draw the header's icons as paths, and the filters as a funnel (#333)
- Give the editor's space and folder a band of their own (#332)

### 🐛 Fixed

- Tint the board's bands with a colour the browser accepts (#331)
- Swallow the context menu wherever a right-button sweep ends (#330)
- Stretch the board surface to the whole of the dotted ground (#328)
- Reload the front end when switching libraries (#326)

## [0.4.0] - 2026-09-22

### ✨ Added

- Hold several libraries, with a registry beside them (#315)
- Keep the body an edit replaced, so a snippet that worked comes back (#314)
- Give a forgotten passphrase a way out of the gate (#313)
- Show the backup copies, and let one be put back (#312)
- Apply the preferences on a button, not as they are typed (#311)
- Split the preferences into three pages, and make the keys movable (#310)
- Turn the guide into a walk with a picture at each step (#309)
- Put the release's notes in the update prompt, and show them (#307)
- Score the language instead of ordering the guesses (#308)
- Put the theme beside the language, where it belongs (#306)
- Let the interface move, in two durations and one off switch (#303)
- Make the card's menu complete, and let the editor move a note (#302)
- Give the card's first line back to the note (#301)
- Sweep a selection band with the right button (#300)
- Tick a whole folder from the menu that already knows what it holds (#299)
- Split the board's tidy-up in two, and pan back to what it wrote (#298)

### 🔧 Changed

- Get the header down to one row, and the first card into the top half (#305)
- Take the operating system's chrome out of the window (#304)

## [0.3.3] - 2026-09-21

### 🔧 Changed

- Say on the desktop that the palette copied something (#292)

### 🐛 Fixed

- Remove the no-folder count rather than move it again (#296)
- Let Escape take the note back while the bar is still offering (#294)
- Make Tab an arrow in the palette, and hold the focus in the field (#288)
- Count the columns a zone actually flows (#291)
- Stage which folder a drop put a card in, not only where (#290)
- Open a palette result the canvas is not holding (#289)
- Say on the card how to call an armed deletion off (#287)
- Leave the focus ring room inside the rail that scrolls (#286)

### 🧰 Under the hood

- Adding a language is five edits, and one of them no type catches (#278)

## [0.3.2] - 2026-09-21

### 🐛 Fixed

- Give every format a badge, and a sweep that says when one has none (#275)
- Split the accent into a surface, a line and a label (#271)
- Ask twice before the Delete key trashes a note (#273)
- Anchor the no-folder count to the board's corner (#270)
- Give the search field a cross that empties it, and the filters a button (#272)
- Open a zone far enough for the card filed into it (#269)
- Make the search placeholder fit, and keep the long form as a tooltip (#266)
- Read an entry's emphasis in Nouveautés instead of printing it (#268)
- Draw a note opened from the palette in front of a help panel (#267)
- Put the keyboard legend on the bottom edge (#265)
- Keep the create slot inside an opened folder (#264)
- Stamp an export name with the local day and minute (#263)
- Hold a dropped card until a view comes back carrying it (#262)
- Say where the keyboard is, on the four controls that did not (#261)
- Make the copy key copy what the card copies, and say which note (#260)
- Open on Enter in the palette, copy on Ctrl+C (#259)
- Give a card with no place a seat nothing is standing on (#258)
- Leave the keyboard where it is when a card is rebuilt (#257)

### 🧰 Under the hood

- Bring the reference in line with what v0.3.2 changed (#276)
- Implement SonarQube scans in CI workflow (#228)
- Implement SonarQube scans in CI workflow (#227)
- Implement SonarQube scans in CI workflow (#226)
- Implement SonarQube scans in CI workflow (#225)

## [0.3.1] - 2026-09-18

### ✨ Added

- Let the library rail go down to 160px (#213)
- Spend a card's two rows on what is left to do (#206)
- Cards and zones land where they are dropped, and nothing lines up (#172)
- The library as a tree, in a rail that hides (#168)

### 🔧 Changed

- Make a destructive action look like one before it is pointed at (#194)

### 🐛 Fixed

- Wait on a condition in every scenario that reads after one (#209)
- Hand the format ring down to the badge that draws it (#208)
- Draw the badge whole, with one ring around the selected one (#205)
- Draw every row of a todo list inside the card that holds it (#204)
- Count with a plural rather than an apologetic "(s)" (#199)
- Move the focus among the cards as they are on screen (#198)
- Give a badge's label something to be read against (#196)
- Wait on a condition, not on a duration (#195)
- Give every control a box a pointer can hit (#193)
- Make every text colour legible on the surface it is drawn on (#192)
- Contain the name an attachment is written under (#187)
- The title starts where the buttons end (#174)
- The grip and the selection tick share one corner (#171)
- A card dragged out of a folder leaves nothing under the pointer (#170)
- A todo list on the board cannot be ticked (#169)
- Choosing a note in the palette hides the window instead of opening it (#173)

### 🔒 Security

- Retire a passphrase everywhere this application put a copy (#189)
- Measure the key derivation where it cannot be a debug number (#188)

### 🧰 Under the hood

- Rename the application to DevNotes (#211)

## [0.3.0] - 2026-09-17

### ✨ Added

- A workspace, not a pile (#153)

## [0.2.0] - 2026-09-16

### ✨ Added

- Say what a bulk action touches, and offer to take it back (#146)
- Check the library is sound, and offer a way out when it is not (#145)
- Take a rolling copy of the library at launch (#144)
- Declare the two mirrored rules once, in Rust (#142)
- Give the four files with no spec one each (#141)
- Encrypt the library at rest, behind a passphrase (#135)

### 🐛 Fixed

- Write only the columns an edit moved (#140)
- Compare the query params exhaustively, like the patch table next door (#139)
- Seed the first launch as one write (#138)
- Let a storage failure reach the screen instead of killing the launch (#137)
- Choose the durability rather than inherit it (#136)

### 🧰 Under the hood

- Measure what a query costs as the corpus grows (#143)

## [0.1.4] - 2026-09-16

### ✨ Added

- Measure what the commands cost, and save a baseline (#126)
- Let a space be pinned to the head of the list (#125)
- Leave a filtered canvas in one gesture (#123)
- Say how many notes a search matched, and why each one is there (#112)
- Give a card's body the line the badge row was taking (#111)
- Let an update be silenced, and say it is still there (#100)
- Remember the window's size and position (#99)
- Add Rust, Go, Java, C#, PHP and C to the language list (#96)

### 🔧 Changed

- Let a red E2E leg block the run (#109)

### 🐛 Fixed

- Show a todo list the item a search found it by (#124)
- Keep the tag rail's Manage button out of the row that scrolls (#122)
- Make the e2e suite tell the truth, and fix what it was telling (#101)
- Degrade a bundle from a newer version instead of refusing it whole (#95)
- Fold the accents in search, not just the case (#94)
- Give every text field its text cursor back (#93)
- Make fullscreen actually fill the screen (#92)
- Align the attachments band with the editor's other bands (#90)

### 🧰 Under the hood

- Keep the comments that say what the code cannot (#128)
- Refilm the README, and drop the screenshot the GIF repeats (#113)
- Give a card title its width back, and plant the pin in the corner (#114)
- Say which platforms DevNotes ships for, and stop implying macOS (#110)
- Make the e2e suite pass, and fix the data loss it was reporting
- Film the README on a full board (#105)
- Show a note created while its editor was closing (#104)
- Make the README a front door (#103)
- Bump the rust-minor group across 1 directory with 8 updates (#91)
- Bump eslint from 9.39.5 to 10.10.0 (#89)
- Bump @eslint/js from 9.39.5 to 10.0.1 (#87)
- Bump png from 0.17.16 to 0.18.1 in /src-tauri (#84)
- Bump the npm-minor group with 15 updates (#86)
- Bump base64 from 0.22.1 to 0.23.1 in /src-tauri (#85)
- Bump toml from 0.9.12+spec-1.1.0 to 1.1.3+spec-1.1.0 in /src-tauri (#83)
- Bump the actions group with 7 updates (#81)
- Review the Rust back-end (#75)

## [0.1.3] - 2026-09-13

### ✨ Added

- Open in the system language when none has been chosen (#53)

### 🐛 Fixed

- Move the titlebar menus to the left (#52)

### 🧰 Under the hood

- Generate the changelog from merged pull requests and their labels (#67)
- Add e2e testing harness (#66)
- Give the front-end tree the shape of the interface (#65)
- The section and the card read their own state (#63)
- The editor overlay reaches for its own stores (#62)
- Delete the contribution registries, each menu owns its actions (#61)
- Describe the application from Cargo.toml, at compile time (#58)
- Size dialogs with CSS custom properties, not inputs (#57)
- Drop the DTO aliases, rename note.dto.ts to note.mapper.ts (#56)
- Merge AppShellComponent into AppComponent (#51)
- Enhance code safety, optimize performance, and improve documentation (#8)

## [0.1.2] - 2026-09-11

### ✨ Added

- **Todo-list notes.** A note is now either a snippet or a checklist: ordered items, ticked from
  the card without opening it, reordered with the pointer or `Alt+↑` / `Alt+↓`.
- **`{{field}}` values are kept.** What you type into a snippet's fields is stored with the note,
  and global variables (Preferences → Variables) propose a value for the whole corpus.
- **Preferences panel.** Theme, density, tray behaviour, start with the system, the quick-paste
  shortcut and copy confirmations, all applied as they are typed.
- **Help in the About menu:** "Nouveautés" (this file), "Prise en main" and "Raccourcis clavier".
- **Sample notes on first launch**, in their own space, to show what a note can carry.

## [0.1.1] - 2026-08-27

### 🐛 Fixed

- Various fixes to note editing and to the test suite.

## [0.1.0] - 2026-07-28

### ✨ Added

- **Notes and spaces.** Spaces to file notes in, with creation, renaming and deletion — deleting a
  space moves its notes rather than dropping them.
- **The canvas.** Sections by age, pinned notes first, full-text search, tag rail, language rail and
  quick filters, all computed by the Rust engine.
- **The editor.** Title, body, language, tags, source, pin, deadline, move to another space.
- **Syntax highlighting** of the body, for thirteen languages.
- **`{{fields}}` in a snippet**, filled before copying.
- **Trash with a 30-day retention**, and `Ctrl+Z` to undo a deletion.
- **Attachments**, dropped on the window or pasted from the clipboard.
- **Import, export and share** as a bundle or as Markdown.
- **Quick-paste palette** on a global shortcut, plus capture and new note.
- **System tray**, translated, with the window filed away on close.
- **Signed application updates**, offered and never installed behind your back.
- **French and English interface.**
