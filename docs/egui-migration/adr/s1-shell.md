# S1 — Coquille : eframe suffit ; Linux en X11 quand `DISPLAY` existe (D15)

- **Statut** : accepté pour Linux X11 et pour D15 ; **Windows et vrais bureaux Wayland non vérifiés**
  (2026-11-02)
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

## Reste à vérifier (hors de ce conteneur)

Windows 10/11 (DX12, mise à l'échelle 100–200 %, plein écran, tray, raccourci global, tube
nommé), un vrai bureau GNOME et KDE en XWayland (dépôt depuis le gestionnaire de fichiers, tray,
plein écran), le clic de menu du tray, le dialogue avec un portail réel.
