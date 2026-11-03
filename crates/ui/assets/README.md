# Embedded assets

| File                                          | What                                      | Source                                                               | Licence                          |
| --------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------- | -------------------------------- |
| `fonts/inter-latin*-wght-normal.ttf`          | Inter, variable weight, Latin + Latin Ext | `@fontsource-variable/inter` 5.3, its woff2 decompressed to TrueType | OFL 1.1, `Inter-OFL.txt`         |
| `fonts/jetbrains-mono-latin*-wght-normal.ttf` | JetBrains Mono, variable weight           | `@fontsource-variable/jetbrains-mono` 5.3, likewise                  | OFL 1.1, `JetBrainsMono-OFL.txt` |
| `fonts/NotoEmoji-wght.ttf`                    | Noto Emoji, monochrome, variable weight   | `google/fonts`, `ofl/notoemoji/NotoEmoji[wght].ttf` (2026-11-03)     | OFL 1.1, `NotoEmoji-OFL.txt`     |
| icon paths in `src/theme/icons.rs`            | Lucide                                    | <https://lucide.dev>, as the Angular reference drew them             | ISC, `LUCIDE-LICENSE.txt`        |

The Latin subsets are the ones the reference loaded; what they do not draw comes from the
system's fonts.
