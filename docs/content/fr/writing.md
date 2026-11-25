# Écrire

Les cinq sortes de notes, les champs à remplir d’un snippet, l’éditeur de code et Prettier, les notes mises en forme et leur présentation, les feuilles de calcul, les dessins, les listes à cocher et les pièces jointes.

## Cinq sortes de notes

Le bouton *+ Nouvelle note* les propose toutes. Chacune a aussi son raccourci, actif sur le canevas :

**Créer une note**

| Sorte | Pour | Raccourci |
| --- | --- | --- |
| **Snippet de code** | un bloc de code, son langage et ses champs à remplir | <kbd>Ctrl N</kbd> |
| **Note** | du texte mis en forme : titres, listes, liens, tableaux, blocs de code | <kbd>Ctrl Shift N</kbd> |
| **Liste de tâches** | des étapes à cocher, dans l’ordre | <kbd>Ctrl Shift L</kbd> |
| **Calcul** | une ligne, un résultat : unités, dates, totaux | <kbd>Ctrl Shift M</kbd> |
| **Dessin** | des boîtes, des flèches et du texte : un schéma | <kbd>Ctrl Shift D</kbd> |

Le menu *Type* de l’éditeur transforme une note en une autre sorte, un dessin excepté.

Une note porte un titre, une source (le *Contexte…* d’où elle vient : un outil, une doc, un projet) et des tags. Elle n’est écrite sur le disque qu’une fois qu’elle contient quelque chose : une note ouverte puis refermée vide ne laisse rien derrière elle. <kbd>Échap</kbd> referme l’éditeur en enregistrant.

> [!TIP]
> La sorte proposée par défaut se choisit dans *Préférences… → Général → Type de note par défaut*.

## Les snippets

Un snippet est coloré selon son langage : 142 au choix, de JSON, TypeScript, Python, Rust, Go ou SQL à Shell, PowerShell, YAML, TOML ou Elixir, plus le texte brut. Les couleurs sont les mêmes sur la carte que dans l’éditeur. Le menu du langage se filtre au clavier : tapez les premières lettres, puis <kbd>Entrée</kbd>.

L’éditeur de code numérote les lignes, garde <kbd>Tab</kbd> dans le texte pour indenter, et cherche ou remplace avec <kbd>Ctrl F</kbd>. Le retour à la ligne automatique se règle dans les préférences.

### Formater avec Prettier

Le bouton *Formater* (<kbd>Shift Alt F</kbd>) passe le snippet dans Prettier. Un bandeau dit combien de lignes ont changé, et *Annuler* revient en arrière.

- **Formatés :** JavaScript, TypeScript, JSON, CSS, SCSS, HTML, Markdown, YAML, GraphQL.
- **Pas par Prettier :** SQL, Python, Rust, Go, Java, C#, PHP, C, Shell, TOML, XML. Le bouton y est désactivé et dit pourquoi.

La largeur de ligne, l’indentation, les guillemets, les points-virgules et les virgules finales se règlent une fois pour toute la bibliothèque, comme le formatage à l’enregistrement. Un champ `{{host}}` traverse le formatage intact ; si Prettier devait en abîmer un, le texte n’est pas touché.

### Lire un JSON en graphe

Un snippet JSON, YAML, TOML ou XML s’ouvre aussi en *Graphe* ou en *Arbre*. *Côte à côte* garde le code d’un côté et le graphe de l’autre : sélectionner un nœud surligne ses lignes. Dans le graphe, les flèches se déplacent de nœud en nœud, <kbd>Entrée</kbd> ouvre ou ferme une entrée, et <kbd>Shift →</kbd> déplie tout.

## Les champs à remplir

Écrivez un nom entre doubles accolades dans un snippet, et sa valeur vous sera demandée au moment de copier. Un nom se compose de lettres, de chiffres, de tirets et de soulignés. Une valeur proposée s’écrit après un signe égal :

**Un snippet à trois champs**

```sh
ssh -p {{port=22}} {{user}}@{{host}}
```

Copiez-le depuis sa carte (le bouton, ou <kbd>C</kbd> sur la carte qui a le focus) : DevNotes demande *Remplir les champs*, montre l’aperçu de ce qui part dans le presse-papiers, puis propose *Copier rempli* ou *Copier tel quel*. Un champ laissé vide garde la valeur proposée par le texte.

Ce que vous saisissez reste avec la note, pour la fois suivante. Pour une valeur valable partout, ajoutez une variable dans *Préférences… → Variables* : elle est proposée à chaque snippet qui a un champ de ce nom, et ce qui est saisi sur une note passe devant.

> [!TIP]
> La palette de collage rapide pose les mêmes questions : un snippet à champs se colle depuis n’importe quelle application. Voir [Collage rapide](paste.md).

## Les notes mises en forme

