# S7 — Mermaid : `merman`, moteur natif tiers (D2)

- **Statut** : accepté (2026-11-02)
- **Ticket** : #1142 · plan § 3.9.3, D2, E6
- **Prototype** : `spikes/s7-mermaid` (`cargo run --release -p s7-mermaid`, depuis `spikes/s7-mermaid`)

## Contexte

D2 : un moteur natif tiers s'il passe S7, sinon organigrammes et séquences maison. Les
échantillons du premier lancement n'ont aucun bloc `mermaid` ; les diagrammes réellement écrits
sont ceux des specs (`graph TD; A-->B`, erreurs de syntaxe), du scénario e2e 03
(`sequenceDiagram` « POST /login »), et les sept du plan de migration (trois organigrammes dont
un à sous-graphes imbriqués, trois séquences, un Gantt). S'y ajoutent un exemple de chaque type
demandé (classes, états, camembert, organigramme accentué à sous-graphe).

## Candidats

Recherche crates.io « mermaid » (2026-11-02) ; retenus les trois moteurs SVG les plus avancés :

| Crate                 | Version | Licence           | Types annoncés | Note                                                    |
| --------------------- | ------- | ----------------- | -------------- | ------------------------------------------------------- |
| `merman`              | 0.9.3   | MIT OR Apache-2.0 | 23+            | comparé aux fixtures de Mermaid amont ; utilisé par Zed |
| `mermaid-rs-renderer` | 0.3.1   | MIT               | 23             | le plus téléchargé                                      |
| `mermaid-svg`         | 0.7.0   | MIT               | 23             |                                                         |

## Ce qui a été mesuré

Chaque diagramme rendu en SVG puis rastérisé par `resvg` (la voie d'image du plan), images
comparées à l'œil. Sorties : `spikes/out/s7/<moteur>/`, planches `spikes/out/s7/compare-*.png`.

| Diagramme                                    | `merman`                    | `mermaid-rs-renderer`                       | `mermaid-svg`              |
| -------------------------------------------- | --------------------------- | ------------------------------------------- | -------------------------- |
| organigrammes des specs                      | ✅ 1–8 ms                   | ✅ < 1 ms                                   | ✅ < 1 ms                  |
| organigramme accentué, sous-graphe           | ✅ 12 ms                    | ✅ 50 ms                                    | ✅                         |
| organigramme du plan, sous-graphes imbriqués | ✅ 14 ms, fidèle            | ⚠️ 250 ms                                   | ❌ sous-graphes superposés |
| séquences (e2e + 3 du plan)                  | ✅ 3 ms                     | ✅                                          | ✅                         |
| classes (génériques `~T~`, cardinalités)     | ✅ `Vec<String>`, `*`/`1`   | ❌ `Vec~String~` laissé, cardinalité perdue | ✅                         |
| états                                        | ✅                          | ⚠️ boucles mal placées                      | ✅                         |
| Gantt du plan                                | ✅                          | ✅                                          | ✅                         |
| camembert                                    | ✅                          | ✅                                          | ✅                         |
| erreur `graph TD\n  A -->`                   | erreur typée + `SourceSpan` | « 1:1 », ligne fausse                       | « line 2 », juste          |

## Décision

**D2 : `merman`.** Seul candidat qui rend correctement tous les diagrammes du jeu, y compris
l'organigramme à sous-graphes imbriqués ; sa compatibilité est vérifiée contre Mermaid amont ;
licence compatible GPL-3.0 ; ses erreurs portent un `SourceSpan` (octets) dont `diagram` tire la
ligne. Le plan B (organigrammes et séquences maison) est abandonné.

## Conséquences

- **Pipeline `SvgPipeline::resvg_safe()`** obligatoire : par défaut, `merman` écrit les libellés en
  `foreignObject` HTML (comme Mermaid), que `resvg` ne dessine pas.
- **Mesure du texte par nos polices** : avec la mesure par défaut, des libellés sont rognés au
  rendu `resvg` (« Folde », « kinc »). La feature `native-fonts` (`with_native_font_context`)
  mesure avec les polices fournies : `diagram` lui passe Inter embarquée, comme le dessin.
- Couleurs : `merman` lit la configuration de thème de Mermaid (`theme: base` +
  `themeVariables`) ; `diagram` la remplit depuis les jetons, comme `MermaidRenderer` aujourd'hui.
- Compiler sans les types inutiles (features `diagram-*`) et sans ELK si la taille du binaire
  le demande ; les temps (≤ 20 ms) permettent le rendu sur `compute` après pause de frappe.
- E6 reste (rendu proche, pas identique) ; R6 baisse pour Mermaid.
