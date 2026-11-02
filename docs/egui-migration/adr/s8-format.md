# S8 — Formatage : Prettier dans QuickJS (D1)

- **Statut** : accepté (2026-11-02)
- **Ticket** : #1143 · plan § 3.9.4, D1, R6
- **Prototype** : `spikes/s8-format` (`golden.mjs`, binaires `prettier_qjs` et `native`)

## Contexte

D1 : Prettier dans QuickJS si p95 ≤ 500 ms pour un snippet de 13 kB, hôte gardé chaud ; sinon
formateurs natifs, écarts relus un par un.

## Ce qui a été mesuré

Fixtures : les entrées de `prettier-runner.spec.ts` (14 cas, 12 parseurs ; `{{champs}}`
remplacés par des marqueurs comme le fait `field-shield`) et cinq textes réels de 6 à 13 kB
(`http-rail.component.ts`, un extrait de `package-lock.json`, `docs/releasing.md`,
`styles.scss`, un workflow YAML). Référence : Prettier 3.9.10 sous Node, options de
`prettierOptions(DEFAULT_PRETTIER_SETTINGS, '  ')`. QuickJS : `rquickjs` 0.14 (celui de l'hôte
de scripts), `standalone.js` + 9 plugins évalués depuis la source, 20 passes chaudes.

| Fixture            | Taille  | QuickJS p50 | QuickJS p95 | = Node | Natif p95 | Natif = Prettier         |
| ------------------ | ------- | ----------- | ----------- | ------ | --------- | ------------------------ |
| TypeScript         | 13,2 kB | 265 ms      | **340 ms**  | oui    | 5,1 ms    | non (125/369 lignes)     |
| JSON               | 13,3 kB | 111 ms      | 154 ms      | oui    | 1,2 ms    | oui                      |
| Markdown           | 6,3 kB  | 213 ms      | 266 ms      | oui    | 0,5 ms    | oui                      |
| SCSS               | 8,0 kB  | 77 ms       | 104 ms      | oui    | 0,7 ms    | non (7/252)              |
| YAML               | 12,4 kB | 104 ms      | 161 ms      | oui    | 1,3 ms    | non (4/389)              |
| 14 cas des specs   | < 120 B | 1–8 ms      | ≤ 14 ms     | 14/14  | < 0,1 ms  | 8/14 (HTML, Vue, Angular, CRLF YAML…) |

QuickJS : chargement à froid de Prettier + 9 plugins 253 ms (une fois), 28 Mo résidents.
Natifs : `dprint-plugin-typescript` 0.96 trie les imports par défaut, `markup_fmt` demande un
formateur externe pour `<script>`/`<style>` (non branché ici), `pretty_yaml` perd le CRLF.
Sorties brutes : `spikes/out/s8-*.txt`, `spikes/out/s8-native/`.

## Décision

**D1 : Prettier dans QuickJS.** Le critère est tenu (340 ms ≤ 500 ms, sur une machine plus lente
par cœur que celle de référence) et la sortie est **identique** à l'application actuelle sur
toutes les fixtures : plus de corpus d'or du formatage à relire, plus d'écart à accepter.

## Conséquences

- Un hôte QuickJS **gardé chaud** sur un thread dédié (un `Runtime` n'est pas partagé entre
  threads), chargé paresseusement au premier formatage ou au repos ; on donne du texte, jamais
  du code (`field-shield` porté en Rust, inchangé).
- Le paquet Prettier (2,4 Mo de JS pour les 10 fichiers) est empaqueté comme le bac à sable
  (`sandbox/build.mjs`) : `package.json` garde la dépendance `prettier`, figée à la version
  actuelle. Précompiler en bytecode est une optimisation possible du chargement, non nécessaire.
- La marge n'est que de ×1,5 sur le TS de 13 kB : un snippet de 40 kB dépasse la seconde. Le
  formatage reste asynchrone (`Pending`), avec le même indicateur qu'aujourd'hui.
- Les formateurs natifs (100× plus rapides) restent le repli si une mesure sur la machine de
  référence infirme celle-ci ; la liste des écarts ci-dessus en est le point de départ.
