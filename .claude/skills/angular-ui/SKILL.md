---
name: angular-ui
description: Use when creating or editing Angular components, templates, styles, stores or translations in src/app (standalone, signals, zoneless, OnPush). For the website use marketing-site.
---

# Angular UI

- Component: standalone, `OnPush`, signals; private `_x` writable signals exposed via `.asReadonly()`; derived state in `computed()`. No `new Date()` in `computed` (use `ClockService`).
- Location mirrors the screen (`notes/{sidebar,header,canvas,overlays}`, `tools/`, `http/`…); non-visual code lives in `core/`; `shared/` injects nothing (except `DialogComponent`). Check "Where a file goes" before creating a file.
- Framework questions (signals, `linkedSignal`, `resource`, control flow, host bindings, testing patterns) go to the vendored `angular-developer`. It is generic Angular, so here the project wins: no router, no `HttpClient`, no SSR, no Tailwind; classes keep their suffixes and `@Injectable({ providedIn: 'root' })`; files are placed by hand, not by `ng generate`; `@angular/forms`, `@angular/cdk` and `@angular/aria` are not installed, and `ng add` / `npm install` is a dependency change the user approves first.
- Only `core/data/` and `core/ipc/` call generated commands. Never filter or group on the front what Rust returns ready-made.
- Modals use `DialogComponent`. Lazy parts use `@defer` with the import on its own line.
- Strings: translation keys in both `src/app/core/services/i18n/translations/{fr,en}.json`; counts are ICU plurals.
- Accessibility: visible focus, labels on icon buttons, targets ≥ 24×24, 4.5:1 text contrast, keyboard twin for every pointer gesture, reduced motion respected. Colours are CSS variables on `:root` in `src/styles/styles.scss`.
- Responsive: the window resizes; check the narrow width, both themes (light/dark) and densities.
- Verify: `npm run lint`, `npm test`, and look at it with `preview_start` (`npm start`, port 1420) when the change is visible. Tauri-only APIs do not run there; say so rather than claiming a check.
