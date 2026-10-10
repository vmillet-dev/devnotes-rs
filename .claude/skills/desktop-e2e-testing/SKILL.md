---
name: desktop-e2e-testing
description: Use when writing, running or debugging end-to-end specs for the assembled Tauri app (WebdriverIO, e2e/specs/). Not for the website, which has no e2e suite (marketing-site).
---

# Desktop e2e

- Build first, every time `src/` or `src-tauri/` changed: `npm run build`, `npm run e2e:build`, then `npm run test:e2e` (resets the profile, runs `wdio`). The WebDriver server is inside the `e2e` build: no `tauri-driver`, no msedgedriver to install.
- All specs share one process, one DB, one preferences file; the numeric prefix is the run order. Each file seeds its own preconditions. New spec = next number, `e2e/specs/NN-name.e2e.ts`; reuse `e2e/support/`.
- Wait on conditions with `eventually` (`support/app.ts`), never a duration; a `browser.pause` is only for "nothing happened" and marked `deliberately` (`scripts/e2e-waits.test.mjs` enforces it).
- Select by `data-testid`, never by text: the suite switches language. Add the test id in the template if missing.
- `reopenSession()` is not a restart. Never point the suite at a real library.
- Lint the specs: `npm run lint` (type-checks `e2e/`) and `npm run test:scripts`.
- On failure, read the wdio log and screenshot before changing the spec; decide whether the app or the test is wrong.
