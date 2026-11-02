# S9 — Paquets et mises à jour : `cargo-packager-updater` (D26)

- **Statut** : accepté pour D26 et la signature ; **NSIS et AppImage non construits ici** (voir
  « Reste à vérifier ») (2026-11-02)
- **Ticket** : #1144 · plan § 4.1, § 4.3, D8, D26, D27, R7
- **Prototype** : `spikes/s9-packages` (binaire `devnotes-s9` qui affiche sa version et ses
  ressources ; `--update <latest.json> <clé publique>` vérifie, télécharge, contrôle et installe)

## Contexte

D26 : signé par `cargo-packager` avec la clé actuelle ; client `cargo-packager-updater` s'il lit
notre `latest.json`, installe NSIS en mode passif et remplace l'AppImage ; sinon un client maison.
S9 doit aussi prouver la chaîne de paquets (NSIS par utilisateur, AppImage, deb, rpm) et les
chemins réels d'une installation.

## Ce qui a été vérifié

`cargo-packager` 0.11.8, `cargo-packager-updater` 0.2.3, `cargo-generate-rpm`, conteneur Linux.
**Aucune clé réelle n'a été lue** : une paire de test a été générée par la CLI Tauri
(`tauri signer generate`, avec mot de passe) — le même outil, donc le même format, que le secret
`TAURI_SIGNING_PRIVATE_KEY` — dans un dossier temporaire.

| Point                                                                                                                                                | Résultat                                                                                                                                                |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Signature par `cargo-packager` avec une clé Tauri chiffrée, enveloppée en base64, et son mot de passe (`CARGO_PACKAGER_SIGN_PRIVATE_KEY[_PASSWORD]`) | ✅ `.sig` produit                                                                                                                                       |
| Format des signatures                                                                                                                                | ✅ identique à la CLI Tauri (minisign, 4 lignes, base64) ; les deux sont acceptées par le client                                                        |
| `latest.json` tel que `release.yml` l'écrit aujourd'hui                                                                                              | ❌ refusé : `missing field 'format'`                                                                                                                    |
| Même manifeste avec `"format": "appimage"` / `"nsis"` par plateforme                                                                                 | ✅ lu ; mise à jour 0.1.0 → 0.2.0 trouvée                                                                                                               |
| Remplacement de l'AppImage (`$APPIMAGE`) : téléchargement, vérification, remplacement, relance                                                       | ✅ le fichier installé redémarre en 0.2.0                                                                                                               |
| Fichier altéré d'un octet                                                                                                                            | ✅ « The signature verification failed », l'installé reste en 0.1.0                                                                                     |
| `.deb` par `cargo-packager`                                                                                                                          | ✅ `/usr/bin/<binaire>`, ressources dans `/usr/lib/<binaire>/identity/`, `.desktop` ; installé par `dpkg -i`, ressources trouvées en `../lib/<binaire>` |
| `.rpm` par `cargo-generate-rpm`                                                                                                                      | ✅ mêmes chemins (déclarés dans `[package.metadata.generate-rpm]`)                                                                                      |
| NSIS passif                                                                                                                                          | lu dans le source : arguments `/P /R`, modèle NSIS avec `SkipIfPassive` ; non construit                                                                 |
| AppImage par `cargo-packager`                                                                                                                        | non construit : l'outil télécharge `linuxdeploy` et `AppRun` depuis GitHub et les exécute ; refusé dans ce conteneur                                    |

Le remplacement de l'AppImage a été testé sur l'exécutable lui-même posé à la place d'une
AppImage (`APPIMAGE=…/DevNotes.AppImage`) : c'est exactement ce que fait le client (il renomme le
fichier pointé par `$APPIMAGE`, écrit le nouveau, rend les droits).

## Décision

- **D26 : `cargo-packager-updater`.** Il couvre NSIS passif et le remplacement de l'AppImage,
  vérifie la signature minisign avec la clé publique actuelle, et lit notre manifeste dès qu'il
  porte un champ `format` par plateforme — un ajout permis par D6 (le format de `latest.json` est
  celui que ce client lit). Pas de client maison.
- **Signature** : la clé et le mot de passe actuels servent tels quels à `cargo-packager`
  (variables renommées `CARGO_PACKAGER_SIGN_PRIVATE_KEY` et `_PASSWORD`, mêmes secrets).

## Conséquences

- `release.yml` (P11) ajoute `format: 'nsis'` / `'appimage'` à chaque entrée de `platforms`.
- Le client tire `reqwest` 0.12 (le cœur est en 0.13) : deux versions de `reqwest` dans le
  binaire tant que le client ne suit pas ; à relire avec `dependency-audit`. Il vérifie TLS avec
  les racines du système (`rustls-tls-native-roots`), comme l'updater actuel.
- Le paquet `.deb` prend son nom de `product-name` en kebab-case (« DevNotes » donnerait
  `dev-notes`) : le fixer à `devnotes` ; renseigner le mainteneur (`dpkg` avertit sans).
- `cargo-generate-rpm` n'a pas su lire des champs hérités du workspace (`edition.workspace`) : à
  revérifier avec `version.workspace = true` (D27), sinon les champs sont passés à la ligne de
  commande.
- Chemins du § 4.1 confirmés pour deb et rpm ; AppImage (`$APPDIR/usr/lib/…`) et Windows (à côté
  de l'exécutable) restent à voir sur un paquet réel.

## Reste à vérifier (runners de CI)

Construire NSIS par utilisateur sur le runner Windows et l'AppImage sur le runner Linux (réseau
vers GitHub permis), puis rejouer `--update` contre un `latest.json` de test : installation NSIS
passive puis relance. C'est le test « 1.0.0 → 1.0.1 » du § 4.3, avancé au premier job de paquets.
