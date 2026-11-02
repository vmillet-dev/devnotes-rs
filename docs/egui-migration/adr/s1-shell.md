# S1 — Coquille : eframe suffit ; Linux en X11 quand `DISPLAY` existe (D15)

- **Statut** : accepté pour Linux X11, Windows 11 et D15 ; **mise à l'échelle > 100 % sous Windows
  et vrais bureaux Wayland non vérifiés** (2026-11-02)
- **Ticket** : #1136 · plan § 3.10, D15, D16, D29, R3
- **Prototype** : jetable, non conservé dans le dépôt

## Contexte

Deux questions : eframe tient-il la coquille (fenêtre créée invisible, cachée vers le tray puis
réveillée par le tray, un raccourci global ou une seconde instance — R3), ou faut-il une boucle
`egui-winit` + `egui-wgpu` maison ? Et D15 : Wayland natif si un fichier déposé depuis le
gestionnaire de fichiers arrive (GNOME et KDE) et si le tray révèle la fenêtre ; sinon X11 forcé
quand `DISPLAY` existe.

## Ce qui a été vérifié

Environnement : conteneur Linux, Xvfb sans gestionnaire de fenêtres, Mesa lavapipe (Vulkan
logiciel), weston 13 sans écran pour Wayland ; eframe 0.37 (winit 0.30.13, wgpu 30).

| Point                                                                   | Résultat                                                                                                            |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Fenêtre créée invisible (`with_visible(false)`), montrée à la 1re frame | ✅ X11 : `IsViewable` après la première frame, pas avant ; 444 ms du lancement (debug, Vulkan logiciel)             |
| Cacher (`Visible(false)`) puis révéler depuis un autre thread           | ✅ X11 : `IsUnMapped` puis `IsViewable` ; `send_viewport_cmd` + `request_repaint` réveillent la boucle cachée       |
| Instance unique par profil (`interprocess`, espace de noms abstrait)    | ✅ seconde instance : se connecte, transmet `reveal`/`hide`/`quit`, sort ; révélation en **163 ms** de bout en bout |
| Raccourci global (`global-hotkey`, Ctrl+Maj+D)                          | ✅ X11 : reçu sur la fenêtre cachée, la révèle                                                                      |
| Tray (`tray-icon` sur un thread GTK unique, D29) à côté de winit        | ✅ construit, aucun conflit avec la boucle winit ; clic de menu non testable (pas d'hôte StatusNotifier)            |
| Mise à l'échelle                                                        | ✅ `WINIT_X11_SCALE_FACTOR=1.5` → `pixels_per_point` 1,5                                                            |
| Plein écran                                                             | ⚠️ commande reçue, mais sans gestionnaire de fenêtres Xvfb ne l'applique pas : non concluant                        |
| Dialogue par le portail XDG (`rfd`, `xdg-portal`) sans portail          | ⚠️ pas de plantage, mais `None` — indiscernable d'une annulation                                                    |
| Wayland natif (weston) : lancement                                      | ✅ fenêtre créée, rendu Vulkan                                                                                      |
| Wayland natif : cacher, créer invisible, position, dépôt de fichiers    | ❌ absents de winit 0.30.13 (voir ci-dessous)                                                                       |

**Le backend Wayland de winit 0.30.13 n'implémente pas** (lecture du source,
`platform_impl/linux/wayland/`) : `set_visible` (« Not possible on Wayland », sans effet),
`outer_position`/`set_outer_position`, ni aucune émission de `DroppedFile`/`HoveredFile` (seuls
X11, Windows et macOS les émettent ; pas de `wl_data_device` côté réception de fichiers).

Clavier (X11, positions AZERTY posées par `xmodmap`) : voir l'ADR S5.

### Windows

