# Ranger et retrouver

Les espaces et leurs dossiers, la vue Tableau, les tags, la recherche, les filtres, le tri et la sélection multiple.

## Les espaces

Chaque note vit dans un espace : « Perso », « Boulot », un client, un projet. La barre latérale les liste ; *Tous les espaces* les montre ensemble. *Nouvel espace* en crée un, et le menu d’un espace l’épingle en tête, le renomme ou le supprime.

Supprimer un espace demande toujours où ses notes doivent aller : elles ne partent jamais avec lui. Il faut donc au moins un second espace pour en supprimer un.

## Les dossiers

Un dossier découpe un espace en régions nommées et colorées : bleu, ambre, violet, vert ou rouge. *Nouveau dossier* en crée un dans l’espace ouvert ; son menu le renomme, change sa couleur ou le supprime, et *Nouveau sous-dossier* en crée un à l’intérieur. Les notes d’un dossier supprimé restent, confiées au dossier du dessus. Un dossier glissé sur un autre entre dedans ; <kbd>Alt ←</kbd> et <kbd>Alt →</kbd> le font au clavier. Glissé sur un autre espace, il y part avec ses notes.

## La vue Tableau

Le sélecteur *Liste* / *Tableau* change la façon de voir un espace. En *Tableau*, chaque dossier est une zone que l’on déplace et redimensionne, et chaque note une carte posée dessus. Le tableau montre un niveau à la fois, et un clic sur une zone ouvre le sien :

- glissez une carte dans une zone pour la ranger dans ce dossier ;
- lâchez-la sur le fond pour la sortir de son dossier ;
- *Aligner* (<kbd>A</kbd>) aligne les cartes libres sous les dossiers, sans toucher aux zones ;
- *Autres rangements*, à côté, propose de replacer et de redimensionner les dossiers et leurs cartes. C’est annulable.

Sans souris : <kbd>X</kbd> coche les cartes, puis *Déplacer vers…* les envoie dans un dossier. Ouvrir un dossier montre ses seules notes ; <kbd>Échap</kbd> en ressort.

## La recherche

<kbd>Ctrl K</kbd> va au champ de recherche. Elle traverse les titres, les corps, les sources, les items des listes et les tags, accents compris : `deploiement` trouve « Déploiement ». Chaque carte trouvée cite la ligne qui correspond.

## Filtres, tags et tri

- **Les filtres rapides** : *Tout*, *Épinglées* et *À trier* (les notes qui ont une échéance).
- **Tags et formats** : le bouton à côté filtre par tag, par langage, par sorte de note ou par priorité.
- **Trier et regrouper** : par date de modification ou de création, par priorité, par format ou par titre ; regroupé par date, par priorité, par format, ou sans groupe. *Épinglées en tête* garde les notes épinglées au-dessus.

Le bouton *Gérer* des tags ouvre *Gérer les tags* : renommer un tag sur toutes ses notes, en fusionner plusieurs, ou les retirer. Renommer vers un tag existant fusionne les deux.

## La sélection multiple

Cochez des cartes avec <kbd>X</kbd>, ou <kbd>Ctrl</kbd> + clic ; <kbd>Shift</kbd> + clic étend la sélection. La barre qui apparaît agit sur toutes à la fois : *Déplacer vers…* un espace ou un dossier, *Priorité*, *+ tag*, *Mettre à la corbeille*. Chaque action dit ce qu’elle a changé, et s’annule exactement.

## Le canevas au clavier

**Sur le canevas**

| Touches | Action |
| --- | --- |
| <kbd>Ctrl K</kbd> | Aller au champ de recherche |
| <kbd>↑ ↓ ← →</kbd> | Se déplacer de carte en carte |
| <kbd>Entrée</kbd> | Ouvrir la carte sous le focus |
| <kbd>C</kbd> | Copier la carte, ou demander ses champs |
| <kbd>P</kbd> | Épingler ou désépingler |
| <kbd>0 … 4</kbd> | Donner une priorité : 0 aucune, 1 basse… 4 urgente |
| <kbd>X</kbd> | Cocher la carte pour une action de masse |
| <kbd>A</kbd> | Aligner les cartes libres (tableau) |
| <kbd>Suppr</kbd> | Mettre à la corbeille, en deux fois |
| <kbd>Ctrl Z</kbd> | Annuler la dernière suppression |
| <kbd>Échap</kbd> | Reprendre la note, la sélection, les filtres, puis sortir du dossier |

Les lettres nues n’agissent que sur le canevas, jamais pendant une saisie. Elles se changent dans *Préférences… → Raccourcis*.
