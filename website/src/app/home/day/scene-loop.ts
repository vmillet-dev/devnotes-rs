import { DestroyRef, Directive, ElementRef, afterNextRender, inject, input, output } from '@angular/core';

import { canObserve, prefersReducedMotion } from '../../core/browser';

/**
 * Plays a scene again and again while it is on screen (`_pieces.scss` says how each piece moves).
 * Without script, under reduced motion, or where nothing observes, the scene keeps the end it is
 * written in. Typing and revealing go a character at a time: each piece is told how many it has.
 */
@Directive({
  selector: '[appSceneLoop]',
  host: { class: 'scene' },
})
export class SceneLoop {
  /** How long one turn lasts, the pause at its end included, in milliseconds. */
  readonly appSceneLoop = input.required<number>();
  /** Each time the scene starts again: what CSS cannot move is moved by its listener. */
  readonly replay = output<void>();

  constructor() {
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      if (prefersReducedMotion() || !canObserve()) return;
      for (const piece of host.querySelectorAll<HTMLElement>('.ty, .ph, .rv'))
        piece.style.setProperty('--n', String([...(piece.textContent ?? '')].length));
      host.classList.add('anim');

      let timer = 0;
      const restart = () => {
        host.classList.remove('run');
        void host.offsetWidth;
        host.classList.add('run');
        this.replay.emit();
        timer = window.setTimeout(restart, this.appSceneLoop());
      };
      const watch = new IntersectionObserver(
        ([entry]) => {
          clearTimeout(timer);
          if (entry?.isIntersecting) restart();
          else host.classList.remove('run');
        },
        { threshold: 0.2 },
      );
      watch.observe(host);
      destroyRef.onDestroy(() => {
        clearTimeout(timer);
        watch.disconnect();
      });
    });
  }
}
