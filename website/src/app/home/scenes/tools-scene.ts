import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { part, typed } from '../../core/browser';
import { Lang } from '../../core/lang';
import { Language } from '../../core/language';
import { ScenePlayer } from '../scene-player';

type Category = 'text' | 'crypto' | 'encode' | 'generate' | 'time' | 'calc' | 'reference';

/** Each family's hue, as the application's tools home draws it. */
const HUES: Record<Category, string> = {
  text: 'var(--blue-rgb)',
  crypto: 'var(--purple-rgb)',
  encode: 'var(--green-rgb)',
  generate: 'var(--red-rgb)',
  time: 'var(--teal-rgb)',
  calc: 'var(--pink-rgb)',
  reference: 'var(--olive-rgb)',
};

type Catalogue = readonly { id: Category; name: string; tools: readonly (readonly [string, string])[] }[];

/** The catalogue as the application lists it, a name and one line each. */
const FR_CATALOGUE: Catalogue = [
  {
    id: 'text',
    name: 'Texte',
    tools: [
      ['Convertisseur de casse', 'camelCase, snake_case, dot.case…'],
      ['Sauts de ligne', 'CRLF ↔ LF, espaces en fin de ligne'],
      ['Statistiques du texte', 'Caractères, mots, lignes, octets'],
      ['Diff', 'Deux textes, ou deux documents JSON, YAML, TOML, XML'],
    ],
  },
  {
    id: 'crypto',
    name: 'Crypto',
    tools: [
      ['Hash / HMAC', 'MD5, SHA-1, SHA-2, SHA-3'],
      ['Mots de passe', 'Longueur, jeux de caractères, entropie'],
      ['Identifiants', 'UUID, ULID, NanoID, CUID2…'],
      ['JWT', 'Décodé, ses dates lues, sa signature vérifiée'],
    ],
  },
  {
    id: 'encode',
    name: 'Encodeurs',
    tools: [
      ['URL', 'Encoder, décoder, analyser'],
      ['Base64 et autres bases', 'Base64, Base32, hexadécimal, binaire'],
      ['JSON ↔ YAML ↔ TOML ↔ XML', 'D’un format à l’autre'],
      ['Couleurs', 'HEX, RGB, HSL, OKLCH, contraste'],
    ],
  },
  {
    id: 'generate',
    name: 'Générateurs',
    tools: [
      ['JSON', 'À partir d’un schéma ou d’un modèle'],
      ['Identité fictive', 'Une personne de test cohérente'],
      ['Lorem ipsum', 'Mots, phrases, paragraphes'],
      ['QR code', 'Texte, URL, Wi-Fi, contact'],
    ],
  },
  {
    id: 'time',
    name: 'Temps',
    tools: [
      ['Dates et durées', 'Unix ↔ ISO 8601, écarts et durées'],
      ['Fuseaux horaires', 'Une heure, dans chaque fuseau'],
      ['Cron', 'Lu à voix haute, ses prochaines exécutions'],
    ],
  },
  {
    id: 'calc',
    name: 'Calcul',
    tools: [
      ['Tailles et transfert', 'Ko, Mo ↔ Kio, Mio, et le temps d’un transfert'],
      ['Pourcentages', 'Part, évolution, hausse et baisse'],
      ['Permissions Unix', 'chmod : 755 ↔ rwxr-xr-x'],
      ['Luhn et IBAN', 'Valide ou non, et la bonne clé'],
      ['Sous-réseaux CIDR', 'Masques, hôtes, découpage'],
    ],
  },
  {
    id: 'reference',
    name: 'Références',
    tools: [
      ['Codes de statut HTTP', '404, 429, 503'],
      ['En-têtes HTTP', 'Content-Type, CORS, CSP'],
      ['Types MIME', 'Le type d’une extension'],
      ['Ports courants', '22, 443, 5432, 6379'],
      ['Signaux Unix', 'SIGTERM, SIGKILL, SIGHUP'],
      ['Table ASCII', 'Les 128 caractères'],
      ['Aide-mémoire regex', 'JavaScript et PCRE'],
      ['Aide-mémoire Markdown', 'CommonMark et GitHub'],
      ['Aide-mémoire Mustache', 'Variables, sections, partiels'],
    ],
  },
];

