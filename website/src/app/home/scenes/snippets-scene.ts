import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { field, part, typed } from '../../core/browser';
import { Lang } from '../../core/lang';
import { Language } from '../../core/language';
import { CodeView, Line } from '../code-view';
import { ScenePlayer } from '../scene-player';

const HOST = 'db-1.staging.internal';
const USER = 'deploy';
const QUERY = 'tunnel';

const FR = {
  title: 'Tunnel vers la base de staging',
  copyFilled: 'Copier rempli',
  fields: 'Champs',
  goes: 'Ce qui part dans le presse-papiers',
  search: 'Rechercher un snippet…',
  esc: 'Échap',
  enter: 'Entrée',
  move: 'choisir',
  open: 'ouvrir',
  copy: 'copier',
  close: 'fermer',
  copied: 'Copié dans le presse-papiers',
  space: 'Plateforme',
  /** What the palette finds: every space at once, the canvas filters ignored. */
  snippets: [
    'Tunnel vers la base de staging',
    'Tunnel SSH vers Redis',
    'Postgres en local',
    'Quelles requêtes sont lentes',
    'Appeler une API avec un jeton',
  ],
};

const EN: typeof FR = {
  title: 'Tunnel to the staging database',
  copyFilled: 'Copy filled',
  fields: 'Fields',
  goes: 'What goes to the clipboard',
  search: 'Search a snippet…',
  esc: 'Esc',
  enter: 'Enter',
  move: 'move',
  open: 'open',
  copy: 'copy',
  close: 'close',
  copied: 'Copied to the clipboard',
  space: 'Platform',
  snippets: [
    'Tunnel to the staging database',
    'SSH tunnel to Redis',
    'Postgres on localhost',
    'Which queries are slow',
    'Call an API with a token',
  ],
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

const LANGUAGES = ['sh', 'sh', 'sh', 'sql', 'sh'];

const SNIPPET: readonly Line[] = [
  [['k', 'ssh'], ' -N -L ', ['n', '5433'], ':', ['fld', field('db_host')], ':', ['n', '5432'], ' \\'],
  ['    ', ['fld', field('user')], '@bastion.internal'],
];

/** Two fields filled in, then the same snippet recalled over a terminal with Ctrl Alt P. */
@Component({
  selector: 'app-snippets-scene',
  imports: [CodeView],
  templateUrl: './snippets-scene.html',
  styleUrl: './snippets-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[attr.data-step]': 'step()',
    '[class.pal-open]': 'paletteOpen()',
    '[class.copied]': 'copied()',
  },
})
export class SnippetsScene {
  private readonly player = inject(ScenePlayer);
  protected readonly step = this.player.step;
  private readonly t = this.player.t;
  protected readonly field = field;
  protected readonly keys = ['Ctrl', 'Alt', 'P'];
  protected readonly snippet = SNIPPET;
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);

  protected readonly host = computed(() => this.typedIn(HOST, 0.05, 0.5));
  protected readonly user = computed(() => this.typedIn(USER, 0.55, 0.9));
  protected readonly hostTyping = computed(() => this.step() === 1 && this.t() < 0.52);
  protected readonly userTyping = computed(() => this.step() === 1 && this.t() >= 0.52 && this.t() < 0.95);
  protected readonly filled = computed(() => `${(this.host() ? 1 : 0) + (this.user() ? 1 : 0)}/2`);
  protected readonly command = computed(
    () =>
      `ssh -N -L 5433:${this.host() || field('db_host')}:5432 ${this.user() || field('user')}@bastion.internal`,
  );

  protected readonly paletteOpen = computed(() => this.step() === 2 && this.t() >= 0.2);
  protected readonly query = computed(() => {
    if (this.step() === 2) return typed(QUERY, part(this.t(), 0.3, 0.7));
    return this.step() === 3 ? QUERY : '';
  });
  protected readonly results = computed(() => {
    const query = this.query().toLowerCase();
    return this.text().snippets.flatMap((title, i) => {
      const lang = LANGUAGES[i] ?? 'sh';
      const at = title.toLowerCase().indexOf(query);
      if (query && at < 0) return [];
      const end = at + query.length;
      return [
        {
          title,
          lang,
          before: query ? title.slice(0, at) : title,
          match: query ? title.slice(at, end) : '',
          after: query ? title.slice(end) : '',
        },
      ];
    });
  });
  /** The copy is `Ctrl C` in the palette: `Enter` opens the note instead. */
  protected readonly copying = computed(() => this.step() === 2 && this.t() > 0.88);
  protected readonly copied = computed(() => this.step() === 3 && this.t() > 0.08);
  protected readonly pasted = computed(() => (this.copied() ? this.command() : ''));

  protected pressed(index: number): boolean {
    const t = this.t();
    return this.step() === 2 && t > 0.02 + index * 0.05 && t < 0.24;
  }

  private typedIn(value: string, from: number, to: number): string {
    const step = this.step();
    if (step === 0) return '';
    return step === 1 ? typed(value, part(this.t(), from, to)) : value;
  }
}
