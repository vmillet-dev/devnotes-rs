import { DocPage, b, code, h2, h3, keys, link, ol, p, table, tip, ui, ul, warn } from '../doc.model';

export const SAFETY: DocPage = {
  slug: 'securite',
  title: 'Sécurité et sauvegardes',
  summary:
    'Ce qui est chiffré et comment, la phrase de passe, plusieurs bibliothèques, les copies de sauvegarde, la corbeille, les versions et l’annulation.',
  body: [
    h2('ce-qui-sort', 'Ce qui quitte votre machine'),
    p(
      'Rien, sauf ce que vous envoyez. Ni compte, ni cloud, ni télémétrie. Une requête du client API part vers le serveur que vous nommez, et nulle part ailleurs ; les outils n’appellent rien ; la vérification des mises à jour demande seulement à GitHub si une version plus récente existe.',
    ),
    h2('chiffrement', 'Le chiffrement'),
    p(
      'DevNotes demande votre phrase de passe une fois à chaque lancement. Elle ouvre la bibliothèque et n’est enregistrée nulle part : ni trousseau, ni « se souvenir de moi ».',
    ),
    table(
      'Comment une bibliothèque est scellée',
      ['Quoi', 'Comment'],
      [
        b('Scellé sur le disque'),
        'titres, corps, sources et leur historique, items des listes, noms des espaces et des dossiers, valeurs des champs, noms et contenu des pièces jointes',
      ],
      [b('Dérivation de clé'), 'Argon2id (64 Mio, 3 passes) sur un sel aléatoire'],
      [b('Chiffrement'), 'AES-256-GCM, un nonce neuf à chaque écriture ; une valeur altérée est refusée'],
      [
        b('Phrase de passe'),
        'au moins douze caractères ; elle enveloppe la clé de la bibliothèque, elle n’est pas la clé',
      ],
    ),
    h3('ce-qui-reste-lisible', 'Ce qu’un fichier volé laisse voir'),
    p(
      'Ce sur quoi la base filtre et trie n’est pas scellé : le sceller obligerait à tout déchiffrer pour répondre à une recherche. Sans la phrase de passe, on ne lit ni un titre, ni un corps, ni une pièce jointe, mais on voit :',
    ),
    ul(
      ['le nombre de notes et leur répartition en espaces et en dossiers, sans leurs noms ;'],
      ['les dates de création, de modification et d’échéance, et les notes épinglées ;'],
      ['le langage et la sorte de chaque note ;'],
      ['chaque tag et chaque nom de champ, tels que vous les avez tapés ;'],
      ['la longueur de chaque valeur scellée.'],
    ),
    tip(
      'Ne faites donc pas porter le secret par un tag ou un nom de champ : ',
      code('db_password'),
      ' reste lisible, sa valeur ne l’est pas.',
    ),
    h2('phrase-de-passe', 'La phrase de passe'),
    p(
      'Elle se change dans ',
      ui('Préférences… → Sécurité → Changer…'),
      ' : la nouvelle s’applique au prochain lancement. Rien n’est rechiffré, la phrase enveloppe seulement la clé ; les copies de sauvegarde gardées à côté sont réenveloppées avec elle.',
    ),
    warn(
      b('Il n’y a pas de récupération.'),
      ' Une phrase oubliée ne se réinitialise pas. ',
      ui('Phrase de passe oubliée ?'),
      ' propose seulement d’archiver la bibliothèque, toujours scellée, et d’en recommencer une. Gardez un export en clair en lieu sûr si vos notes comptent.',
    ),
    h2('bibliotheques', 'Plusieurs bibliothèques'),
    p(
      ui('Fichier → Bibliothèques…'),
      ' en crée, en renomme, en ouvre ou en supprime. Chacune a sa base, ses pièces jointes, ses copies de sauvegarde et sa propre phrase de passe : rien ne passe de l’une à l’autre. Changer de bibliothèque recharge la fenêtre ; ni celle qui est ouverte, ni la dernière ne peuvent être supprimées.',
    ),
    h2('sauvegardes', 'Les copies de sauvegarde'),
    p(
      'Avec ',
      ui('Copier la bibliothèque au lancement'),
      ' (activé par défaut), DevNotes prend une copie au démarrage, une par jour au plus, et garde les trois dernières : la base, la clé qui l’ouvre et les pièces jointes. Les pièces jointes sont liées plutôt que dupliquées : une copie coûte à peine plus que les notes.',
    ),
    p(
      ui('Préférences… → Sécurité → Restaurer…'),
      ' remplace la bibliothèque entière par une copie. Rien n’est supprimé : l’actuelle est mise de côté, et son emplacement vous est donné.',
    ),
    tip(
      'Ces copies sont rangées à côté de l’original : elles couvrent une corbeille vidée trop vite ou une mise à jour ratée, pas un disque mort. Pour ça, ',
      link('exportez', '/docs/import-export'),
      ' ailleurs.',
    ),
    h2('corbeille', 'La corbeille et Ctrl Z'),
    p(
      'Une note supprimée part à la corbeille et y reste trente jours. Le bandeau qui suit propose ',
      ui('Annuler'),
      ', et ',
      keys('Ctrl Z'),
      ' le fait encore après qu’il a disparu. La ',
      ui('Corbeille'),
      ', en bas de la barre latérale, restaure une note ou la supprime définitivement.',
    ),
    h2('versions', 'Les versions d’un snippet'),
    p(
      'Un snippet garde ses vingt dernières versions. L’',
      ui('Historique'),
      ' de l’éditeur montre ce qui reviendrait et ce qui partirait avant de ',
      ui('Revenir à cette version'),
      '. Les versions ne partent pas dans les exports.',
    ),
    h2('integrite', 'Si la base est abîmée'),
    p(
      'La base est vérifiée à chaque ouverture. Si elle est endommagée, DevNotes le dit au lieu de démarrer dessus, et propose de la mettre de côté avec ce qui peut encore être récupéré. Votre phrase de passe ne change pas.',
    ),
    h2('vos-fichiers', 'Où sont vos fichiers'),
    table(
      'Le dossier de vos bibliothèques',
      ['Système', 'Dossier'],
      ['Windows', code('%APPDATA%\\com.devnotes.app\\')],
      ['Linux', code('~/.local/share/com.devnotes.app/')],
    ),
    p(
      'Chaque bibliothèque vit dans ',
      code('libraries/<id>/'),
      ' : la base, ',
      code('vault.json'),
      ' (la clé que votre phrase ouvre), les pièces jointes et les copies de sauvegarde.',
    ),
    warn(
      'La base et ',
      code('vault.json'),
      ' ne valent qu’ensemble. Copiez l’un sans l’autre, et vous aurez copié quelque chose que personne ne pourra rouvrir.',
    ),
    h2('en-memoire', 'Pendant que DevNotes tourne'),
    ol(
      ['La clé reste en mémoire pendant la session : il n’y a pas de reverrouillage après inactivité.'],
      [
        'Ouvrir une pièce jointe en écrit une copie déchiffrée dans votre profil, effacée quand DevNotes se ferme.',
      ],
      [
        'DevNotes protège vos fichiers au repos (un portable volé, un profil copié), pas une machine déjà compromise pendant qu’il tourne.',
      ],
    ),
  ],
};
