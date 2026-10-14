import { DocPage, b, field, h2, keys, link, ol, p, shortcuts, table, tip, ui, ul } from '../doc.model';

export const QUICK_PASTE: DocPage = {
  slug: 'collage-rapide',
  title: 'Collage rapide et raccourcis',
  summary:
    'Retrouver un snippet et le coller sans quitter ce que vous faites, capturer le presse-papiers dans une note, et régler les raccourcis.',
  body: [
    h2('palette', 'La palette de collage rapide'),
    p(
      keys('Ctrl Alt P'),
      ' ouvre la palette par-dessus n’importe quelle application, même quand la fenêtre de DevNotes est fermée. Elle cherche dans tous vos espaces à la fois, sans tenir compte des filtres du canevas : quand on rappelle un snippet, on ne se souvient pas toujours où on l’a rangé.',
    ),
    ol(
      ['Tapez quelques lettres du titre, du code ou d’un tag.'],
      ['Choisissez avec les flèches.'],
      [
        keys('Ctrl C'),
        ' copie la note et referme la palette : vous retrouvez l’application d’où vous veniez, prêt à coller.',
      ],
    ),
    p(
      'Un snippet à champs, comme ',
      field('host'),
      ', demande d’abord ses valeurs : copier une commande à trous telle quelle donnerait une commande inutilisable. ',
      keys('Entrée'),
      ' ou un clic ouvrent la note au lieu de la copier, ce qui fait aussi de la palette un « trouver cette note ».',
    ),
    shortcuts(
      'Dans la palette',
      ['↑ ↓', 'Parcourir les résultats (Tab aussi)'],
      ['Entrée', 'Ouvrir la note'],
      ['Ctrl C', 'Copier la note et fermer'],
      ['Échap', 'Fermer la palette'],
    ),
    tip(
      'Rien ne correspond ? La dernière ligne propose ',
      ui('+ Créer une note « … »'),
      ' avec ce que vous avez tapé.',
    ),
    h2('capturer', 'Capturer le presse-papiers'),
    p(
      keys('Ctrl Alt V'),
      ' fait l’inverse de la palette : ce que vous venez de copier, où que ce soit, devient une note. ',
      keys('Ctrl Alt N'),
      ' ouvre une note vide. Les deux marchent fenêtre fermée.',
    ),
    h2('barre-systeme', 'Depuis la barre système'),
    p(
      'L’icône de DevNotes dans la barre système propose ',
      ui('Collage rapide'),
      ', ',
      ui('Nouvelle note'),
      ' et ',
      ui('Coller le presse-papier'),
      '. Par défaut, fermer la fenêtre la range dans la barre système, où DevNotes reste à l’écoute de ses raccourcis (',
      ui('Fermer dans la barre système'),
      ', dans les préférences) ; ',
      ui('Démarrer avec le système'),
      ' le lance à l’ouverture de session.',
    ),
    h2('globaux', 'Les raccourcis globaux'),
    table(
      'Actifs partout, même fenêtre fermée',
      ['Par défaut', 'Action'],
      [keys('Ctrl Alt P'), 'Ouvrir la palette de collage rapide'],
      [keys('Ctrl Alt V'), 'Capturer le presse-papier dans une note'],
      [keys('Ctrl Alt N'), 'Créer une note'],
    ),
    p(
      'Ces trois-là agissent dans toutes vos applications : ils demandent au moins un modificateur, sans quoi la touche serait avalée partout. Le premier arrivé est servi : si une autre application tient déjà une combinaison, DevNotes le signale au démarrage, et vous en choisissez une autre.',
    ),
    h2('zones', 'Changer de zone'),
    shortcuts(
      'Dans les trois zones',
      ['Ctrl 1', 'Aller aux notes'],
      ['Ctrl 2', 'Aller aux outils'],
      ['Ctrl 3', 'Aller au client HTTP'],
      ['Ctrl Shift T', 'Chercher un outil'],
    ),
    h2('regler', 'Régler ses raccourcis'),
    p(
      'Dans ',
      ui('Fichier → Préférences… → Raccourcis'),
      ', cliquez dans le champ d’une action, puis composez la combinaison. DevNotes prévient si elle est déjà prise par une autre action. ',
      ui('Rétablir'),
      ' revient à celle d’origine.',
    ),
    ul(
      [b('Globaux : '), 'au moins un modificateur (Ctrl, Alt, Maj ou Super).'],
      [b('Canevas : '), 'une lettre nue suffit, puisqu’ils n’agissent jamais pendant une saisie.'],
      [
        b('Fixes : '),
        'les flèches, Échap, Tab et les clics avec Ctrl ou Maj ne se déplacent pas. Ce sont la navigation, la sortie et la sélection.',
      ],
    ),
    p(
      'La liste complète des raccourcis, selon l’endroit où vous êtes, est dans ',
      ui('À propos → Raccourcis clavier'),
      '. Ceux du canevas sont détaillés dans ',
      link('Ranger et retrouver', '/docs/ranger'),
      '.',
    ),
  ],
};