Environnement : Windows 11 Pro (26300), Ryzen 7 9800X3D, RTX 5070 Ti, écran 2560 × 1440 à 100 %,
disposition AZERTY fr-FR ; même binaire en release. Le tray est construit sur le thread de winit,
dont la boucle de messages le sert (pas de GTK : `gtk` n'est une dépendance que sous Linux).

| Point                                                             | Résultat                                                                                                        |
| ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Fenêtre créée invisible, montrée à la 1re frame                   | ✅ trouvée invisible à 375 ms, visible à 819 ms du lancement ; première frame à ≈ 440 ms du processus           |
| Instance unique (`interprocess`, tube nommé)                      | ✅ après correction (ci-dessous) ; `hide`, `reveal`, `quit` transmis ; révélation en **108 ms** de bout en bout |
| Raccourci global Ctrl+Maj+D sur la fenêtre cachée                 | ✅ révélée en 70 ms                                                                                             |
| Fermeture (`WM_CLOSE`) vers le tray, puis clic gauche sur le tray | ✅ cachée, processus vivant ; révélée en 44 ms                                                                  |
| Plein écran, puis fenêtré                                         | ✅ 0,0 → 2560 × 1440, puis retour à 916 × 639                                                                   |
| Dépôt d'un fichier (vrai glisser à la souris, OLE)                | ✅ `dropped "…\dropped note.md"`, effet `Copy`                                                                  |
| Dialogue natif (`rfd`, `IFileDialog`)                             | ✅ Échap → `None` ; chemin tapé + Entrée → `Some(chemin)`                                                       |
| Backend                                                           | Vulkan (et non DX12) : wgpu préfère Vulkan quand le pilote l'offre                                              |

**Piège** : quand le tube nommé existe déjà, Windows répond `ERROR_ACCESS_DENIED`
(`ErrorKind::PermissionDenied`), pas `AddrInUse` ; la seconde instance paniquait au lieu de se
connecter. `platform` doit lire les deux comme « une instance tourne déjà ».

Le clic sur le tray est le message que l'Explorateur poste à la fenêtre cachée de `tray-icon`
(`WM_USER_TRAYICON` + `WM_LBUTTONUP`), pas un clic sur l'icône elle-même.

## Décision

- **eframe suffit** : la fenêtre cachée se réveille depuis un autre thread par la commande de
  viewport et `request_repaint` (le proxy de la boucle) ; aucune boucle winit maison. R3 baisse.
- **D15 : X11 forcé (XWayland) quand `DISPLAY` existe, Wayland natif à défaut.** Le critère ne
  peut pas être tenu sous Wayland natif quel que soit le bureau : winit 0.30 n'y livre pas les
  dépôts de fichiers, et n'y sait pas cacher la fenêtre (fermeture vers le tray). Une constante de
  `app`, appliquée par `NativeOptions::event_loop_builder` (`EventLoopBuilderExtX11::with_x11`) ;
  winit 0.30 n'a plus de variable d'environnement pour cela.

## Conséquences

- E12 (position non restaurée sous Wayland) ne vaut que pour une session Wayland sans XWayland.
  Dans ce cas, fermer vers le tray **minimise** au lieu de cacher, et le dépôt depuis
  l'explorateur est absent : à documenter, pas à contourner.
- D15 est à rouvrir quand eframe passera à une version de winit qui implémente le DnD et la
  visibilité sous Wayland (pas avant la 1.0.0 : version épinglée, R11).
- Le dialogue de fichiers doit savoir s'il y a un portail **avant** de l'ouvrir (présence de
  `org.freedesktop.portal.Desktop` sur le bus de session) pour répondre une erreur nommée au lieu
  d'une annulation silencieuse.
- Sous Windows, la seconde instance reconnaît `PermissionDenied` comme `AddrInUse` (ci-dessus).

## Reste à vérifier

Windows : mise à l'échelle 125–200 % (un réglage système à changer à la main), Windows 10, DX12
forcé (`WGPU_BACKEND=dx12`), un clic réel sur l'icône du tray et son menu. Linux : un vrai bureau
GNOME et KDE en XWayland (dépôt depuis le gestionnaire de fichiers, tray, plein écran), le clic de
menu du tray, le dialogue avec un portail réel.
