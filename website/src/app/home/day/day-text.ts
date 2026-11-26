import { Lang } from '../../core/lang';
import { Run } from '../runs';

export type Scene =
  'list' | 'snippet' | 'drawing' | 'request' | 'jwt' | 'diff' | 'search' | 'calc' | 'note' | 'lock';

/** The kind's colour in the application, which names the moment. */
export type MomentTone = 'list' | 'snip' | 'draw' | 'http' | 'tool' | 'find' | 'calc' | 'note' | 'lock';

export interface Moment {
  /** Its anchor, its hour without the colon: `m0852`. */
  readonly id: string;
  readonly time: string;
  readonly tone: MomentTone;
  readonly scene: Scene;
  readonly kind: string;
  readonly title: string;
  readonly text: readonly Run[];
  readonly gesture?: readonly Run[];
}

interface DayText {
  readonly section: string;
  readonly span: string;
  readonly bridge: string;
  /** What the hour says before the first moment is reached. */
  readonly before: string;
  readonly moments: readonly Moment[];
}

const FR: DayText = {
  section: 'Une journée',
  span: 'Dix moments, de 08:52 à 18:10',
  bridge:
    "Vous voulez voir comment DevNotes s'utilise, concrètement ? Voici une journée dans la peau d'un développeur.",
  before: 'Une journée de développeur',
  moments: [
    {
      id: 'm0852',
      time: '08:52',
      tone: 'list',
      scene: 'list',
      kind: 'Liste de tâches',
      title: 'La liste du jour',
      text: [
        "La démonstration est à 10 h, et il reste trois choses à faire. Une liste les garde, et sa carte montre où vous en êtes sans l'ouvrir.",
      ],
    },
    {
      id: 'm0940',
      time: '09:40',
      tone: 'snip',
      scene: 'snippet',
      kind: 'Snippet',
      title: 'Relancer le paiement en recette',
      text: [
        'Le ticket demande de relancer le service. La commande est dans un snippet, avec deux champs : ',
        { b: 'service' },
        ' et ',
        { b: 'namespace' },
        '. Vous restez dans le terminal.',
      ],
      gesture: [
        { keys: ['Ctrl', 'Alt', 'P'] },
        ' ouvre la palette, ',
        { keys: ['Entrée'] },
        ' demande les deux valeurs, puis la commande remplie est copiée.',
      ],
    },
    {
      id: 'm1030',
      time: '10:30',
      tone: 'draw',
      scene: 'drawing',
      kind: 'Dessin',
      title: 'Le réseau expliqué au nouvel arrivant',
      text: [
        "Une recrue demande par où passe un paiement. Cinq formes, et des flèches accrochées aux formes, qui les suivent quand on les déplace. Le Dessin reste avec les notes, une Note peut l'afficher, et il part en ",
        { b: 'PNG' },
        ' ou en ',
        { b: 'SVG' },
        ' pour le wiki.',
      ],
    },
    {
      id: 'm1115',
      time: '11:15',
      tone: 'http',
      scene: 'request',
      kind: 'Requête HTTP',
      title: 'La liste des factures revient vide',
      text: [
        'La requête ',
        { b: 'Lister les factures' },
        " part sur l'environnement de recette. Ses variables viennent de la requête, du dossier, de la collection, puis de l'environnement : une variable définie nulle part bloque l'envoi.",
      ],
      gesture: [{ keys: ['Ctrl', 'Entrée'] }, ' envoie la requête.'],
    },
    {
      id: 'm1330',
      time: '13:30',
      tone: 'tool',
      scene: 'jwt',
      kind: 'Outil JWT',
      title: 'Un jeton dans les journaux',
      text: [
        "Un JWT traîne dans un log. L'outil le décode sur votre machine, sans rien envoyer, et lit ses dates : émis quand, valable jusqu'à quand. Avec le secret, il vérifie aussi la signature.",
      ],
      gesture: [{ keys: ['Ctrl', 'Maj', 'T'] }, ' cherche parmi les 32 outils.'],
    },
    {
      id: 'm1420',
      time: '14:20',
      tone: 'tool',
      scene: 'diff',
      kind: 'Outil Diff',
      title: "La recette et la prod n'ont pas la même config",
      text: [
        "La recette répond en HTTP, la production en HTTPS. Les deux fichiers côte à côte : Diff montre les lignes changées et, dans chaque ligne, ce qui change. Chaque côté peut venir d'une note, et l'écart peut en devenir une.",
      ],
    },
    {
      id: 'm1505',
      time: '15:05',
      tone: 'find',
      scene: 'search',
      kind: 'Recherche',
      title: 'La requête SQL de mars',
      text: [
        'Elle est quelque part. Vous tapez « impayee », sans accent : la recherche lit les titres, le code, les listes et les tags, et cite la ligne trouvée.',
      ],
      gesture: [{ keys: ['Ctrl', 'K'] }, ' cherche dans la bibliothèque.'],
    },
    {
      id: 'm1620',
      time: '16:20',
      tone: 'calc',
      scene: 'calc',
      kind: 'Calcul',
      title: 'Le budget du déplacement',
      text: [
        "Le train, l'hôtel, les repas : une ligne chacun, le total en dessous. Un Calcul compte en euros, en heures ou en octets, et refait le compte dès qu'un chiffre change.",
      ],
    },
    {
      id: 'm1730',
      time: '17:30',
      tone: 'note',
      scene: 'note',
      kind: 'Note',
      title: 'La mise en production de demain',
      text: [
        "Les étapes s'écrivent une fois, mises en forme : la liste numérotée, le tableau des durées, la commande à lancer. La Note est enregistrée en ",
        { b: 'Markdown' },
        ", et « Présenter » l'affiche en plein écran pour la réunion de demain.",
      ],
    },
    {
      id: 'm1810',
      time: '18:10',
      tone: 'lock',
      scene: 'lock',
      kind: 'Fermeture',
      title: 'On ferme. Tout est écrit, et chiffré.',
      text: [
        'En quittant, DevNotes écrit ce qui reste et ferme la bibliothèque. Sur le disque, vos titres et vos contenus sont chiffrés ; demain, la première ouverture demandera la phrase de passe et prendra une copie.',
      ],
    },
  ],
};

