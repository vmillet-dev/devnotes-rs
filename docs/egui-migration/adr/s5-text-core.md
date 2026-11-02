# S5 — Noyau d'édition : `ropey` + `EditorView`, coloration `onig` hors du thread UI (D22)

- **Statut** : accepté (2026-11-02) ; **IME japonais sous Windows non vérifié**
- **Ticket** : #1140 · plan § 3.8, § 3.9, § 3.12, D14, D22, E11
- **Prototypes** : `spikes/s5-highlight` (coloration, une fois `--features fancy`, une fois
  `--features onig`), `spikes/s5-editor` (noyau + vue), `spikes/s1-shell` (touches réelles sous X11)

## Contexte

D22 : `fancy-regex` si 1 Mo de JSON et 1 Mo de TypeScript se colorent en ≤ 200 ms (p95) ; sinon
`onig`. Budgets du § 6.2 : frappe ≤ 33 ms dans 1 Mo, ouverture d'un snippet de 1 Mo ≤ 300 ms,
corps de réponse de 1 Mo coloré ≤ 200 ms.

## Ce qui a été mesuré

**Coloration** — `syntect` 5.3 + `two-face` 0.5 (grammaires de bat), `ParseState` + `ScopeStack`
sur tout le texte, 7 passes. Entrées : 1 Mo de JSON (le `package-lock.json`, indenté) et 1 Mo de
TypeScript réel (les `.ts` de `src/` mis bout à bout).

| Moteur        | 1 Mo JSON p50 / max | 1 Mo TypeScript p50 / max | Ligne la plus lente (TS) |
| ------------- | ------------------- | ------------------------- | ------------------------ |
| `fancy-regex` | 287 / 346 ms        | **12 881 / 13 259 ms**    | 82 ms                    |
| `onig`        | 296 / 347 ms        | **2 662 / 3 105 ms**      | 13 ms                    |

Le coût est réparti (les 100 lignes les plus lentes font 2 à 4 % du total) : ce n'est pas une
ligne pathologique, c'est la grammaire TypeScript.

**Noyau et vue** — `ropey` 1.6 (1 Mo chargé en 1,7 ms), historique d'annulation, points de
reprise de l'état `syntect` toutes les 128 lignes, une galley par ligne visible, indentation
suspendue, caret, ligne courante, bulle de complétion ; une frame = `Context::run` + tessellation.

| Mesure (1 Mo de TypeScript, `onig`)                                                           | Résultat                           |
| --------------------------------------------------------------------------------------------- | ---------------------------------- |
| Frappe au milieu (insertion + annulation + recoloration + mise en page)                       | p50 6,0 · p95 **8,9** · p99 9,3 ms |
| Saut à un endroit déjà colorié (recolore ≤ 128 lignes + l'écran)                              | p50 15 · p95 26 · max 39 ms        |
| Premier saut au milieu d'un fichier jamais colorié (points de reprise faits sur le thread UI) | **1 218 ms**                       |

**Indentation suspendue** : faisable sans réécrire la mise en page d'egui — on met en page la
ligne à la largeur moins le retrait, puis on décale les `PlacedRow` suivantes (champ `pos`
public) ; `Galley::pos_from_cursor` suit le décalage, le caret reste juste. Limite : la première
rangée est coupée à la même largeur réduite que les suivantes (CodeMirror lui laisse toute la
largeur) ; à régler en P4 en mettant en page la première rangée à part si l'écart se voit.

**Touches** (`egui` 0.37 / `egui-winit` 0.37 lus, puis X11 réel sous Xvfb, `spikes/s1-shell`) :
`Event::Key` porte `physical_key`, tiré du `KeyCode` de winit, c'est-à-dire le `code` W3C. Tous les
`code` de `NAMED_KEYS` (`shortcut.model.ts`), `KeyA–Z`, `Digit0–9`, `F1–F24` et les
modificateurs ont un `egui::Key`. Mesuré sous X11 avec les positions AZERTY : la touche marquée
A arrive en `key A, physical Q` ; Ctrl+A en `A / Q` ; « & » (Digit1 sans Maj) en `Num1`. Touches
mortes françaises : « ^ » puis « e » donne `Text("ê")`, « ¨ » puis « i » `Text("ï")`.

## Décision

- **D22 : `onig`.** Aucun des deux moteurs ne tient 200 ms ; le critère renvoie à `onig`, 4,8 fois
  plus rapide que `fancy-regex` sur TypeScript.
- **API du noyau** (crate `text`, sans egui) : `Buffer` (rope, annulation regroupée, éditions
  qui invalident les points de reprise à partir de leur ligne) et `Highlighter` (points de
  reprise tous les N lignes, états clonables, plages par ligne demandées pour une fenêtre de
  lignes). `EditorView` (dans `ui`) ne fait que la mise en page des lignes visibles.
- **La coloration ne bloque jamais le thread UI** : les points de reprise sont construits sur
  `compute`, du début vers la fin, la fenêtre visible en priorité ; une ligne pas encore coloriée
  se dessine en couleur de texte, puis se colore quand la réponse arrive (comme CodeMirror).

## Conséquences

- Les budgets « corps de réponse de 1 Mo coloré ≤ 200 ms » et « ouverture d'un snippet de 1 Mo
  ≤ 300 ms » se lisent **premier écran coloré** ; la coloration complète d'1 Mo de TypeScript
  prend ~3 s en tâche de fond. Le plafond actuel (coloration jusqu'à 256 KB) n'est plus
  nécessaire mais le corps reste lisible pendant ce temps.
- Touches non représentables ou confondues (§ 3.12) : `Numpad0–9` = `Digit0–9`,
  `NumpadEnter` = `Enter`, `NumpadSubtract` = `Minus`, `NumpadDivide` = `Slash`, `NumpadAdd` →
  `Plus` ; `NumpadMultiply`, `NumpadDecimal`, `NumpadEqual`, `NumpadComma`, `IntlRo`, `IntlYen`,
  `Lang1–5` : aucun `egui::Key`. Aucune n'est dans la table des raccourcis ni dans `CANVAS_KEYS`.
- **Deux pièges pour `keys::dispatch`** : (1) une touche morte émet aussi un `Event::Key` (sur
  AZERTY, « ^ » arrive en `OpenBracket`) avant le texte composé — un raccourci sans
  modificateur sur cette position ne doit pas se déclencher pendant une composition ; (2) un
  caractère qu'egui ne nomme pas (« & », « é ») retombe sur la touche physique : sur AZERTY, la
  touche 1 sans Maj déclenche `Num1`, là où la WebView donnait `key = "&"`. Le § 3.12 garde la
  fourche actuelle (raccourcis globaux par position, canevas par caractère) : les chiffres de
  priorité (0–4) et les lettres du canevas se lisent donc dans `Event::Text`, pas dans
  `Event::Key::key`, sans quoi « & » poserait la priorité 1 sur AZERTY.

## Reste à vérifier

IME japonais sous Windows (`ImeOutput` au caret), touches mortes sous Windows, et une
disposition AZERTY complète par `setxkbmap` (sans effet sous ce Xvfb) sur une vraie machine.
