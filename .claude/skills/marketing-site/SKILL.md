---
name: marketing-site
description: Use when planning or building the DevNotes marketing/showcase website in website/ (Angular, prerendered; not the in-app UI). Drives art direction from the real product and holds the SEO, performance and accessibility floor. Pairs with frontend-design, angular-developer, seo, core-web-vitals and accessibility.
---

# Marketing site

Load `frontend-design` and follow its plan → review → build → critique process. Then:

1. Ground it in the product: read `README.md`, `docs/tour.gif`, `docs/scratch-*.html` and the app's real tokens (`src/styles/styles.scss`: palette, amber accent, density, dark base). The site should feel made by the same hands as the app, not from a template.
2. Real content only: encrypted multi-library vault, snippets with `{{fields}}`, Markdown notes, todo lists, canvas/board, quick-paste palette, HTTP/JSON/tools. Show real UI captures or live demos, not icons in cards.
3. Avoid: identical feature cards, SaaS hero with gradient and big stat, fade-up on every section, decorative blobs. One memorable element; the rest stays quiet.
4. It is a second project of the app's workspace, in `website/` (`angular.json` → `projects.website`), on the root `package.json`: the app's Angular version, no second `package.json`, and a dependency the site alone needs is still one the user approves first. The two bundles stay apart: nothing under `website/` imports from `src/app/`; the design tokens are the one thing shared. The app's desktop rules (no router, IPC repositories) do not apply there: framework questions go to `angular-developer`. A workflow publishes the build to the site's own repository; the public URL is a build input, never hard-coded, and canonical URLs, the sitemap and Open Graph tags are built from it.
5. Every route is prerendered (SSG): a client-rendered Angular page hands a crawler an empty `<app-root>`. Decide it when scaffolding (`angular-developer`, rendering strategies) and prove it on the build output: each route's built HTML carries its text without running JavaScript.
6. SEO, per route (`seo`): its own `<title>` and meta description, a canonical URL, an Open Graph image, a descriptive `<h1>`, real text rather than text in images, alt text on captures. `robots.txt` and `sitemap.xml` at the root. JSON-LD only for what the page shows (`SoftwareApplication` is the type that fits). `lang` on `<html>`, and `hreflang` pairs if the site ships in French and English.
7. Performance (`core-web-vitals`): the largest element above the fold, usually the hero capture, is in the initial HTML with explicit dimensions and priority (`NgOptimizedImage`: `ngSrc`, `width`, `height`, `priority`); fonts are self-hosted; nothing shifts when captures or fonts arrive. Measure before quoting a number.
8. Quality floor (`accessibility`): mobile to desktop, keyboard focus, reduced motion, AA contrast, 24×24 targets, no third-party trackers, download links to GitHub releases.
9. Tests assert the rendering, not the logic: Vitest + jsdom on the rendered page (heading outline, landmarks, accessible names, `alt` and dimensions, both locales holding the same keys), plus a check over the prerendered output. No HTML snapshots, and no e2e suite for the site; `webapp-testing` is only for looking at a built page in a real browser.
