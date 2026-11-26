---
name: marketing-site
description: Use when planning or building the DevNotes marketing/showcase website in website/ (Angular, prerendered; not the in-app UI). Drives art direction from the real product and holds the SEO, performance and accessibility floor. Pairs with frontend-design, seo, core-web-vitals and accessibility.
---

# Marketing site

Load `frontend-design` and follow its plan → review → build → critique process. Then:

1. Ground it in the product: read `README.md`, the three tours (`docs/gifs/`) and the app's real tokens (`crates/ui/src/theme/tokens.rs`: palette, amber accent, density, dark base). The site should feel made by the same hands as the app, not from a template.
2. Real content only: encrypted multi-library vault, snippets with `{{fields}}`, Markdown notes, todo lists, canvas/board, quick-paste palette, HTTP/JSON/tools. Show real UI captures or live demos, not icons in cards.
3. Avoid: identical feature cards, SaaS hero with gradient and big stat, fade-up on every section, decorative blobs, a hue per section, a window redrawn where a capture would do. One memorable element; the rest stays quiet.

## What the design rests on

Chosen in #1655 among nine proposals; a new page is drawn from these, not from a template.

- **A construction grid, drawn** (`src/styles/_frame.scss`): one column between two hairlines down the whole page, sections between rules with a cross where they meet, each headed by a mono row with its number and name (`01 / L'application`). Content sits in hairline cells (`.cells`), never in floating cards; a large screen is filled by the grid, not by empty bands.
- **The application itself**: a real capture at the top (`app::screens` at 2×, one per language and theme), with numbered marks and a legend that says what each part does. No animation at the top of a page.
- **A developer's day**: the home page's moments are rows, words on the left, a scene on the right that moves the application's own pieces in its own words (`src/styles/_pieces.scss`, `home/day/`). A scene is written in its end state and only plays, on screen, when script and motion allow; on a wide screen the hour stands in the right margin.
- The application's palette and its two faces, Inter and JetBrains Mono, self-hosted: no other face.

4. It is an Angular project of its own in `website/`, outside the Cargo workspace: its own `package.json` and lockfile, `npm ci` from `website/`, and a dependency the site alone needs is still one the user approves first. It imports nothing from the application: the design tokens are a copy of `crates/ui/src/theme/tokens.rs` (`scripts/tokens.test.mjs` keeps them equal). The in-app desktop rules (egui, no router) do not apply there. A workflow publishes the build to the site's own repository; the public URL is a build input, never hard-coded, and canonical URLs, the sitemap and Open Graph tags are built from it.
5. Every route is prerendered (SSG): a client-rendered Angular page hands a crawler an empty `<app-root>`. Prove it on the build output: each route's built HTML carries its text without running JavaScript.
6. SEO, per route (`seo`): its own `<title>` and meta description, a canonical URL, an Open Graph image, a descriptive `<h1>`, real text rather than text in images, alt text on captures. `robots.txt` and `sitemap.xml` at the root. JSON-LD only for what the page shows (`SoftwareApplication` is the type that fits). `lang` on `<html>`, and `hreflang` pairs if the site ships in French and English.
7. Performance (`core-web-vitals`): the largest element above the fold, usually the hero capture, is in the initial HTML with explicit dimensions, `srcset` widths and `fetchpriority="high"`. A capture that differs by theme is a `<picture>` whose light source follows the system's theme until the site's is known: one file fetched, so no preload (it would fetch the other half's file). Fonts are self-hosted; nothing shifts when captures or fonts arrive. Measure before quoting a number.
8. Quality floor (`accessibility`): mobile to desktop, keyboard focus, reduced motion, AA contrast, 24×24 targets, no third-party trackers, download links to GitHub releases.
9. Tests assert the rendering, not the logic: Vitest + jsdom on the rendered page (heading outline, landmarks, accessible names, `alt` and dimensions, both locales holding the same keys), plus a check over the prerendered output. No HTML snapshots, and no e2e suite for the site; `webapp-testing` is only for looking at a built page in a real browser.
