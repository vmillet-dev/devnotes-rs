import { LATEST_RELEASE, REPOSITORY } from '../../core/site';
import { DocPage, b, code, field, h2, h3, keys, link, p, table, tip, ui, ul } from '../doc.model';

export const TRANSFER: DocPage = {
  slug: 'import-export',
  title: 'Importer et exporter',
  summary:
    'Exporter tout, un espace ou une sélection dans une archive .devnotes, protégée ou non, la réimporter sans doublon, et copier des notes en Markdown.',
  body: [
    h2('exporter', 'Exporter'),
    p('Le menu ', ui('Fichier'), ' exporte au choix :'),
    ul(
      [ui('Exporter tout…'), ' : toute la bibliothèque ;'],
      [ui('Exporter l’espace actif…'), ' : l’espace ouvert ;'],
      [ui('Exporter la sélection…'), ' : les notes cochées.'],
    ),
    p(
      'L’export est une archive ',
      code('.devnotes'),
      ' : les notes, leurs espaces et leurs pièces jointes. Les versions des snippets n’en font pas partie.',
    ),
    h3('proteger', 'Protéger un export'),
    p(
      'Avant d’écrire le fichier, DevNotes demande ',
      ui('Protéger cet export ?'),
      ' Une phrase de passe le scelle avec une clé qui lui est propre et qui voyage avec le fichier, pas avec cette machine : celui à qui vous donnez la phrase pourra l’ouvrir, personne d’autre. ',
      ui('Exporter en clair'),
      ' écrit toutes les notes et toutes les pièces jointes lisibles par quiconque détient le fichier.',
    ),
    tip(
      'Un export en clair, rangé en lieu sûr, est la seule copie qui ne dépend pas de votre phrase de passe. Voir ',
      link('Sécurité et sauvegardes', '/docs/securite'),
      '.',
    ),
    h2('importer', 'Importer'),
    p(
      ui('Fichier → Importer…'),
      ' lit une archive ',
      code('.devnotes'),
      ' (ou un ancien export ',
      code('.json'),
      '). Une archive protégée demande sa phrase de passe d’abord.',
    ),
    ul(
      ['Un import fusionne, il ne remplace jamais : les espaces sont rapprochés par leur nom.'],
      [
        'Une note déjà présente est comptée comme ignorée, pas écrasée : réimporter le même fichier ne crée aucun doublon.',
      ],
      ['DevNotes dit toujours ce qu’il a importé, même quand c’est zéro note.'],
    ),
    h2('markdown', 'Copier en Markdown'),
    p(
      ui('Fichier → Copier la sélection en Markdown'),
      ' met les notes cochées dans le presse-papiers : pour chacune, son titre, son espace, son contexte, ses tags, puis son contenu dans un bloc de code. De quoi coller une procédure dans un ticket ou un wiki.',
    ),
  ],
};

export const SETTINGS: DocPage = {
  slug: 'preferences',
  title: 'Préférences',
  summary:
    'Le thème, la langue et la densité, l’éditeur, le comportement de la fenêtre, les variables globales, les raccourcis et la sécurité.',
  body: [
    p(
      ui('Fichier → Préférences…'),
      ' ouvre quatre pages. Le thème et la densité se montrent pendant que vous choisissez ; rien n’est enregistré avant ',
      ui('Appliquer'),
      ' ou ',
      ui('OK'),
      '.',
    ),
    h2('general', 'Général'),
    table(
      'Les réglages généraux',
      ['Groupe', 'Réglages'],
      [
        b('Apparence'),
        'la langue (français ou anglais), le thème (Système, Sombre, Clair), la densité (Confortable, Compacte)',
      ],
      [b('Éditeur'), 'le type de note par défaut, l’indentation avec Tab, le retour à la ligne dans le code'],
      [b('Comportement'), 'démarrer avec le système, réduire et fermer dans la barre système'],
      [b('Collage rapide'), 'les notes épinglées en premier dans la palette'],
      [b('Notifications'), 'accuser chaque copie, signaler les mises à jour'],
    ),
    p(
      'La densité ',
      ui('Compacte'),
      ' resserre les espacements, jamais le texte. Sans barre système, la fenêtre se réduit et se ferme normalement : DevNotes refuse de se cacher là où plus rien ne pourrait la rappeler.',
    ),
    h2('variables', 'Variables'),
    p(
      'Les valeurs par défaut des champs de vos snippets, valables pour toutes les notes : une variable ',
      code('host'),
      ' est proposée à chaque ',
      field('host'),
      '. Ce qui est saisi sur une note passe devant. Un nom se limite aux lettres, chiffres, tirets et soulignés.',
    ),
    h2('raccourcis', 'Raccourcis'),
    p(
      'Les trois raccourcis globaux, ceux du canevas et celui des zones. Voir ',
      link('Collage rapide et raccourcis', '/docs/collage-rapide'),
      '.',
    ),
    h2('securite', 'Sécurité'),
    ul(
      [b('Phrase de passe'), ' : la changer, pour le prochain lancement.'],
      [b('Copies de sauvegarde'), ' : copier la bibliothèque au lancement, et restaurer une copie.'],
    ),
    p('Les deux sont détaillés dans ', link('Sécurité et sauvegardes', '/docs/securite'), '.'),
    h2('preferences-par-bibliotheque', 'Ce que chaque bibliothèque retient'),
    p(
      'Les préférences ci-dessus valent pour l’application. Chaque bibliothèque retient en plus ses propres choix d’affichage, comme la vue ',
      ui('Liste'),
      ' ou ',
      ui('Tableau'),
      ', le tri, ou l’environnement actif du client API.',
    ),
  ],
};

