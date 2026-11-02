# S6 — Éditeur riche : faisable ; E15 retiré (sauf Wayland pur)

- **Statut** : accepté (2026-11-02) ; collage HTML vérifié sous Linux X11 et Windows 11
- **Ticket** : #1141 · plan § 3.9.1, § 7 phase 4, R1, E15
- **Prototype** : jetable, non conservé dans le dépôt

## Contexte

L'éditeur WYSIWYG des Notes est le plus gros poste et le plus risqué (R1). S6 doit montrer qu'il
est faisable, affiner l'estimation de P4, et dire si `arboard` lit `text/html` (sinon E15 : le
collage HTML se réduit au texte).

## Ce qui a été vérifié

**Aller-retour Markdown** — corpus : les 21 sorties TipTap attendues par
`rich-text-editor.component.spec.ts` (gras, liens, titres, `&#9;` en tête de ligne, blocs de code
avec et sans langage, images `devnotes:drawing/<id>`, emphase espacée `2 * 3 * 4`, `a *b c* d`,
tâches) et les 4 Notes d'exemple du premier lancement (fr/en).

| Corpus                      | Réécrit octet pour octet | Stable après une passe |
| --------------------------- | ------------------------ | ---------------------- |
| 21 sorties TipTap des specs | **21 / 21**              | 21 / 21                |
| 4 Notes d'exemple           | 0 / 4                    | 4 / 4                  |

Les exemples sont écrits à la main, pas par TipTap : la seule différence est la ligne vide que
l'écriture met entre un titre et la liste qui le suit.
La règle du plan (« le corps n'est réécrit que si l'utilisateur a modifié la Note ») les laisse
intacts.

**Modèle et vue** — paragraphes, titres, listes (puces, numérotées, tâches), citations, code,
tableaux, règle ; marques gras/italique/barré/code/lien ; une galley par bloc. Tests kittest
(3 verts) : ↓/↑ passent de bloc en bloc en gardant l'abscisse du caret, et
parcourent d'abord les rangées d'un paragraphe replié ; → en fin de bloc va au début du suivant,
← au début revient en fin du précédent ; `## ` tapé dans un paragraphe vide le change en titre de
niveau 2 et le caret suit (règles du `StarterKit` : `#`…`######`, `- * +`, `1. `, `[ ] `, `[x] `,
`> `, ` ``` `, `---`, `**gras**`).

**Presse-papiers** (`arboard` 3.7, `Get::html`) :

| Plateforme                        | Résultat                                                                                        |
| --------------------------------- | ----------------------------------------------------------------------------------------------- |
| X11 (Xvfb + `xclip -t text/html`) | ✅ le HTML exact (`<p><b>Gras</b> et <a href=…>lien</a></p><ul>…`)                              |
| Windows (copie depuis Edge)       | ✅ le fragment HTML (`<p><b>Gras</b> et <a href=…>lien</a> – é</p><ul>…`), accents intacts      |
| Windows (`Set-Clipboard -AsHtml`) | ⚠️ le `CF_HTML` brut : en-tête `Version:0.9…StartFragment…`, page entière et `\0` final         |
| Wayland natif                     | seulement par le protocole `wlr-data-control` (feature `wayland-data-control`), absent de GNOME |

## Décision

- **Faisable** dans l'architecture du § 3.9.1 : modèle hors egui, lecture `pulldown-cmark` aux
  options de `notes::markdown`, sérialiseur maison, vue par blocs. Le sérialiseur reproduit déjà
  la sortie TipTap sur tout le corpus disponible.
- **E15 retiré** : avec D15 (X11 dès que `DISPLAY` existe, ADR S1), une session Wayland passe par
  XWayland, dont le presse-papiers est celui de X11 ; le collage HTML garde sa mise en forme. Il ne
  reste réduit au texte que dans une session Wayland sans XWayland (même cas que E12).
- **Estimation de l'éditeur riche : 4 → 4,5 S.** Le prototype couvre vite ce qui était redouté
  (aller-retour, caret entre blocs, règles de saisie), et Mermaid devient l'intégration de `merman`
  (ADR S7) au lieu d'un rendu maison. Ce qu'il ne couvre pas pèse plus que prévu : sélection qui
  traverse des blocs imbriqués, Tab qui imbrique/désimbrique une liste, édition de tableau (Tab
  de cellule, outils), annulation par transactions, IME dans un widget maison, copie d'une
  sélection en Markdown. P4 passe de 12,5 à **13 S**.

## Conséquences

- Le corpus d'or du § 8.6 (D25) doit contenir **la sortie TipTap des Notes d'exemple ouvertes puis
  modifiées**, pas seulement les specs : c'est là que la ligne vide entre blocs se règle.
- Les listes du prototype sont aplaties pour la vue ; la vraie vue garde l'imbrication (une
  position est un chemin de blocs, pas un indice).
- `GetExtLinux` d'`arboard` choisit la sélection (`CLIPBOARD` vs `PRIMARY`) : le collage lit
  `CLIPBOARD`.
- Sous Windows, `arboard` n'extrait le fragment que si les décalages de l'en-tête `CF_HTML` sont
  justes ; sinon il rend le format brut (cas de PowerShell 5.1). Le collage retire donc un en-tête
  `Version:` et ne garde que ce qui est entre `<!--StartFragment-->` et `<!--EndFragment-->`.
- Sous Windows, l'aller-retour et les tests (`cargo test -p s6-rich`) donnent le même résultat
  que sous Linux : 21 / 21 octet pour octet, 25 / 25 stables, 3 tests kittest verts.
