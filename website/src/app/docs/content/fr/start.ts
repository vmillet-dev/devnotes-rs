import { DocPage, b, code, h2, keys, link, ol, p, snippet, table, tip, ui, ul, warn } from '../../doc.model';

export const OVERVIEW: DocPage = {
  id: 'overview',
  title: 'Documentation',
  summary:
    'Comment se servir de DevNotes au quotidien : écrire et ranger ses notes, coller un snippet depuis n’importe où, les outils, le client API et la sécurité de vos données.',
  body: [
    p(
      'DevNotes garde vos snippets, vos notes et vos listes dans une bibliothèque chiffrée sur votre machine, avec une trentaine d’outils hors ligne et un client API. Ces pages expliquent comment s’en servir, dans l’ordre où on le découvre. Si vous débutez, commencez par ',
      link('la prise en main', '/docs/prise-en-main'),
      '.',
    ),
    { kind: 'guides' },
    h2('dans-l-application', 'Dans l’application'),
    p(
      'Le menu ',
      ui('À propos'),
      ' de la barre de titre ouvre un guide de ',
      ui('Prise en main'),
      ', la liste des ',
      ui('Raccourcis clavier'),
      ' actifs selon l’endroit où vous êtes, et les ',
      ui('Nouveautés'),
      ' de chaque version. Les raccourcis se règlent dans ',
      ui('Fichier → Préférences… → Raccourcis'),
      '.',
    ),
  ],
};

export const GETTING_STARTED: DocPage = {
  id: 'start',
  title: 'Prise en main',
  summary:
    'Installer DevNotes, choisir la phrase de passe de votre bibliothèque, et faire le tour des notes d’exemple du premier lancement.',
  body: [
    h2('installer', 'Installer'),
    p(
      'DevNotes fonctionne sous Windows et sous Linux. Tous les fichiers sont sur ',
      link('la page de téléchargement', '/telecharger'),
      ', chacun en lien direct : prenez celui qui correspond à votre système.',
    ),
    table(
      'Les fichiers de chaque version',
      ['Système', 'Fichier', 'À prendre si'],
      ['Windows', code('…_x64-setup.exe'), 'vous voulez simplement l’installer : c’est celui-là'],
      ['Windows', code('…_x64_en-US.msi'), 'vous déployez par stratégie de groupe'],
      ['Windows', code('…-windows.exe'), 'vous ne voulez aucune installation'],
      ['Linux', code('…_amd64.AppImage'), 'toutes les distributions, rien à installer'],
      ['Linux', code('…_amd64.deb'), 'Debian, Ubuntu et dérivées'],
      ['Linux', code('…x86_64.rpm'), 'Fedora, RHEL et dérivées'],
      ['Linux', code('…-linux'), 'l’exécutable seul'],
    ),
    p(
      'L’installateur Windows s’installe pour votre compte, sans droits administrateur. Sous Linux, une AppImage se rend exécutable avant d’être lancée, et un paquet s’installe avec le gestionnaire de votre distribution :',
    ),
    snippet(
      'Installer sous Linux',
      'sh',
      'chmod +x devnotes_*_amd64.AppImage        # puis lancez-la\nsudo apt install ./devnotes_*_amd64.deb   # Debian et Ubuntu\nsudo dnf install ./devnotes-*.x86_64.rpm  # Fedora et RHEL',
    ),
    tip(
      'Windows affiche un avertissement SmartScreen au premier lancement : choisissez ',
      ui('Informations complémentaires'),
      ', puis ',
      ui('Exécuter quand même'),
      '. Il porte sur l’identité de l’éditeur, pas sur le fichier : DevNotes n’a pas de certificat de signature de code. Ses mises à jour, elles, sont signées et vérifiées avant d’être installées.',
    ),
    h2('phrase-de-passe', 'Choisir sa phrase de passe'),
    p(
      'Au premier lancement, DevNotes vous demande de ',
      ui('Protéger cette bibliothèque'),
      ' : choisissez une phrase de passe d’au moins douze caractères, et tapez-la deux fois. Elle vous sera demandée une fois à chaque lancement, et n’est enregistrée nulle part.',
    ),
    p(
      'Plusieurs mots sans rapport entre eux tiennent bien mieux qu’un seul mot astucieux suivi d’un chiffre : ',
      code('lampe vélo orage pivoine'),
      ' est plus long et plus facile à retenir que ',
      code('P@ssw0rd2026'),
      '.',
    ),
    warn(
      b('Il n’y a pas de récupération.'),
      ' Ni compte, ni réinitialisation : une phrase de passe perdue, c’est une bibliothèque perdue. Un export en clair est la seule copie qui n’en dépend pas. Voir ',
      link('Sécurité et sauvegardes', '/docs/securite'),
      '.',
    ),
    h2('premier-tour', 'Le premier tour'),
    p(
      'Une bibliothèque neuve contient un espace ',
      ui('Découverte'),
      ' avec quelques notes d’exemple : un mot de bienvenue, un snippet ',
      ui('Connexion psql'),
      ' qui a des champs à remplir, une note de code Angular et une liste ',
      ui('À essayer'),
      ' qui vous fait faire le tour :',
    ),
    ol(
      ['Ouvrir une note en cliquant sa carte.'],
      ['Copier une note sans l’ouvrir, depuis le bouton de la carte.'],
      ['Copier ', ui('Connexion psql'), ' et remplir ses champs.'],
      ['Ouvrir la palette de collage rapide avec ', keys('Ctrl Alt P'), '.'],
      ['Créer un espace à soi et y déplacer une note.'],
      ['Passer en vue ', ui('Tableau'), ' et glisser une note dans une zone.'],
    ),
    p(
      'Cochez-la depuis la carte au fur et à mesure. Les notes d’exemple se suppriment comme les autres une fois lues.',
    ),
    h2('trois-zones', 'Trois zones'),
    p('La barre de titre passe d’une zone à l’autre :'),
    ul(
      [
        b('Notes'),
        ' (',
        keys('Ctrl 1'),
        ') : vos snippets, notes et listes, dans leurs espaces et leurs dossiers.',
      ],
      [b('Outils'), ' (', keys('Ctrl 2'), ') : une trentaine d’utilitaires hors ligne.'],
      [b('HTTP'), ' (', keys('Ctrl 3'), ') : le client API, ses collections et ses environnements.'],
    ),
    h2('mises-a-jour', 'Les mises à jour'),
    p(
      'DevNotes vérifie lui-même s’il existe une version plus récente et vous la propose, avec ses notes de version : ',
      ui('Installer maintenant'),
      ', ',
      ui('Plus tard'),
      ' ou ',
      ui('Ne plus me proposer la version…'),
      '. Le menu ',
      ui('À propos'),
      ' propose aussi de ',
      ui('Rechercher les mises à jour'),
      '. Rien d’autre ne quitte votre machine : la vérification demande seulement à GitHub si une nouvelle version existe.',
    ),
  ],
};
