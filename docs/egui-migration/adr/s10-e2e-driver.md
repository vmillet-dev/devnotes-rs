# S10 — Pilote e2e : `egui_inspection` validé sous Linux (xvfb + Mesa) et Windows (WARP)

- **Statut** : accepté pour Linux et Windows (2026-11-02)
- **Ticket** : #1145 · plan § 8.3, R13
- **Prototype** : jetable, non conservé dans le dépôt

## Contexte

Tous les scénarios e2e de la migration passent par `egui_inspection` 0.37, crate jeune. S10 doit
le valider sur les runners de CI, ou replier sur un petit canal de contrôle maison.

## Ce qui a été vérifié

`app` : un bouton qui ajoute des cartes, un champ de recherche qui écrit sa valeur dans le profil
sur Entrée, les cartes ; chaque contrôle porte son `data-testid` en `author_id` AccessKit
(`Context::accesskit_node_builder`, § 8.2). `driver` (≈ 150 lignes) : lance l'application avec
`--profile <dossier temporaire>` et `EGUI_INSPECTION=127.0.0.1:<port libre>`, se connecte
(poignée de main `eins` v1, trames MessagePack), et :

| Étape                                                                   | Résultat                        |
| ----------------------------------------------------------------------- | ------------------------------- |
| `GetInfo`, `GetTree`, trouver un nœud par `author_id`                   | ✅                              |
| cliquer au centre de sa boîte (`ApplyEvents` : pointeur), trois fois    | ✅ trois cartes                 |
| taper « déploiement » (`Event::Text`), presser Entrée (`Event::Key`)    | ✅ valeur écrite dans le profil |
| `eventually` : relire l'arbre jusqu'à la condition, `Settle` entre deux | ✅ aucune attente de durée      |
| `GetScreenshot` (800 × 600), `Resize`                                   | ✅                              |
| tuer et **relancer réellement** sur le même profil                      | ✅ valeur relue                 |

Xvfb, Vulkan logiciel (lavapipe) : scénario complet en 0,67–0,86 s, port ouvert 120–160 ms
après le lancement, **6 exécutions sur 6** ; même scénario en GL logiciel (`WGPU_BACKEND=gl`,
llvmpipe) : ✅.

**Plusieurs appareils `wgpu` dans un processus de test** (`egui_kittest` avec `.wgpu()`, chaque
`Harness` crée son appareil) : deux appareils sur un thread, deux sur deux threads à la fois, et
six tests de rendu en parallèle (fils de test par défaut, 4 vCPU) : ✅ 5 exécutions sur 5 sous
lavapipe (1,1–1,4 s, contre 3,1 s en série), ✅ sous GL.

### Windows

Windows 11, RTX 5070 Ti. `app` prend `S10_SOFTWARE=1` pour n'accepter qu'un adaptateur CPU (WARP,
« Microsoft Basic Render Driver », DX12), comme sur un runner sans GPU.

| Configuration                    | Résultat                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------- |
| GPU (Vulkan, choisi par défaut)  | ✅ 4 exécutions sur 4, scénario en 1,3–2,8 s                                    |
| WARP (DX12 logiciel)             | ✅ **6 exécutions sur 6**, scénario en 1,8–1,9 s ; capture juste                |
| `wgpu_devices` (WARP, parallèle) | ✅ 6 sur 6, 0,8–1,0 s (3,1 s en série) ; ✅ aussi `WGPU_BACKEND=dx12`, `vulkan` |

Le port s'ouvre **≈ 510 ms** après le lancement (120–160 ms sous Linux) : création de la fenêtre
et énumération des adaptateurs. `egui_kittest` choisit de lui-même un adaptateur CPU : WARP dès
que DX12 est là.

## Décision

- **Le pilote `egui_inspection` est retenu** ; pas de canal de contrôle maison. Le protocole
  couvre tout ce que le § 8.3 demande ; les deux portes de la saveur `e2e` (`--action` par la
  socket d'instance unique, dialogue fixé par `DEVNOTES_E2E_DIALOG`) restent nécessaires.
- **Règle de concurrence des tests `wgpu`** : aucun verrou global ; les tests de rendu
  (`Harness::render`, `snapshot`) tournent en parallèle comme les autres, un appareil par
  `Harness`. Seul le pilote e2e reste « un scénario par processus », pour son profil.

## Conséquences

- R13 baisse à **faible** pour Linux et Windows. La version reste épinglée (protocole v1 : un changement non
  additif en change le numéro, que la poignée de main refuse).
- Une touche s'injecte avec son `physical_key` (`Event::Key { physical_key: Some(…) }`) : les
  scénarios qui pressent un raccourci doivent le remplir, puisque `keys::dispatch` lit la
  position (ADR S5).
- La capture demande une fenêtre rendue : en CI Linux, `xvfb-run` suffit, sans gestionnaire de
  fenêtres.
- La saveur `e2e` force l'adaptateur logiciel par `native_adapter_selector` : eframe ne lit pas de
  variable d'environnement pour choisir l'adaptateur, et un runner Windows peut offrir Vulkan.

## Reste à vérifier

Le runner Windows de GitHub lui-même (session non interactive) : même `driver` avec
`S10_SOFTWARE=1`, même test `wgpu_devices` ; première tâche de P2.
