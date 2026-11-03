# The Angular application, for reference

The Angular front-end (`src/`) and its WebdriverIO scenarios (`e2e/`) as they were at the
`angular-reference` tag, kept to read how a screen, a component or a scenario was built while it
is ported to egui. Nothing here is built, linted, tested or formatted, and nothing here is
updated: the translations moved to `crates/i18n/translations/`.

To run it, check the tag out beside the repository:

```bash
git worktree add ../devnotes-angular angular-reference
```

then `npm ci` and `npm run tauri dev` in that folder. Its profile is `com.devnotes.app`; the egui
application uses `com.devnotes.app.dev`, so the two never share a library.