const EN: DayText = {
  section: 'A day',
  span: 'Ten moments, from 08:52 to 18:10',
  bridge: 'Want to see how DevNotes is used, day to day? Here is a day in the life of a developer.',
  before: "A developer's day",
  moments: [
    {
      id: 'm0852',
      time: '08:52',
      tone: 'list',
      scene: 'list',
      kind: 'To-do list',
      title: "The day's list",
      text: [
        'The demo is at 10, and three things are left to do. A list keeps them, and its card shows where you stand without opening it.',
      ],
    },
    {
      id: 'm0940',
      time: '09:40',
      tone: 'snip',
      scene: 'snippet',
      kind: 'Snippet',
      title: 'Restarting payments on staging',
      text: [
        'The ticket asks to restart the service. The command is in a snippet, with two fields: ',
        { b: 'service' },
        ' and ',
        { b: 'namespace' },
        '. You stay in the terminal.',
      ],
      gesture: [
        { keys: ['Ctrl', 'Alt', 'P'] },
        ' opens the palette, ',
        { keys: ['Enter'] },
        ' asks for both values, then the filled command is copied.',
      ],
    },
    {
      id: 'm1030',
      time: '10:30',
      tone: 'draw',
      scene: 'drawing',
      kind: 'Drawing',
      title: 'The network, explained to a newcomer',
      text: [
        'A new hire asks which way a payment goes. Five shapes, and arrows attached to the shapes, which follow them when they move. The Drawing stays with the notes, a Note can show it, and it leaves as ',
        { b: 'PNG' },
        ' or ',
        { b: 'SVG' },
        ' for the wiki.',
      ],
    },
    {
      id: 'm1115',
      time: '11:15',
      tone: 'http',
      scene: 'request',
      kind: 'HTTP request',
      title: 'The invoice list comes back empty',
      text: [
        'The ',
        { b: 'List invoices' },
        ' request goes to the staging environment. Its variables come from the request, the folder, the collection, then the environment: a variable set nowhere stops the send.',
      ],
      gesture: [{ keys: ['Ctrl', 'Enter'] }, ' sends the request.'],
    },
    {
      id: 'm1330',
      time: '13:30',
      tone: 'tool',
      scene: 'jwt',
      kind: 'JWT tool',
      title: 'A token in the logs',
      text: [
        'A JWT is lying in a log. The tool decodes it on your machine, sending nothing, and reads its dates: issued when, valid until when. Given the secret, it checks the signature too.',
      ],
      gesture: [{ keys: ['Ctrl', 'Shift', 'T'] }, ' searches the 32 tools.'],
    },
    {
      id: 'm1420',
      time: '14:20',
      tone: 'tool',
      scene: 'diff',
      kind: 'Diff tool',
      title: 'Staging and production differ',
      text: [
        'Staging answers over HTTP, production over HTTPS. Both files side by side: Diff shows the lines that changed and, within each line, what changed. Either side can come from a note, and the difference can become one.',
      ],
    },
    {
      id: 'm1505',
      time: '15:05',
      tone: 'find',
      scene: 'search',
      kind: 'Search',
      title: 'The March SQL query',
      text: [
        'It is somewhere. The search reads titles, code, lists and tags, ignoring case and accents, and quotes the line it found.',
      ],
      gesture: [{ keys: ['Ctrl', 'K'] }, ' searches the library.'],
    },
    {
      id: 'm1620',
      time: '16:20',
      tone: 'calc',
      scene: 'calc',
      kind: 'Calculation',
      title: 'The trip budget',
      text: [
        'Train, hotel, meals: one line each, the total underneath. A Calculation counts in euros, hours or bytes, and redoes the sum as soon as a figure changes.',
      ],
    },
    {
      id: 'm1730',
      time: '17:30',
      tone: 'note',
      scene: 'note',
      kind: 'Note',
      title: "Tomorrow's release",
      text: [
        'The steps are written once, formatted: the numbered list, the table of durations, the command to run. The Note is saved as ',
        { b: 'Markdown' },
        ', and “Present” shows it full screen for tomorrow’s meeting.',
      ],
    },
    {
      id: 'm1810',
      time: '18:10',
      tone: 'lock',
      scene: 'lock',
      kind: 'Closing',
      title: 'Closing time. Everything is written, and encrypted.',
      text: [
        'On quitting, DevNotes writes what is left and closes the library. On disk, your titles and contents are encrypted; tomorrow, the first launch will ask for the passphrase and take a copy.',
      ],
    },
  ],
};

export const DAY_TEXT: Readonly<Record<Lang, DayText>> = { fr: FR, en: EN };
