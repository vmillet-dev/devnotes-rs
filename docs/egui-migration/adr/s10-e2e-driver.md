# S10 — Pilote e2e : `egui_inspection` validé sous Linux (xvfb + Mesa)

- **Statut** : accepté pour Linux ; **Windows (WARP) non vérifié** (2026-11-02)
- **Ticket** : #1145 · plan § 8.3, R13
- **Prototype** : `spikes/s10-e2e` — binaire `app` (feature `e2e` = `eframe/inspection`),
  binaire `driver`, test `tests/wgpu_devices.rs`

## Contexte

Tous les scénarios e2e de la migration passent par `egui_inspection` 0.37, crate jeune. S10 doit
le valider sur les runners de CI, ou replier sur un petit canal de contrôle maison.

## Ce qui a été vérifié

`app` : un bouton qui ajoute des cartes, un champ de recherche qui écrit sa valeur dans le profil
sur Entrée, les cartes ; chaque contrôle porte son `data-testid` en `author_id` AccessKit
(`Context::accesskit_node_builder`, § 8.2). `driver` (≈ 150 lignes) : lance l'application avec
`--profile <dossier temporaire>` et `EGUI_INSPECTION=127.0.0.1:<port libre>`, se connecte
(poignée de main `eins` v1, trames MessagePack), et :

| Étape                                                                  | Résultat                                   |
| ---------------------------------------------------------------------- | ------------------------------------------ |
| `GetInfo`, `GetTree`, trouver un nœud par `author_id`                  | ✅                                         |
| cliquer au centre de sa boîte (`ApplyEvents` : pointeur), trois fois    | ✅ trois cartes                            |
| taper « déploiement » (`Event::Text`), presser Entrée (`Event::Key`)   | ✅ valeur écrite dans le profil            |
| `eventually` : relire l'arbre jusqu'à la condition, `Settle` entre deux | ✅ aucune attente de durée                 |
| `GetScreenshot` (800 × 600), `Resize`                                  | ✅                                         |
| tuer et **relancer réellement** sur le même profil                     | ✅ valeur relue                            |

Xvfb, Vulkan logiciel (lavapipe) : scénario complet en 0,67–0,86 s, port ouvert 120–160 ms
après le lancement, **6 exécutions sur 6** ; même scénario en GL logiciel (`WGPU_BACKEND=gl`,
llvmpipe) : ✅.

**Plusieurs appareils `wgpu` dans un processus de test** (`egui_kittest` avec `.wgpu()`, chaque
`Harness` crée son appareil) : deux appareils sur un thread, deux sur deux threads à la fois, et
six tests de rendu en parallèle (fils de test par défaut, 4 vCPU) : ✅ 5 exécutions sur 5 sous
lavapipe (1,1–1,4 s, contre 3,1 s en série), ✅ sous GL.

## Décision

- **Le pilote `egui_inspection` est retenu** ; pas de canal de contrôle maison. Le protocole
  couvre tout ce que le § 8.3 demande ; les deux portes de la saveur `e2e` (`--action` par la
  socket d'instance unique, dialogue fixé par `DEVNOTES_E2E_DIALOG`) restent nécessaires.
- **Règle de concurrence des tests `wgpu`** : aucun verrou global ; les tests de rendu
  (`Harness::render`, `snapshot`) tournent en parallèle comme les autres, un appareil par
  `Harness`. Seul le pilote e2e reste « un scénario par processus », pour son profil.

## Conséquences

- R13 baisse à **faible** pour Linux. La version reste épinglée (protocole v1 : un changement non
  additif en change le numéro, que la poignée de main refuse).
- Une touche s'injecte avec son `physical_key` (`Event::Key { physical_key: Some(…) }`) : les
  scénarios qui pressent un raccourci doivent le remplir, puisque `keys::dispatch` lit la
  position (ADR S5).
- La capture demande une fenêtre rendue : en CI Linux, `xvfb-run` suffit, sans gestionnaire de
  fenêtres.

## Reste à vérifier

Le runner Windows avec WARP (DX12 logiciel) : même `driver`, même test `wgpu_devices` ; c'est la
première tâche de P2 si ce spike ne tourne pas d'abord sur une machine Windows.
