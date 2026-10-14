import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { field, part, typed } from '../../core/browser';
import { CodeView, Line } from '../code-view';
import { ScenePlayer } from '../scene-player';

const HOST = 'db-1.staging.internal';
const USER = 'deploy';
const QUERY = 'tunnel';

/** What the palette finds: every space at once, the canvas filters ignored. */
const SNIPPETS = [
  { title: 'Tunnel vers la base de staging', lang: 'sh' },
  { title: 'Tunnel SSH vers Redis', lang: 'sh' },
  { title: 'Postgres en local', lang: 'sh' },
  { title: 'Quelles requêtes sont lentes', lang: 'sql' },
  { title: 'Appeler une API avec un jeton', lang: 'sh' },
] as const;

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
    return SNIPPETS.flatMap(({ title, lang }) => {
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
