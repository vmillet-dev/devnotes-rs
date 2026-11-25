# Prise en main

Installer DevNotes, choisir la phrase de passe de votre bibliothèque, et faire le tour des notes d’exemple du premier lancement.

## Installer

DevNotes fonctionne sous Windows et sous Linux. Tous les fichiers sont sur [la page de téléchargement](https://devnotes.fr/telecharger), chacun en lien direct : prenez celui qui correspond à votre système. Il n’y a aucun moteur de navigateur à l’intérieur, donc rien d’autre à installer.

**Les fichiers de chaque version**

| Système | Fichier | À prendre si |
| --- | --- | --- |
| Windows | `…_x64-setup.exe` | vous voulez simplement l’installer : c’est celui-là |
| Windows | `…-windows.exe` | vous ne voulez aucune installation |
| Linux | `…_x86_64.AppImage` | toutes les distributions, rien à installer |
| Linux | `…_amd64.deb` | Debian, Ubuntu et dérivées |
| Linux | `…x86_64.rpm` | Fedora, RHEL et dérivées |
| Linux | `…-linux` | l’exécutable seul |

L’installateur Windows s’installe pour votre compte, sans droits administrateur. Sous Linux, une AppImage se rend exécutable avant d’être lancée, et un paquet s’installe avec le gestionnaire de votre distribution :

**Installer sous Linux**

```sh
chmod +x devnotes_*_x86_64.AppImage       # puis lancez-la
sudo apt install ./devnotes_*_amd64.deb   # Debian et Ubuntu
sudo dnf install ./devnotes-*.x86_64.rpm  # Fedora et RHEL
```

> [!TIP]
> Windows affiche un avertissement SmartScreen au premier lancement : choisissez **Informations complémentaires**, puis **Exécuter quand même**. Il porte sur l’identité de l’éditeur, pas sur le fichier : DevNotes n’a pas de certificat de signature de code. Ses mises à jour, elles, sont signées et vérifiées avant d’être installées.

## Choisir sa phrase de passe

Au premier lancement, DevNotes vous demande de *Protéger cette bibliothèque* : choisissez une phrase de passe d’au moins douze caractères, et tapez-la deux fois. Elle vous sera demandée une fois à chaque lancement, et n’est enregistrée nulle part.

Plusieurs mots sans rapport entre eux tiennent bien mieux qu’un seul mot astucieux suivi d’un chiffre : `lampe vélo orage pivoine` est plus long et plus facile à retenir que `P@ssw0rd2026`.

> [!WARNING]
> **Il n’y a pas de récupération.** Ni compte, ni réinitialisation : une phrase de passe perdue, c’est une bibliothèque perdue. Un export en clair est la seule copie qui n’en dépend pas. Voir [Sécurité et sauvegardes](safety.md).

## Le premier tour

Une bibliothèque neuve contient un espace *Découverte* avec quelques notes d’exemple : un mot de bienvenue, un snippet *Connexion psql* qui a des champs à remplir, une réponse d’API à ouvrir en graphe, une Note *Mise en production*, un code à formater avec Prettier et une liste *À essayer* qui vous fait faire le tour :

1. Ouvrir une note en cliquant sa carte.
2. Copier *Connexion psql* et remplir ses champs.
3. Ouvrir la réponse d’API en graphe.
4. Chercher un mot dans *Mise en production* avec <kbd>Ctrl F</kbd>.
5. Formater le code à formater avec <kbd>Shift Alt F</kbd>.
6. Ouvrir les outils (<kbd>Ctrl 2</kbd>) et enregistrer un résultat comme note.
7. Coller une note depuis une autre application avec <kbd>Ctrl Alt P</kbd>.
8. Passer en vue *Tableau* et glisser une note dans une zone.
9. Convertir cette liste en Note, depuis le menu *Type* de l’éditeur.

Cochez-la depuis la carte au fur et à mesure. Les notes d’exemple se suppriment comme les autres une fois lues.

## Trois zones

La barre de titre passe d’une zone à l’autre :

- **Notes** (<kbd>Ctrl 1</kbd>) : vos snippets, notes et listes, dans leurs espaces et leurs dossiers.
- **Outils** (<kbd>Ctrl 2</kbd>) : une trentaine d’utilitaires hors ligne.
- **HTTP** (<kbd>Ctrl 3</kbd>) : le client API, ses collections et ses environnements.

## Les mises à jour

DevNotes vérifie lui-même s’il existe une version plus récente et vous la propose, avec ses notes de version : *Installer maintenant*, *Plus tard* ou *Ne plus me proposer la version …*. Le menu *À propos* propose aussi de *Rechercher les mises à jour*. Rien d’autre ne quitte votre machine : la vérification demande seulement à GitHub si une nouvelle version existe. L’installateur Windows et l’AppImage installent la mise à jour eux-mêmes, signée et vérifiée d’abord ; un .deb ou un .rpm est mis à jour par votre gestionnaire de paquets.
