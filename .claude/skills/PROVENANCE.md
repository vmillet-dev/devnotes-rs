# Provenance of installed skills

| Skill               | Source                                                                            | Commit                                   | Licence    | Reviewed   |
| ------------------- | --------------------------------------------------------------------------------- | ---------------------------------------- | ---------- | ---------- |
| `frontend-design`   | https://github.com/anthropics/skills/tree/main/skills/frontend-design             | dbd4588f9e1033efb41dad4bef2f7947c8993d44 | Apache-2.0 | 2026-10-10 |
| `webapp-testing`    | https://github.com/anthropics/skills/tree/main/skills/webapp-testing              | dbd4588f9e1033efb41dad4bef2f7947c8993d44 | Apache-2.0 | 2026-10-10 |
| `angular-developer` | https://github.com/angular/skills/tree/main/angular-developer                     | 07e3c101ac123dad59676c8d8e5fd2b1a67a48e4 | MIT        | 2026-10-10 |
| `seo`               | https://github.com/addyosmani/web-quality-skills/tree/main/skills/seo             | afa8da942115f2961fdbfa80807ea0b232ff6c00 | MIT        | 2026-10-10 |
| `core-web-vitals`   | https://github.com/addyosmani/web-quality-skills/tree/main/skills/core-web-vitals | afa8da942115f2961fdbfa80807ea0b232ff6c00 | MIT        | 2026-10-10 |
| `accessibility`     | https://github.com/addyosmani/web-quality-skills/tree/main/skills/accessibility   | afa8da942115f2961fdbfa80807ea0b232ff6c00 | MIT        | 2026-10-10 |

Unmodified copies, each read in full before it was added; see each `LICENSE.txt`. Every other folder here is local to DevNotes. The copies are listed in `.prettierignore` so that formatting never makes them differ from their source. To update one: read the new version in full, replace the folder whole, then change its commit and date here.

- **`anthropics/skills`**: `webapp-testing` needs Python + Playwright and is for looking at the website in a real browser: it drives neither the Tauri app (see `desktop-e2e-testing`) nor a test suite (see `marketing-site`).
- **`angular/skills`** is a daily snapshot of `angular/angular` (`skills/dev-skills/`), here built from `458f1d76b8a44c1b091e4239e1f95ec949390e73`. Markdown only. The snapshot ships no licence file, so `LICENSE.txt` is `angular/angular`'s at that commit: the one file added. It is generic Angular; `angular-ui` and `CLAUDE.md` win where they disagree.
- **`addyosmani/web-quality-skills`**: Markdown only, meant for the marketing site. `LICENSE.txt` is the repository's `LICENSE`. Their links into `../performance/` and `../web-quality-audit/` point at two skills that are not vendored (the second ships a shell script).