export const FAQ: DocPage = {
  slug: 'faq',
  title: 'Questions fréquentes',
  summary:
    'Le prix, macOS, la synchronisation, une phrase de passe oubliée, l’avertissement de Windows, et où vont vos données.',
  body: [
    h2('gratuit', 'DevNotes est-il gratuit ?'),
    p(
      'Oui, et libre : il est publié sous licence GPL-3.0, son code est sur ',
      link('GitHub', REPOSITORY),
      '. Pas de compte, pas d’abonnement, pas de publicité.',
    ),
    h2('macos', 'Y a-t-il une version macOS ?'),
    p(
      'Pas pour l’instant. Elle ne peut pas être testée ici, et Gatekeeper réclame un compte Apple Developer payant, sans contournement gratuit sur les versions récentes.',
    ),
    h2('synchroniser', 'Puis-je synchroniser deux ordinateurs ?'),
    p(
      'DevNotes ne synchronise rien : c’est ce qui garantit que vos notes ne quittent pas votre machine. Pour passer des notes d’un poste à l’autre, exportez-les dans une archive protégée, puis importez-la de l’autre côté. Un import ne crée jamais de doublon.',
    ),
    h2('oubli', 'J’ai oublié ma phrase de passe.'),
    p(
      'Elle ne se récupère pas : elle enveloppe la clé qui scelle vos notes, et rien d’autre ne l’ouvre. ',
      ui('Phrase de passe oubliée ?'),
      ' archive la bibliothèque, toujours scellée, et en commence une nouvelle. Un export en clair gardé à part est la seule copie qui n’en dépend pas.',
    ),
    h2('smartscreen', 'Windows affiche un avertissement au premier lancement.'),
    p(
      'Choisissez ',
      ui('Informations complémentaires'),
      ', puis ',
      ui('Exécuter quand même'),
      '. L’avertissement porte sur l’identité de l’éditeur : DevNotes n’a pas de certificat de signature de code. Chaque version publie ',
      code('SHA256SUMS.txt'),
      ' pour vérifier les fichiers, et les mises à jour sont signées et vérifiées avant d’être installées.',
    ),
    h2('hors-ligne', 'DevNotes marche-t-il hors ligne ?'),
    p(
      'Entièrement. Seuls le client API, qui contacte les serveurs que vous lui donnez, et la recherche de mises à jour ont besoin du réseau.',
    ),
    h2('mises-a-jour', 'Comment mettre à jour ?'),
    p(
      'DevNotes propose lui-même les nouvelles versions, avec leurs notes. Vous pouvez aussi télécharger ',
      link('la dernière version', LATEST_RELEASE),
      ' et l’installer par-dessus : vos bibliothèques ne sont pas touchées.',
    ),
    h2('donnees', 'Où sont mes notes ?'),
    p(
      'Dans le dossier de votre profil : ',
      code('%APPDATA%\\com.devnotes.app\\'),
      ' sous Windows, ',
      code('~/.local/share/com.devnotes.app/'),
      ' sous Linux. Le détail est dans ',
      link('Sécurité et sauvegardes', '/docs/securite'),
      '.',
    ),
    h2('signaler', 'J’ai trouvé un bug, ou j’ai une idée.'),
    p(
      'Ouvrez un ticket sur ',
      link('le dépôt GitHub', `${REPOSITORY}/issues`),
      ', en disant ce que vous faisiez, ce que vous attendiez et ce qui s’est passé. La version est dans ',
      ui('À propos'),
      '.',
    ),
    tip(
      'Rien de ce que vous tapez dans DevNotes n’est envoyé avec un ticket : copiez seulement ce qui aide à comprendre.',
    ),
    h2('raccourci-pris', 'Ctrl Alt P ne fait rien.'),
    p(
      'Une autre application tient sans doute déjà cette combinaison : DevNotes le signale au démarrage. Choisissez-en une autre dans ',
      ui('Préférences… → Raccourcis'),
      ', ou libérez-la dans l’autre application. Les raccourcis globaux marchent fenêtre fermée tant que DevNotes tourne dans la barre système ; une fois ',
      ui('Quitter DevNotes'),
      ' choisi, ',
      keys('Ctrl Alt P'),
      ' n’a plus d’application à réveiller.',
    ),
  ],
};
