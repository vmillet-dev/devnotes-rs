import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { part, typed } from '../../core/browser';
import { Lang } from '../../core/lang';
import { Language } from '../../core/language';
import { CodeView, Line } from '../code-view';
import { ScenePlayer } from '../scene-player';

const FR = {
  title: 'Petit utilitaire',
  plain: 'Texte brut',
  filter: 'Filtrer les langages',
  format: 'Formater',
  formatted: 'Formaté avec Prettier · 4 lignes modifiées',
  undo: 'Annuler',
  root: 'racine',
  keys4: '4 clés',
  key1: '1 clé',
  item1: '1 élément',
};

const EN: typeof FR = {
  title: 'Small helper',
  plain: 'Plain text',
  filter: 'Filter languages',
  format: 'Format',
  formatted: 'Formatted with Prettier · 4 lines changed',
  undo: 'Undo',
  root: 'root',
  keys4: '4 keys',
  key1: '1 key',
  item1: '1 item',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

/** The language menu's names after the first, the same in both languages. */
const LANGUAGES = [
  'Bash',
  'C#',
  'CSS',
  'Go',
  'GraphQL',
  'JSON',
  'Python',
  'Rust',
  'SQL',
  'TypeScript',
  'YAML',
];

const PLAIN: readonly Line[] = [['const a={b:1,c:[1,2,3]};function f(x){return x*2}']];

const MESSY: readonly Line[] = [
  [
    ['k', 'const'],
    ' a={b:',
    ['n', '1'],
    ',c:[',
    ['n', '1'],
    ',',
    ['n', '2'],
    ',',
    ['n', '3'],
    ']};',
    ['k', 'function'],
    ' ',
    ['f', 'f'],
    '(x){',
    ['k', 'return'],
    ' x*',
    ['n', '2'],
    '}',
  ],
];

const PRETTY: readonly Line[] = [
  [
    ['k', 'const'],
    ' a = { b: ',
    ['n', '1'],
    ', c: [',
    ['n', '1'],
    ', ',
    ['n', '2'],
    ', ',
    ['n', '3'],
    '] };',
  ],
  [['k', 'function'], ' ', ['f', 'f'], '(x) {'],
  ['  ', ['k', 'return'], ' x * ', ['n', '2'], ';'],
  ['}'],
];

const JSON_LINES: readonly Line[] = [
  ['{'],
  ['  ', ['f', '"id"'], ': ', ['s', '"evt_1PqX4c"'], ','],
  ['  ', ['f', '"type"'], ': ', ['s', '"invoice.paid"'], ','],
  ['  ', ['f', '"data"'], ': {'],
  ['    ', ['f', '"object"'], ': {'],
  ['      ', ['f', '"amount_paid"'], ': ', ['n', '4900'], ','],
  ['      ', ['f', '"currency"'], ': ', ['s', '"eur"'], ','],
  ['      ', ['f', '"lines"'], ': [ … ]'],
  ['    }'],
  ['  },'],
  ['  ', ['f', '"livemode"'], ': ', ['k', 'false']],
  ['}'],
];

/** A node of the JSON graph. */
type Selection = 'none' | 'type' | 'data' | 'object' | 'lines';

/** The lines of the code each node stands for. */
const SPANS: Record<Selection, readonly number[]> = {
  none: [],
  type: [2],
  data: [3, 4, 5, 6, 7, 8, 9],
  object: [4, 5, 6, 7, 8],
  lines: [7],
};

/** The language picked from a filtered menu, Prettier, then a JSON read as a graph beside its code. */
@Component({
  selector: 'app-code-scene',
  imports: [CodeView],
  templateUrl: './code-scene.html',
  styleUrl: './code-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[attr.data-step]': 'step()',
    '[class.menu]': 'menuOpen()',
    '[class.chosen]': 'chosen()',
    '[class.formatted]': 'formatted()',
  },
})
export class CodeScene {
  private readonly player = inject(ScenePlayer);
  protected readonly step = this.player.step;
  private readonly t = this.player.t;
  protected readonly plain = PLAIN;
  protected readonly messy = MESSY;
  protected readonly pretty = PRETTY;
  protected readonly json = JSON_LINES;
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);

  protected readonly menuOpen = computed(() => this.step() === 0 && this.t() > 0.12 && this.t() < 0.8);
  protected readonly filter = computed(() =>
    this.step() === 0 ? typed('ty', part(this.t(), 0.3, 0.55)) : '',
  );
  protected readonly languages = computed(() => {
    const filter = this.filter().toLowerCase();
    return [this.text().plain, ...LANGUAGES].filter((name) => name.toLowerCase().includes(filter));
  });
  protected readonly chosen = computed(() => this.step() > 0 || this.t() >= 0.8);
  protected readonly formatted = computed(() => this.step() > 1 || (this.step() === 1 && this.t() > 0.2));
  protected readonly selected = computed<Selection>(() => {
    if (this.step() < 2) return 'none';
    const t = this.t();
    if (t < 0.25) return 'type';
    if (t < 0.5) return 'data';
    return t < 0.75 ? 'object' : 'lines';
  });
  protected readonly litLines = computed(() => SPANS[this.selected()]);
}
