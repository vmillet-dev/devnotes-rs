# Golden files

Test data: inputs, and the exact output expected of each. The tests below read these files and compare word for word, so a change in what the application writes or shows fails there. Nothing here ships with the application.

Each `<subject>/cases.json` is a list of `{ input, options?, output }`.

| Subject           | What it holds                                                                             | Read by                                                               |
| ----------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `markdown`        | A Note's Markdown as it comes in, and as the editor writes it back                        | `crates/richtext/tests/golden.rs`                                     |
| `markdown-escape` | Typed text, escaped only where it would read as Markdown                                  | `crates/richtext/tests/golden.rs`                                     |
| `html-paste`      | HTML pasted into a Note, and the Markdown it becomes                                      | `crates/richtext/src/html.rs`                                         |
| `format`          | « Formater »: Prettier's answer for each parser, `{{fields}}` kept                        | `crates/text/src/format/tests.rs`                                     |
| `intl`            | Numbers, dates, lists and countries, in French and English                                | `crates/i18n/src/format.rs`                                           |
| `relative-time`   | « il y a 5 min », « dans 2 jours », an expiry, in French and English                      | `crates/i18n/src/time.rs`                                             |
| `calc-format`     | A Calcul sheet's values as they are shown, its sum and its mean                           | `crates/ui/src/notes/calc.rs`, `crates/engine` (`notes/calc/summary`) |
| `mind-map`        | Where a presentation's mind map places each heading                                       | `crates/ui` (`presentation/mind_map`)                                 |
| `fabric`          | A drawing as it is stored (the format is Fabric's), read and written back to the byte     | `crates/drawing`, and `crates/ui`'s captures                          |
| `fabric-wrap`     | Where a drawing's text breaks its lines, at a width and a font size, in the embedded font | `crates/drawing/src/text/tests.rs`                                    |

## When one of these tests fails

The output changed. Either it was not meant to, and the code is wrong; or it was, and the case's `output` is corrected here by hand, with the reason in the commit. `markdown` and `fabric` hold what people have stored: a difference there changes a note or a drawing the next time it is saved.

A new case is written by hand too: nothing generates these files. `format` moves when the version of Prettier does, and `intl` when a pattern is added.
