# Golden corpora

What the JavaScript libraries of the Angular front produce today, kept for the Rust that replaces them (`docs/egui-migration-plan.md`, § 8.6): each port reproduces these outputs, or lists where it differs and why. Data, not tests: nothing here is asserted by CI.

Written by `npm run golden` (the `*.golden.spec.ts` files, inert in `npm test`), in the `Europe/Paris` time zone. Two runs give the same bytes; rewrite them only on purpose, and say in the commit which library moved.

Each `<subject>/cases.json` is a list of `{ input, options?, output }`.

| Subject           | What it captures                                                                                                                 | Source                                            |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `markdown`        | A Note's Markdown read into the editor and written back: every GFM construction, the sample notes, what the schema does not hold | TipTap 3.32.0                                     |
| `markdown-escape` | Typed text as it is stored, at a line's start or not                                                                             | `escapeMarkdownText`                              |
| `html-paste`      | HTML pasted into an empty Note, as the Markdown it becomes                                                                       | TipTap 3.32.0                                     |
| `rich-keys`       | Each editor extension's keyboard shortcuts and input rules                                                                       | TipTap 3.32.0                                     |
| `format`          | Prettier's answer for every parser, two indents, `{{fields}}` kept                                                               | Prettier 3.9.10                                   |
| `syntax-tokens`   | The `tok-*` ranges each grammar puts on the detection corpus (`crates/engine/tests/corpus`)                                      | CodeMirror language 6.13.1, Lezer highlight 1.2.5 |
| `mind-map`        | Where « Présenter »'s mind map places each outline                                                                               | `layoutMindMap`                                   |
| `plurals`         | Every plural key of both locales at counts that tell the rules apart                                                             | our ICU transpiler                                |
| `intl`            | Each `Intl` formatter of the front, with its options and edge values                                                             | Node 26.8.1, ICU 78.3                             |
| `relative-time`   | Relative, expiry and span labels from a second to a year, both locales                                                           | `relative-time.util`                              |
| `calc-format`     | How a Calcul sheet writes each kind of value, its sum and its mean; each setting swept with the others at their default          | `CalcFormatter`                                   |

Captured on 2026-11-03 under Vitest 4.1.11 and jsdom 30.1.2. Mermaid (12.2.0) and Fabric (7.4.0) draw pictures: their references are the screenshots of the visual reference, not files here.
