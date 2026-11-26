import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
} from '@angular/core';

import { clamp, ease, field } from '../../core/browser';
import { Lang } from '../../core/lang';
import { Scene } from './day-text';
import { IconName, KindIcon } from './kind-icon';
import { SceneLoop } from './scene-loop';
import { SCENE_WORDS } from './scene-words';

/** How long a scene's turn lasts, the pause at its end included, in milliseconds. */
export const LOOPS: Readonly<Record<Scene, number>> = {
  list: 7200,
  snippet: 9000,
  drawing: 8000,
  request: 8600,
  jwt: 8000,
  diff: 7600,
  search: 7600,
  calc: 9000,
  note: 10000,
  lock: 7000,
};

/** The kinds of the notes the search goes through, in their order. */
const ROW_KINDS: readonly IconName[] = ['snip', 'snip', 'list', 'calc', 'note'];

/** Where the drawing's database is dragged to, and when, in the turn. */
const DRAG = { dx: 24, dy: 40, from: 3000, lasts: 1300 };

/** One moment's scene: the application's pieces, moving as the moment says. */
@Component({
  selector: 'app-day-scene',
  imports: [KindIcon],
  templateUrl: './day-scene.html',
  styleUrl: './day-scene.scss',
  hostDirectives: [{ directive: SceneLoop, inputs: ['appSceneLoop: loop'] }],
  host: { 'aria-hidden': 'true' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DayScene {
  readonly scene = input.required<Scene>();
  readonly lang = input.required<Lang>();

  protected readonly words = computed(() => SCENE_WORDS[this.lang()]);
  protected readonly field = field;
  /** A code view keeps its indent: the template's own spaces would be collapsed. */
  protected readonly indent = '    ';
  protected readonly open = 'server {';
  protected readonly close = '}';
  protected readonly header = '{ "alg": "HS256", "typ": "JWT" }';
  protected readonly claims = computed(() => {
    const [sub, scope] = this.words().jwt.claims;
    return `{ ${sub}\n  ${scope} }`;
  });
  protected readonly terminal = computed(() => {
    const { service, namespace } = this.words().snippet;
    return ` kubectl rollout restart \\\n    deployment/${service} -n ${namespace}`;
  });
  /** The notes the search goes through, each with its kind's icon. */
  protected readonly searchRows = computed(() =>
    this.words().search.rows.map((row, i) => ({ ...row, kind: ROW_KINDS[i] ?? 'note' })),
  );

  constructor() {
    const loop = inject(SceneLoop);
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    const destroyRef = inject(DestroyRef);

    // The drawing's arrows hold on to the shape they point at: they are drawn again as it moves.
    afterNextRender(() => {
      if (this.scene() !== 'drawing') return;
      const dragged = host.querySelector('.dragged');
      const held = [host.querySelector('.held-1'), host.querySelector('.held-2')];
      const cursor = host.querySelector<SVGGElement>('.cursor');
      if (!dragged || !held[0] || !held[1] || !cursor) return;

      const put = (t: number) => {
        const dx = DRAG.dx * t;
        const dy = DRAG.dy * t;
        dragged.setAttribute('transform', `translate(${dx} ${dy})`);
        held[0]?.setAttribute('d', `M480 73 L${522 + dx} ${132 + dy}`);
        held[1]?.setAttribute('d', `M480 223 L${522 + dx} ${164 + dy}`);
        cursor.setAttribute('transform', `translate(${586 + dx} ${154 + dy})`);
      };
      let frame = 0;
      let start = 0;
      const step = (now: number) => {
        const elapsed = now - start;
        put(ease(clamp((elapsed - DRAG.from) / DRAG.lasts)));
        cursor.style.opacity =
          elapsed > DRAG.from - 400 && elapsed < DRAG.from + DRAG.lasts + 500 ? '1' : '0';
        frame = elapsed < DRAG.from + DRAG.lasts + 700 ? requestAnimationFrame(step) : 0;
      };
      const replay = loop.replay.subscribe(() => {
        cancelAnimationFrame(frame);
        start = performance.now();
        put(0);
        frame = requestAnimationFrame(step);
      });
      destroyRef.onDestroy(() => {
        replay.unsubscribe();
        cancelAnimationFrame(frame);
      });
    });
  }
}
