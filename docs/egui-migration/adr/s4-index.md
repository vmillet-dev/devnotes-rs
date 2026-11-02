# S4 — Index de notes en mémoire : pas de `lz4` pour le texte plié

- **Statut** : accepté (2026-11-02)
- **Ticket** : #1139 · plan § 3.5, § 6.1, § 6.2, R4
- **Prototype** : `spikes/s4-index` (`cargo run --release -p s4-index -- <realistic|stress> <fichier> <plain|lz4>`)

## Contexte

Le § 3.5 garde, après déverrouillage, un index déchiffré et plié du corpus. Sa règle 6 prévoyait de
compresser aussi le texte plié par `lz4` si la mémoire du corpus de stress dépassait le budget du
§ 6.2 (≤ 350 Mo réaliste, ≤ 900 Mo stress).

## Ce qui a été mesuré

Corpus générés dans un SQLite fichier, colonnes scellées comme `vault::key` (AES-256-GCM, nonce
en tête, base64), index `notes_updated_at` du vrai schéma ; texte accentué (le chemin non ASCII de
`fold`). Réaliste : 20 000 snippets de 1,5 kB, 5 000 Notes de 4 kB, 3 000 listes de 20 éléments,
1 500 feuilles Calcul, 500 Dessins de 20 kB, 600 tags. Stress : 30 000 notes de 13 kB.
Construction : lecture SQLite séquentielle par tranches de 2 000 du plus récent au plus ancien,
déchiffrement + pli + `lz4` du corps en parallèle (`rayon`). Requêtes : 30 passes, recherche par
`memchr::memmem` en parallèle.

Machine : conteneur Linux, 4 vCPU Xeon 2,1 GHz, 15 Go — plus lente par cœur que la machine de
référence du § 6.2.

| Mesure                              | Réaliste, plié clair | Réaliste, plié `lz4` | Stress, plié clair | Stress, plié `lz4` | Budget (réaliste / stress) |
| ----------------------------------- | -------------------- | -------------------- | ------------------ | ------------------ | -------------------------- |
| Première tranche (2 000 notes)      | 84 ms                | 89 ms                | 99 ms              | 132 ms             | premières cartes ≤ 600 / 800 ms |
| Index complet                       | 0,42 s               | 0,44 s               | 1,00 s             | 1,15 s             | ≤ 1,5 / 4 s                |
| Mémoire résidente (processus)       | 108 Mo               | 77 Mo                | 465 Mo             | 467 Mo             | ≤ 350 / 900 Mo             |
| Tas : texte plié / corps `lz4`      | 63 / 16 Mo           | 16 / 16 Mo           | 384 / 40 Mo        | 40 / 40 Mo         | —                          |
| Recherche absente (pire cas), p95   | 2,5 ms               | 7,9 ms               | 16,4 ms            | 39,4 ms            | frappe → peint ≤ 50 / 120 ms |
| Vue vide triée, p95                 | 0,9 ms               | 1,3 ms               | 1,2 ms             | 1,2 ms             | —                          |
| Filtre espace + type + tag, p95     | 0,2 ms               | 0,4 ms               | 0,3 ms             | 0,3 ms             | —                          |
| Facettes (tags, types, dossiers)    | 0,4 ms               | 0,4 ms               | 2,0 ms             | 0,4 ms             | `list_tags` ≤ 5 / 10 ms    |

Sortie brute : `spikes/out/s4.txt` (non versionnée, régénérée par le prototype).

## Décision

**Le texte plié reste en clair.** Le corpus de stress tient en 465 Mo pour 900 Mo de budget ; la
compression du texte plié multiplie la recherche par 2,5 à 3 et ne baisse même pas la mémoire
résidente du stress (467 Mo) : l'allocateur garde les pages libérées par le déchiffrement
parallèle. La règle 6 du § 3.5 n'est pas déclenchée.

## Conséquences

- **La tranche est lue par l'index `notes_updated_at`.** Une première version du prototype, sans
  cet index, faisait trier par SQLite toutes les lignes avec leurs corps scellés : 1,76 s pour la
  première tranche du stress, au-delà du budget. La requête de construction doit donc rester
  servie par un index (`EXPLAIN QUERY PLAN` sans `USE TEMP B-TREE FOR ORDER BY`) : à tester en P1.
- Le texte synthétique se répète et se compresse trop bien (`lz4` ×10 sur les corps) ; un corps
  réel se compresse plutôt ×2,5. Estimation corrigée du stress : 465 − 40 + ~160 ≈ **620 Mo**,
  toujours sous 900 Mo ; à reprendre sur le corpus de P1 (`benches/corpus.rs` étendu).
- La recherche est 3 à 8 fois sous son budget : la marge revient au rendu et à `view::build`,
  non mesurés ici (le prototype ne rend que des identifiants).
- R4 (mémoire de l'index) passe de « moyenne » à **faible** probabilité pour la mémoire ; le
  risque d'incohérence après une écriture oubliée reste entier (oracle et `Changed`, P1).

## Non vérifié ici

Pli d'un Dessin par `notes::drawing::words`, décorations (`store::decorations`), coût de
`view::build` générique, `Zeroizing` des textes (même taille, effacement à la libération).
