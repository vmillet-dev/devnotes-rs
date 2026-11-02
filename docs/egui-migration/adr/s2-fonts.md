# S2 — Polices : fichiers variables, `system_fonts`, seuil SSIM 0,95

- **Statut** : accepté (2026-11-02) ; comparé à Chromium sous Linux, **pas encore à WebView2 sous
  Windows** (la référence de D24)
- **Ticket** : #1137 · plan § 3.7 « Polices », § 5.2, § 5.3 E1/E2/E8, R2
- **Prototype** : `spikes/s2-fonts` — `fonts.py` (TTF depuis `@fontsource-variable` 5.3.0),
  binaire `s2-fonts` (epaint par `egui_kittest` + `wgpu`), `chromium.mjs`, `ssim.py`

## Contexte

Le § 3.7 fixe d'avance trois règles que S2 tranche : fichiers variables si `epaint` applique
l'axe `wght` (sinon instances statiques) ; E8 disparaît si le façonnage couvre l'arabe, l'hébreu
et le devanagari ; repli CJK par la feature `system_fonts` d'eframe si elle charge à la demande,
sinon `fontdb`. Il fixe aussi le seuil SSIM de la comparaison visuelle (départ 0,95).

## Ce qui a été mesuré

16 échantillons (`samples.tsv`) : titre de carte (14 px, 650, tronqué à 180 px), recherche de
l'en-tête, Inter à 400/500/600/650/700, étiquette espacée (11 px, 600, +0,66 px), JetBrains Mono
400 et 700, aperçu de code coupé à 3 rangées, émoji, CJK, arabe, hébreu, devanagari. Chacun rendu
par epaint 0.37 (`TextFormat::coords` = `wght`, `extra_letter_spacing`, `TextWrapping` avec
`max_rows` et `…`) et par Chromium (Playwright, mêmes TTF en `data:`, même CSS), à 100, 125 et
150 %. SSIM sur l'encre des deux images (meilleur alignement à ± 2 px, fenêtre 7 × 7), et rapport
des largeurs d'encre.

| Échantillon            | SSIM 100 % | 125 %   | 150 %   | Largeur egui / Chromium |
| ---------------------- | ---------- | ------- | ------- | ----------------------- |
| Inter 400–700, dont 650 | 0,966–0,974 | 0,976–0,980 | 0,982–0,984 | 1,000 (± 0,6 %)   |
| titre de carte tronqué | 0,974      | 0,978   | 0,981   | 1,000                   |
| recherche, étiquette espacée | 0,966–0,980 | 0,974–0,978 | 0,975–0,981 | 1,000         |
| JetBrains Mono 400/700 | 0,968–0,972 | 0,974–0,983 | 0,978–0,984 | 1,000             |
| aperçu sur 3 rangées   | 0,914      | 0,868   | 0,874   | 1,000                   |
| devanagari             | 0,874      | 0,885   | 0,937   | 1,016                   |
| CJK (police système)   | 0,923      | 0,807   | 0,932   | 1,000                   |
| arabe                  | 0,231      | 0,231   | 0,265   | 1,000                   |
| hébreu                 | 0,324      | 0,304   | 0,253   | 1,016                   |
| émoji                  | 0,332      | 0,349   | 0,370   | 0,962                   |

Planches côte à côte : `spikes/out/s2/sheet-100.png`, `sheet-150.png` ; chiffres :
`spikes/out/s2-ssim.txt`.

Lecture des images :

- l'axe `wght` est appliqué : 650 est rendu à 650 (même SSIM que 600 et 700, largeurs égales) ;
- troncature avec `…`, `max_rows`, interlettrage : mêmes points de coupe que le CSS ;
- **arabe et hébreu** : chaque mot est façonné (lettres liées, bons glyphes), mais les mots
  restent dans l'ordre logique, de gauche à droite — epaint 0.37 n'a pas d'algorithme bidi
  (« once RTL/bidi support is added », `text_layout.rs`) ; **devanagari** : conjonctions et
  matras justes ;
- **émoji** : un glyphe monochrome d'une police système, là où Chromium dessine Noto Color Emoji ;
- **CJK** : `egui_system_font_provider` (la feature `system_fonts` d'eframe) trouve la police à la
  demande, au premier caractère manquant (`FontProvider::font_for`) ;
- **ligatures** : JetBrains Mono dessine `=>` en une ligature dans les deux moteurs ; le CSS de
  l'application ne les coupe que dans les références (`_mixins.scss`).

## Décision

- **Fichiers variables** : on embarque les fichiers variables d'aujourd'hui (Inter, JetBrains
  Mono de `@fontsource-variable`, convertis en TTF par sous-ensemble latin + latin étendu) ; les
  graisses CSS (650 compris) passent telles quelles par `TextFormat::coords`. Pas d'instances
  statiques.
- **Repli** : `system_fonts` d'eframe, qui charge à la demande ; pas de `fontdb`. Noto Emoji
  (monochrome) embarqué avant le repli système, pour que l'émoji ne dépende pas de la machine.
- **Seuil SSIM : 0,95** pour une capture, inchangé ; le texte latin seul est entre 0,966 et 0,984
  à toutes les échelles. Un état qui contient de l'arabe, de l'hébreu ou des émoji est comparé
  hors de ces zones (masque) : ce sont des écarts listés, pas des régressions.
- **E1 confirmé** (rastérisation différente, largeurs à ± 0,6 % : sous la tolérance de ± 2 % du
  § 5.1) ; **E2 confirmé** ; **E8 restreint** à l'ordre bidi de l'arabe et de l'hébreu : le
  façonnage des trois écritures est couvert.

## Conséquences

- R2 baisse à **faible** : graisses, crénage et largeurs suivent Chromium à moins de 1 %.
- Ligatures : `TextFormat` n'expose pas de traits OpenType en 0.37 ; si l'on veut les couper
  partout dans le code (comme les références le font), il faut une police sans `calt`/`liga`
  (instance de JetBrains Mono « NL ») plutôt qu'un réglage : à décider avec l'écran du snippet.
- Comparé ici à Chromium sous Linux (lissage en niveaux de gris) ; la référence de D24 est
  WebView2 sous Windows (ClearType). Le seuil se revérifie au gel de la référence en P1, avec
  `ssim.py` sur les mêmes échantillons.
