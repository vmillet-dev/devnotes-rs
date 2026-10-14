import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  booleanAttribute,
  computed,
  inject,
  input,
  viewChild,
} from '@angular/core';

import { canObserve, clamp, prefersReducedMotion } from '../core/browser';
import { Tone } from '../core/page-progress';
import { ScrollFrame } from '../core/scroll-frame';
import { ScenePlayer } from './scene-player';

/** How much of the scene must be in view before it plays. */
const IN_VIEW = 0.55;

/**
 * One feature: a plain headline, a paragraph, its steps — always visible, each one a
 * button — and the scene beside them. The scene plays by itself once well in view, and
 * again each time it comes back after leaving the screen. The page is never held.
 */
@Component({
  selector: 'app-feature',
  providers: [ScenePlayer],
  templateUrl: './feature.html',
  styleUrl: './feature.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Feature {
  readonly anchor = input.required<string>();
  readonly tone = input.required<Tone>();
  readonly label = input.required<string>();
  readonly heading = input.required<string>();
  readonly steps = input.required<readonly string[]>();
  /** How long each step plays, in milliseconds: one per step. */
  readonly durations = input.required<readonly number[]>();
  /** The scene on the left, the words on the right. */
  readonly flip = input(false, { transform: booleanAttribute });
  /** The words centred above, the steps in a row, the scene across the page: a change of pace. */
  readonly wide = input(false, { transform: booleanAttribute });

  protected readonly player = inject(ScenePlayer);
  protected readonly rgb = computed(() => `var(--${this.tone()}-rgb)`);
  private readonly stage = viewChild.required<ElementRef<HTMLElement>>('stage');

  constructor() {
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      const stage = this.stage().nativeElement;
      const reduce = prefersReducedMotion();
      if (reduce) {
        this.player.show(this.steps().length - 1);
        return;
      }

      // The window settles as it rises up the screen; the scroll is never held for it.
      const stop = frame.watch(() => {
        const rise = clamp((innerHeight - stage.getBoundingClientRect().top) / (innerHeight * IN_VIEW));
        stage.style.setProperty('--q', rise.toFixed(3));
      });
      destroyRef.onDestroy(stop);

      if (!canObserve()) return;
      let armed = true;
      const watch = new IntersectionObserver(
        ([entry]) => {
          if (!entry) return;
          if (!entry.isIntersecting) {
            this.player.rest();
            armed = true;
          } else if (entry.intersectionRatio >= IN_VIEW && armed) {
            armed = false;
            void this.player.play(this.durations());
          }
        },
        { threshold: [0, IN_VIEW] },
      );
      watch.observe(stage);
      destroyRef.onDestroy(() => watch.disconnect());
    });
  }

  protected state(index: number): 'on' | 'done' | 'next' {
    const step = this.player.step();
    if (index < step) return 'done';
    return index === step ? 'on' : 'next';
  }

  /** How much of a step's line is drawn under it: the one playing grows, the past ones are full. */
  protected fill(index: number): number {
    const step = this.player.step();
    if (index < step) return 1;
    return index === step ? Number(this.player.t().toFixed(3)) : 0;
  }
}