Une note s’écrit directement mise en forme, et s’enregistre en Markdown. La barre d’outils et les raccourcis couvrent l’essentiel ; un bloc de code collé garde son langage.

**Mettre en forme une note**

| Touches | Action |
| --- | --- |
| <kbd>Ctrl Alt 1 … 3</kbd> | Titre 1, 2 ou 3 |
| <kbd>Ctrl B</kbd> | Gras |
| <kbd>Ctrl I</kbd> | Italique |
| <kbd>Ctrl Shift S</kbd> | Barré |
| <kbd>Ctrl E</kbd> | Code en ligne |
| <kbd>Ctrl Alt C</kbd> | Bloc de code |
| <kbd>Ctrl Shift 8</kbd> | Liste à puces |
| <kbd>Ctrl Shift 7</kbd> | Liste numérotée |
| <kbd>Ctrl Shift 9</kbd> | Liste de tâches |
| <kbd>Ctrl Shift B</kbd> | Citation |
| <kbd>Ctrl K</kbd> | Lien (Ctrl + clic l’ouvre) |

Un tableau s’insère depuis la barre d’outils, puis s’agrandit ligne par ligne ou colonne par colonne.

### Présenter une note

*Présenter* montre la Note en plein écran, une section par diapositive : les flèches les parcourent, <kbd>M</kbd> ouvre la carte des titres et <kbd>Échap</kbd> quitte.

## Les listes de tâches

Une liste se coche depuis sa carte, sans l’ouvrir, et la carte montre son avancement. Dans l’éditeur, <kbd>Entrée</kbd> ajoute un item sous celui-ci, <kbd>Retour arrière</kbd> retire un item vide et <kbd>Alt ↑</kbd> ou <kbd>Alt ↓</kbd> le déplace.

## Les feuilles de calcul

Une feuille de calcul donne un résultat par ligne, pendant que vous tapez, et n’en garde aucun : seul ce que vous avez écrit est enregistré. Nommez une valeur, convertissez des unités, prenez un pourcentage, additionnez des durées :

**Quelques lignes**

```
prix = 120 €
3 h 20 min en min
20 % de 340
```

Un nom est utilisable sur les lignes qui suivent la sienne. La feuille lit les unités (CSS et fréquences comprises), les dates, toutes les formes de pourcentage, les fractions, les arrondis et les fonctions, les bases et les bits, et la finance : intérêts composés, rendements, valeur actuelle et prêts. Hors ligne, elle ne connaît aucun taux de change, et le dit. Une ligne qui ne se calcule pas dit pourquoi, à côté d’elle.

Cliquez un résultat pour le sélectionner, <kbd>Shift</kbd> ou <kbd>Ctrl</kbd> pour en sélectionner d’autres : la feuille les additionne. Le format des nombres, celui des dates et les décimales (jusqu’à quatorze) se règlent dans *Préférences… → Général*.

## Les dessins

Un dessin, ce sont des boîtes, des flèches et du texte sur une surface libre. Une flèche commencée sur une forme la suit quand la forme bouge ; lâchée sur le milieu d’un côté, elle y reste attachée. Ses deux bouts se reprennent ensuite. <kbd>A</kbd> trace une flèche et <kbd>Suppr</kbd> retire la sélection.

<kbd>Ctrl</kbd> et la molette agrandissent ou réduisent le dessin, la molette seule le fait défiler, et <kbd>F</kbd> le montre en entier.

Un dessin s’exporte (*Exporter en PNG* ou *Exporter en SVG*), et s’insère dans une Note avec *Insérer un dessin* : la Note en montre l’image, dont les coins règlent la taille.

## Les pièces jointes

Pendant qu’une note est ouverte, déposez un fichier sur la fenêtre, utilisez *+ Fichier*, ou collez une image : elle est chiffrée et rattachée à la note. Un fichier peut peser jusqu’à 10 Mo. La carte en affiche le compte ; un aperçu s’agrandit au clic, et chaque pièce s’ouvre ou s’enregistre ailleurs.

## Épingler, prioriser, dater

- **Épingler** (<kbd>P</kbd> sur une carte) met une note en tête de son groupe.
- **La priorité** va de `0` (aucune) à `4` (urgente) : tapez le chiffre sur une carte.
- **Une échéance**, posée depuis le champ de date de l’éditeur, fait d’une note une note « à trier » : le filtre *À trier* les retrouve avant qu’elles ne s’oublient.

## Copier, dupliquer, supprimer

1. Le bouton de copie d’une carte met la note dans le presse-papiers sans l’ouvrir.
2. Le menu **⋯** d’une carte la duplique, ou l’envoie dans un espace ou un dossier avec *Déplacer vers…*
3. <kbd>Suppr</kbd> sur une carte la met à la corbeille en deux temps : une première fois pour armer, une seconde pour confirmer. <kbd>Ctrl Z</kbd> l’annule ensuite.
