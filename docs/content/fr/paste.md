# Collage rapide et raccourcis

Retrouver un snippet et le coller sans quitter ce que vous faites, capturer le presse-papiers dans une note, et régler les raccourcis.

## La palette

<kbd>Ctrl Alt P</kbd> ouvre la palette par-dessus n’importe quelle application, même quand la fenêtre de DevNotes est fermée. Elle cherche dans tous vos espaces à la fois, sans tenir compte des filtres du canevas : quand on rappelle un snippet, on ne se souvient pas toujours où on l’a rangé.

1. Tapez quelques lettres du titre, du code ou d’un tag.
2. Choisissez avec les flèches.
3. <kbd>Ctrl C</kbd> copie la note et referme la palette : vous retrouvez l’application d’où vous veniez, prêt à coller.

Un snippet à champs, comme `{{host}}`, demande d’abord ses valeurs : copier une commande à trous telle quelle donnerait une commande inutilisable. <kbd>Entrée</kbd> ou un clic ouvrent la note au lieu de la copier, ce qui fait aussi de la palette un « trouver cette note ».

Elle trouve plus que des notes. Le même champ cherche les requêtes du client API, les outils et les espaces, et lance des commandes ; trois préfixes resserrent la recherche :

**Resserrer la recherche**

| Taper | Pour |
| --- | --- |
| <kbd>@</kbd> | choisir où chercher : les notes, les requêtes, les outils, les espaces, ou les notes d’un espace |
| <kbd>#</kbd> | lister les notes d’un tag |
| <kbd>></kbd> | lancer une commande : nouvelle requête, nouveau dossier, préférences, feuille des raccourcis |

**Dans la palette**

| Touches | Action |
| --- | --- |
| <kbd>↑ ↓</kbd> | Parcourir les résultats (Tab aussi) |
| <kbd>Entrée</kbd> | Ouvrir la note, la requête, l’outil ou l’espace |
| <kbd>Ctrl C</kbd> | Copier la note et fermer |
| <kbd>Échap</kbd> | Fermer la palette |

> [!TIP]
> Rien ne correspond ? La dernière ligne propose *+ Créer une note « … »* avec ce que vous avez tapé.

## Capturer le presse-papiers

<kbd>Ctrl Alt V</kbd> fait l’inverse de la palette : ce que vous venez de copier, où que ce soit, devient une note. <kbd>Ctrl Alt N</kbd> ouvre une note vide. Les deux marchent fenêtre fermée.

## Depuis la barre système

L’icône de DevNotes dans la barre système propose *Palette*, *Nouvelle note* et *Coller le presse-papier*. Par défaut, fermer la fenêtre la range dans la barre système, où DevNotes reste à l’écoute de ses raccourcis (*Fermer dans la barre système*, dans les préférences) ; *Démarrer avec le système* le lance à l’ouverture de session.

## Les raccourcis globaux

**Actifs partout, même fenêtre fermée**

| Par défaut | Action |
| --- | --- |
| <kbd>Ctrl Alt P</kbd> | Ouvrir la palette |
| <kbd>Ctrl Alt V</kbd> | Capturer le presse-papier dans une note |
| <kbd>Ctrl Alt N</kbd> | Créer une note |

Ces trois-là agissent dans toutes vos applications : ils demandent au moins un modificateur, sans quoi la touche serait avalée partout. Le premier arrivé est servi : si une autre application tient déjà une combinaison, DevNotes le signale au démarrage, et vous en choisissez une autre.

## Changer de zone

**Dans les trois zones**

| Touches | Action |
| --- | --- |
| <kbd>Ctrl 1</kbd> | Aller aux notes |
| <kbd>Ctrl 2</kbd> | Aller aux outils |
| <kbd>Ctrl 3</kbd> | Aller au client HTTP |
| <kbd>Ctrl Shift T</kbd> | Chercher un outil |

## Régler ses raccourcis

Dans *Fichier → Préférences… → Raccourcis*, cliquez dans le champ d’une action, puis composez la combinaison. DevNotes prévient si elle est déjà prise par une autre action. *Rétablir* revient à celle d’origine.

- **Globaux :** au moins un modificateur (Ctrl, Alt, Maj ou Super).
- **Canevas :** une lettre nue suffit, puisqu’ils n’agissent jamais pendant une saisie.
- **Fixes :** les flèches, Échap, Tab et les clics avec Ctrl ou Maj ne se déplacent pas. Ce sont la navigation, la sortie et la sélection.

La liste complète des raccourcis, selon l’endroit où vous êtes, est dans *À propos → Raccourcis clavier*. Ceux du canevas sont détaillés dans [Ranger et retrouver](organising.md).
