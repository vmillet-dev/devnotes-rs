# S3 — Canevas : la virtualisation par rangées tient 30 000 cartes

- **Statut** : accepté (2026-11-02)
- **Ticket** : #1138 · plan § 3.6, § 6.2
- **Prototype** : jetable, non conservé dans le dépôt

## Contexte

Le canevas Angular rend toutes les cartes. Le § 3.6 prévoit d'aplatir la vue en rangées virtuelles
(en-tête de section, rangée de cartes de hauteur fixe `150 + gap`), de déduire le nombre de
colonnes de la largeur (`minmax(220px, 1fr)`) et de ne dessiner que les rangées du viewport par
`ScrollArea::show_viewport`. Budget : 60 i/s, p99 du temps CPU d'une frame ≤ 12 ms.

## Ce qui a été mesuré

30 000 cartes peintes à la main (fond, bord au survol, pastille, titre tronqué sur une ligne,
aperçu de 6 lignes monospace en trois couleurs coupé à 5 rangées, deux puces de tags), six
sections (épinglées … plus tôt), zone de 1 660 × 990 px (1 920 × 1 080 moins la barre latérale),
7 colonnes. Décalages cumulés précalculés, première rangée visible par recherche dichotomique.
Une frame = `Context::run` + `tessellate` sans fenêtre (le téléversement GPU n'est pas compté) ;
polices par défaut d'egui. Même machine que S4 (4 vCPU Xeon 2,1 GHz).

| Scénario                                                    | p50     | p95     | p99     | max     |
| ----------------------------------------------------------- | ------- | ------- | ------- | ------- |
| Molette (120 px par frame) + survol qui bouge, 2 000 frames | 1,37 ms | 1,75 ms | 1,82 ms | 3,85 ms |
| Saut à un décalage aléatoire (glisser la barre, Fin), 500   | 1,22 ms | 2,27 ms | 2,38 ms | 5,89 ms |
| Survol seul, 1 000 frames                                   | 0,45 ms | 0,67 ms | 0,73 ms | 0,80 ms |

Première frame (atlas de polices vide) : 3,4 ms. ~34 500 sommets par frame, ~50 cartes visibles.

## Décision

**Le schéma du § 3.6 est validé tel quel** : rangées virtuelles à décalages précalculés,
`show_viewport`, cartes peintes à la main, aucune mise en cache de `LayoutJob` nécessaire pour le
canevas (le cache de galleys d'egui d'une frame à l'autre suffit). Le p99 est six fois sous le
budget.

## Conséquences

- La marge (≈ 10 ms) revient à ce que le prototype ne fait pas : coloration syntaxique réelle des
  aperçus (elle viendra du cache de `LibraryCaches`, D17), miniatures, rendu GPU.
- `pixels_per_point` > 1 ne change pas le nombre de sommets ; à remesurer avec Inter (S2) dans le
  banc d'interface de P3.