const EN_CATALOGUE: Catalogue = [
  {
    id: 'text',
    name: 'Text',
    tools: [
      ['Case converter', 'camelCase, snake_case, dot.case…'],
      ['Line breaks', 'CRLF ↔ LF, trailing spaces'],
      ['Text statistics', 'Characters, words, lines, bytes'],
      ['Diff', 'Two texts, or two JSON, YAML, TOML, XML documents'],
    ],
  },
  {
    id: 'crypto',
    name: 'Crypto',
    tools: [
      ['Hash / HMAC', 'MD5, SHA-1, SHA-2, SHA-3'],
      ['Passwords', 'Length, character sets, entropy'],
      ['Identifiers', 'UUID, ULID, NanoID, CUID2…'],
      ['JWT', 'Decoded, its dates read, its signature verified'],
    ],
  },
  {
    id: 'encode',
    name: 'Encoders',
    tools: [
      ['URL', 'Encode, decode, take apart'],
      ['Base64 and other bases', 'Base64, Base32, hexadecimal, binary'],
      ['JSON ↔ YAML ↔ TOML ↔ XML', 'From one format to another'],
      ['Colours', 'HEX, RGB, HSL, OKLCH, contrast'],
    ],
  },
  {
    id: 'generate',
    name: 'Generators',
    tools: [
      ['JSON', 'From a schema or a template document'],
      ['Fictitious identity', 'A consistent test person'],
      ['Lorem ipsum', 'Words, sentences, paragraphs'],
      ['QR code', 'Text, URL, Wi-Fi, contact'],
    ],
  },
  {
    id: 'time',
    name: 'Time',
    tools: [
      ['Dates and durations', 'Unix ↔ ISO 8601, gaps and durations'],
      ['Time zones', 'One time, in every zone chosen'],
      ['Cron', 'Read aloud, its next runs'],
    ],
  },
  {
    id: 'calc',
    name: 'Calculate',
    tools: [
      ['Sizes and transfer', 'kB, MB ↔ KiB, MiB, and how long a transfer takes'],
      ['Percentages', 'Share, change, rise and fall'],
      ['Unix permissions', 'chmod: 755 ↔ rwxr-xr-x'],
      ['Luhn and IBAN', 'Valid or not, and the right check digits'],
      ['CIDR subnets', 'Masks, hosts, splitting'],
    ],
  },
  {
    id: 'reference',
    name: 'References',
    tools: [
      ['HTTP status codes', '404, 429, 503'],
      ['HTTP headers', 'Content-Type, CORS, CSP'],
      ['MIME types', 'The type of an extension'],
      ['Common ports', '22, 443, 5432, 6379'],
      ['Unix signals', 'SIGTERM, SIGKILL, SIGHUP'],
      ['ASCII table', 'The 128 characters'],
      ['Regex cheat sheet', 'JavaScript and PCRE'],
      ['Markdown cheat sheet', 'CommonMark and GitHub'],
      ['Mustache cheat sheet', 'Variables, sections, partials'],
    ],
  },
];

const FR = {
  catalogue: FR_CATALOGUE,
  tools: 'Outils',
  count: '33 utilitaires, tous hors ligne',
  search: 'Rechercher un outil…',
  searchKeys: 'Ctrl Maj T',
  favourites: 'Favoris',
  all: 'Tous les outils',
  recent: 'Récents',
  recentTool: 'Base64 et autres bases',
  example: 'Exemple',
  clear: 'Vider',
  save: 'Enregistrer comme note',
  expression: 'Expression',
  readAloud: 'Lue à voix haute',
  said: 'Toutes les 15 minutes, de 9 h à 18 h 45, du lundi au vendredi',
  nextRuns: 'Prochaines exécutions',
  runs: [
    'lun. 19 oct. 09:00',
    'lun. 19 oct. 09:15',
    'lun. 19 oct. 09:30',
    'lun. 19 oct. 09:45',
    'lun. 19 oct. 10:00',
    'lun. 19 oct. 10:15',
  ],
  token: 'Jeton',
  issued: 'Émis',
  issuedAt: '1 décembre 2025, 00:00 UTC',
  expires: 'Expire',
  expired: '1 janvier 2026, expiré',
  signature: 'Signature HS256',
  verified: 'vérifiée avec le secret',
  saveTitle: 'Enregistrer comme note',
  what: "Un snippet JSON, avec ce que l'outil a calculé, jamais ce qui y a été tapé.",
  noteTitle: 'Titre',
  space: 'Espace',
  folder: 'Dossier',
  noFolder: 'Aucun dossier',
  tags: 'Tags',
  submit: 'Enregistrer',
  title: 'Charge utile du jeton de test',
  platform: 'Plateforme',
  saved: 'Note « Charge utile du jeton de test » enregistrée dans Plateforme.',
};

