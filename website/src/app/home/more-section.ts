import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  inject,
  signal,
  viewChild,
} from '@angular/core';

import { canObserve, prefersReducedMotion, sleep } from '../core/browser';

const CHECKLIST = ['Geler la branche', 'Lancer la CI', 'Publier la release', "Prévenir l'équipe"];

/** Everything around the notes, a tile each. Two of them play once in view: a list ticks itself, a deletion counts down. */
@Component({
  selector: 'app-more-section',
  templateUrl: './more-section.html',
  styleUrl: './more-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MoreSection {
  protected readonly checklist = CHECKLIST;
  /** How many items of the list are ticked. */
  protected readonly ticked = signal(1);
  protected readonly countingDown = signal(false);
  private readonly list = viewChild.required<ElementRef<HTMLElement>>('list');
  private readonly trash = viewChild.required<ElementRef<HTMLElement>>('trash');

  constructor() {
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      if (!canObserve()) return;
      const list = this.list().nativeElement;
      const watch = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            watch.unobserve(entry.target);
            if (entry.target === list) void this.tick();
            else this.countingDown.set(true);
          }
        },
        { threshold: 0.7 },
      );
      watch.observe(list);
      watch.observe(this.trash().nativeElement);
      destroyRef.onDestroy(() => watch.disconnect());
    });
  }

  private async tick(): Promise<void> {
    const reduce = prefersReducedMotion();
    while (this.ticked() < CHECKLIST.length) {
      if (!reduce) await sleep(650);
      this.ticked.update((count) => count + 1);
    }
  }
}
