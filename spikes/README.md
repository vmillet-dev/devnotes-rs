# Spikes de la phase 0 (migration egui)

Prototypes jetables du § 7 « Phase 0 » de `docs/egui-migration-plan.md` : un workspace Cargo à
part, hors du produit, hors lint. Rien ici n'est livré ni réutilisé tel quel. Chaque spike a son
ADR dans `docs/egui-migration/adr/`, qui donne la commande pour le rejouer et ses mesures.

| Dossier        | Ticket | ADR                    |
| -------------- | ------ | ---------------------- |
| `s1-shell`     | #1136  | `s1-shell.md`          |
| `s2-fonts`     | #1137  | `s2-fonts.md`          |
| `s3-canvas`    | #1138  | `s3-canvas.md`         |
| `s4-index`     | #1139  | `s4-index.md`          |
| `s5-highlight`, `s5-editor` | #1140 | `s5-text-core.md` |
| `s6-rich`      | #1141  | `s6-rich-editor.md`    |
| `s7-mermaid`   | #1142  | `s7-mermaid.md`        |
| `s8-format`    | #1143  | `s8-format.md`         |
| `s9-packages`  | #1144  | `s9-packages.md`       |
| `s10-e2e`      | #1145  | `s10-e2e-driver.md`    |

Les commandes se lancent depuis `spikes/` (`cargo run --release -p <spike>`), sauf mention
contraire de l'ADR. Les sorties vont dans `spikes/out/` (non versionné). Les deux variantes de
`s5-highlight` se construisent séparément (`--features fancy`, `--features onig`).
