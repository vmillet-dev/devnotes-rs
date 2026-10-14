import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { part, typed } from '../../core/browser';
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

/** The catalogue as the application lists it, a name and one line each. */
const CATALOGUE: readonly { id: Category; name: string; tools: readonly (readonly [string, string])[] }[] = [
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

const QUERY = 'cron';
const CRON = '*/15 9-18 * * 1-5';
const RUNS = [
  'lun. 19 oct. 09:00',
  'lun. 19 oct. 09:15',
  'lun. 19 oct. 09:30',
  'lun. 19 oct. 09:45',
  'lun. 19 oct. 10:00',
  'lun. 19 oct. 10:15',
];
const TOKEN =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI0MiIsIm5hbWUiOiJBZGEiLCJpYXQiOjE3NjQ1NDcyMDAsImV4cCI6MTc2NzIyNTYwMH0.6Zq…';
const NOTE_TITLE = 'Charge utile du jeton de test';

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
  protected readonly hues = HUES;
  protected readonly cron = CRON;
  protected readonly runs = RUNS;
  protected readonly token = TOKEN;

  protected readonly query = computed(() =>
    this.step() === 0 ? typed(QUERY, part(this.t(), 0.15, 0.5)) : '',
  );
  protected readonly searching = computed(() => this.step() === 0 && this.t() < 0.6);
  protected readonly panels = computed(() => {
    const query = fold(this.query());
    let first = true;
    return CATALOGUE.map((category) => {
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
    const category = CATALOGUE.find((each) => each.id === id);
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
    this.step() === 1 ? Math.round(RUNS.length * part(this.t(), 0.45, 0.85)) : RUNS.length,
  );

  protected readonly pasted = computed(() => this.step() > 2 || this.t() > 0.1);
  protected readonly decoded = computed(() => this.step() > 2 || this.t() > 0.3);

  protected readonly saving = computed(() => this.step() === 3 && this.t() > 0.05 && this.t() < 0.75);
  protected readonly pressed = computed(() => this.step() === 3 && this.t() > 0.6 && this.t() < 0.7);
  protected readonly saved = computed(() => this.step() === 3 && this.t() >= 0.72);
  protected readonly noteTitle = computed(() =>
    this.step() === 3 ? typed(NOTE_TITLE, part(this.t(), 0.1, 0.45)) : '',
  );
}