const EN: typeof FR = {
  catalogue: EN_CATALOGUE,
  tools: 'Tools',
  count: '33 utilities, all offline',
  search: 'Search for a tool…',
  searchKeys: 'Ctrl Shift T',
  favourites: 'Favourites',
  all: 'All tools',
  recent: 'Recent',
  recentTool: 'Base64 and other bases',
  example: 'Example',
  clear: 'Clear',
  save: 'Save as note',
  expression: 'Expression',
  readAloud: 'Read aloud',
  said: 'Every 15 minutes, from 09:00 to 18:45, Monday to Friday',
  nextRuns: 'Next runs',
  runs: [
    'Mon 19 Oct 09:00',
    'Mon 19 Oct 09:15',
    'Mon 19 Oct 09:30',
    'Mon 19 Oct 09:45',
    'Mon 19 Oct 10:00',
    'Mon 19 Oct 10:15',
  ],
  token: 'Token',
  issued: 'Issued',
  issuedAt: '1 December 2025, 00:00 UTC',
  expires: 'Expires',
  expired: '1 January 2026, expired',
  signature: 'HS256 signature',
  verified: 'verified with the secret',
  saveTitle: 'Save as note',
  what: 'A JSON snippet, holding what the tool computed, never what was typed into it.',
  noteTitle: 'Title',
  space: 'Space',
  folder: 'Folder',
  noFolder: 'No folder',
  tags: 'Tags',
  submit: 'Save',
  title: 'Payload of the test token',
  platform: 'Platform',
  saved: 'Note “Payload of the test token” saved in Platform.',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

const QUERY = 'cron';
const CRON = '*/15 9-18 * * 1-5';
const TOKEN =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI0MiIsIm5hbWUiOiJBZGEiLCJpYXQiOjE3NjQ1NDcyMDAsImV4cCI6MTc2NzIyNTYwMH0.6Zq…';

/** A piece of text with what the search found in it cut out. */
interface Marked {
  readonly before: string;
  readonly match: string;
  readonly after: string;
}

const fold = (text: string) => text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

function mark(text: string, query: string): Marked {
  const at = query ? fold(text).indexOf(fold(query)) : -1;
  if (at < 0) return { before: text, match: '', after: '' };
  return {
    before: text.slice(0, at),
    match: text.slice(at, at + query.length),
    after: text.slice(at + query.length),
  };
}

/** Search the tools, read a cron expression, decode a JWT, keep the result as a note. */
@Component({
  selector: 'app-tools-scene',
  templateUrl: './tools-scene.html',
  styleUrl: './tools-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[class.saving]': 'saving()',
    '[class.pressed]': 'pressed()',
    '[class.saved]': 'saved()',
  },
})
export class ToolsScene {
  private readonly player = inject(ScenePlayer);
  protected readonly step = this.player.step;
  private readonly t = this.player.t;
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  protected readonly hues = HUES;
  protected readonly cron = CRON;
  protected readonly token = TOKEN;

  protected readonly query = computed(() =>
    this.step() === 0 ? typed(QUERY, part(this.t(), 0.15, 0.5)) : '',
  );
  protected readonly searching = computed(() => this.step() === 0 && this.t() < 0.6);
  protected readonly panels = computed(() => {
    const query = fold(this.query());
    let first = true;
    return this.text().catalogue.map((category) => {
      const tools = category.tools.map(([name, line]) => {
        const match = Boolean(query) && fold(`${name} ${line}`).includes(query);
        const opens = match && first;
        if (match) first = false;
        return { name: mark(name, this.query()), line: mark(line, this.query()), match, opens };
      });
      return { ...category, hue: HUES[category.id], tools, found: tools.some((tool) => tool.match) };
    });
  });

  /** The open tool's family in the rail, its neighbours listed under it. */
  protected readonly family = computed(() => {
    const step = this.step();
    if (step === 0) return null;
    const id: Category = step === 1 ? 'time' : 'crypto';
    const category = this.text().catalogue.find((each) => each.id === id);
    return category
      ? {
          name: category.name,
          tools: category.tools.map(([name]) => name),
          open: step === 1 ? 'Cron' : 'JWT',
        }
      : null;
  });

  protected readonly cronTyped = computed(() =>
    this.step() === 1 ? typed(CRON, part(this.t(), 0.02, 0.3)) : CRON,
  );
  protected readonly cronRead = computed(() => this.step() > 1 || this.t() > 0.32);
  protected readonly runsShown = computed(() =>
    this.step() === 1
      ? Math.round(this.text().runs.length * part(this.t(), 0.45, 0.85))
      : this.text().runs.length,
  );

  protected readonly pasted = computed(() => this.step() > 2 || this.t() > 0.1);
  protected readonly decoded = computed(() => this.step() > 2 || this.t() > 0.3);

  protected readonly saving = computed(() => this.step() === 3 && this.t() > 0.05 && this.t() < 0.75);
  protected readonly pressed = computed(() => this.step() === 3 && this.t() > 0.6 && this.t() < 0.7);
  protected readonly saved = computed(() => this.step() === 3 && this.t() >= 0.72);
  protected readonly noteTitle = computed(() =>
    this.step() === 3 ? typed(this.text().title, part(this.t(), 0.1, 0.45)) : '',
  );
}
