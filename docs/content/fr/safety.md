# Sécurité et sauvegardes

Ce qui est chiffré et comment, la phrase de passe, plusieurs bibliothèques, les copies de sauvegarde, la corbeille, les versions et l’annulation.

## Ce qui quitte votre machine

Rien, sauf ce que vous envoyez. Ni compte, ni cloud, ni télémétrie. Une requête du client API part vers le serveur que vous nommez, et nulle part ailleurs ; les outils n’appellent rien ; la vérification des mises à jour demande seulement à GitHub si une version plus récente existe.

## Le chiffrement

DevNotes demande votre phrase de passe une fois à chaque lancement. Elle ouvre la bibliothèque et n’est enregistrée nulle part : ni trousseau, ni « se souvenir de moi ».

**Comment une bibliothèque est scellée**

| Quoi | Comment |
| --- | --- |
| **Scellé sur le disque** | titres, corps, sources et leur historique, items des listes, noms des espaces et des dossiers, valeurs des champs, noms et contenu des pièces jointes |
| **Dérivation de clé** | Argon2id (64 Mio, 3 passes) sur un sel aléatoire |
| **Chiffrement** | AES-256-GCM, un nonce neuf à chaque écriture ; une valeur altérée est refusée |
| **Phrase de passe** | au moins douze caractères ; elle enveloppe la clé de la bibliothèque, elle n’est pas la clé |

### Ce qu’un fichier volé laisse voir

Ce sur quoi la base filtre et trie n’est pas scellé : le sceller obligerait à tout déchiffrer pour répondre à une recherche. Sans la phrase de passe, on ne lit ni un titre, ni un corps, ni une pièce jointe, mais on voit :

- le nombre de notes et leur répartition en espaces et en dossiers, sans leurs noms ;
- les dates de création, de modification et d’échéance, et les notes épinglées ;
- le langage et la sorte de chaque note ;
- chaque tag et chaque nom de champ, tels que vous les avez tapés ;
- la longueur de chaque valeur scellée.

> [!TIP]
> Ne faites donc pas porter le secret par un tag ou un nom de champ : `db_password` reste lisible, sa valeur ne l’est pas.

## La phrase de passe

Elle se change dans *Préférences… → Sécurité → Changer…* : la nouvelle s’applique au prochain lancement. Rien n’est rechiffré, la phrase enveloppe seulement la clé ; les copies de sauvegarde gardées à côté sont réenveloppées avec elle.

> [!WARNING]
> **Il n’y a pas de récupération.** Une phrase oubliée ne se réinitialise pas. *Phrase de passe oubliée ?* propose seulement d’archiver la bibliothèque, toujours scellée, et d’en recommencer une. Gardez un export en clair en lieu sûr si vos notes comptent.

## Plusieurs bibliothèques

*Fichier → Bibliothèques…* en crée, en renomme, en ouvre ou en supprime. Chacune a sa base, ses pièces jointes, ses copies de sauvegarde et sa propre phrase de passe : rien ne passe de l’une à l’autre. Changer de bibliothèque recharge la fenêtre ; ni celle qui est ouverte, ni la dernière ne peuvent être supprimées.

## Les copies de sauvegarde

Avec *Copier la bibliothèque au lancement* (activé par défaut), DevNotes prend une copie au démarrage, une par jour au plus, et garde les trois dernières : la base, la clé qui l’ouvre et les pièces jointes. Les pièces jointes sont liées plutôt que dupliquées : une copie coûte à peine plus que les notes.

*Préférences… → Sécurité → Restaurer…* remplace la bibliothèque entière par une copie. Rien n’est supprimé : l’actuelle est mise de côté, et son emplacement vous est donné.

> [!TIP]
> Ces copies sont rangées à côté de l’original : elles couvrent une corbeille vidée trop vite ou une mise à jour ratée, pas un disque mort. Pour ça, [exportez](transfer.md) ailleurs.

## La corbeille et Ctrl Z

Une note supprimée part à la corbeille et y reste trente jours. Le bandeau qui suit propose *Annuler*, et <kbd>Ctrl Z</kbd> le fait encore après qu’il a disparu. La *Corbeille*, en bas de la barre latérale, restaure une note ou la supprime définitivement.

## Les versions d’un snippet

Un snippet, comme une Note, garde ses vingt dernières versions. L’*Historique* de l’éditeur montre ce qui reviendrait et ce qui partirait avant de *Revenir à cette version*. Les versions ne partent pas dans les exports.

## Si la base est abîmée

La base est vérifiée à chaque ouverture. Si elle est endommagée, DevNotes le dit au lieu de démarrer dessus, et propose de la mettre de côté avec ce qui peut encore être récupéré. Votre phrase de passe ne change pas.

## Où sont vos fichiers

**Le dossier de vos bibliothèques**

| Système | Dossier |
| --- | --- |
| Windows | `%APPDATA%\com.devnotes.app\` |
| macOS | `~/Library/Application Support/com.devnotes.app/` |
| Linux | `~/.local/share/com.devnotes.app/` |

Chaque bibliothèque vit dans `libraries/<id>/` : la base, `vault.json` (la clé que votre phrase ouvre), les pièces jointes et les copies de sauvegarde.

> [!WARNING]
> La base et `vault.json` ne valent qu’ensemble. Copiez l’un sans l’autre, et vous aurez copié quelque chose que personne ne pourra rouvrir.

## Pendant que DevNotes tourne

1. La clé reste en mémoire pendant la session : il n’y a pas de reverrouillage après inactivité.
2. Ouvrir une pièce jointe en écrit une copie déchiffrée dans votre profil, effacée quand DevNotes se ferme.
3. DevNotes protège vos fichiers au repos (un portable volé, un profil copié), pas une machine déjà compromise pendant qu’il tourne.
