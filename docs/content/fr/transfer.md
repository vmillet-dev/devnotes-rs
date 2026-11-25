# Importer et exporter

Exporter tout, un espace ou une sélection dans une archive .devnotes, protégée ou non, la réimporter sans doublon, et copier des notes en Markdown.

## Exporter

Le menu *Fichier* exporte au choix :

- *Exporter tout…* : toute la bibliothèque ;
- *Exporter l’espace actif…* : l’espace ouvert ;
- *Exporter la sélection…* : les notes cochées.

L’export est une archive `.devnotes` : les notes, leurs espaces et leurs pièces jointes. Les versions des snippets n’en font pas partie.

### Protéger un export

Avant d’écrire le fichier, DevNotes demande *Protéger cet export ?* Une phrase de passe le scelle avec une clé qui lui est propre et qui voyage avec le fichier, pas avec cette machine : celui à qui vous donnez la phrase pourra l’ouvrir, personne d’autre. *Exporter en clair* écrit toutes les notes et toutes les pièces jointes lisibles par quiconque détient le fichier.

> [!TIP]
> Un export en clair, rangé en lieu sûr, est la seule copie qui ne dépend pas de votre phrase de passe. Voir [Sécurité et sauvegardes](safety.md).

## Importer

*Fichier → Importer…* lit une archive `.devnotes` (ou un ancien export `.json`). Une archive protégée demande sa phrase de passe d’abord.

- Un import fusionne, il ne remplace jamais : les espaces sont rapprochés par leur nom.
- Une note déjà présente est comptée comme ignorée, pas écrasée : réimporter le même fichier ne crée aucun doublon.
- DevNotes dit toujours ce qu’il a importé, même quand c’est zéro note.

## Copier en Markdown

*Fichier → Copier la sélection en Markdown* met les notes cochées dans le presse-papiers : pour chacune, son titre, son espace, son contexte, ses tags, puis son contenu dans un bloc de code. De quoi coller une procédure dans un ticket ou un wiki.
