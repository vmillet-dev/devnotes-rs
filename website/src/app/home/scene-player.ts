import { Injectable, signal } from '@angular/core';

import { sleep } from '../core/browser';

/** The pause between two steps, so the eye can read the one that ended. */
const BETWEEN_STEPS_MS = 350;

/**
 * The clock of one feature's scene: which step it is on, and how far into it (`t`, 0…1).
 * A scene is a pure function of the two, so a step clicked, a replay and the reduced-motion
 * end state are all the same render. One per feature, provided by `Feature`.
 */
@Injectable()
export class ScenePlayer {
  /**
   * The end of the first step, not its start: that is what the HTML written at build time
   * shows, readable without a script. Once running, a scene off-screen rests at 0.
   */
  private readonly _step = signal(0);
  private readonly _t = signal(1);
  readonly step = this._step.asReadonly();
  readonly t = this._t.asReadonly();

  private run = 0;

  rest(): void {
    this.run++;
    this.render(0, 0);
  }

  show(step: number): void {
    this.run++;
    this.render(step, 1);
  }

  /** Every step in turn, each over its own duration. A later call or a click stops it. */
  async play(durations: readonly number[]): Promise<void> {
    const mine = ++this.run;
    const alive = () => mine === this.run;

    for (const [step, duration] of durations.entries()) {
      const began = performance.now();
      await new Promise<void>((done) => {
        const tick = (now: number) => {
          if (!alive()) return done();
          const t = Math.min(1, (now - began) / duration);
          this.render(step, t);
          if (t < 1) requestAnimationFrame(tick);
          else done();
        };
        requestAnimationFrame(tick);
      });
      if (!alive()) return;
      if (step < durations.length - 1) await sleep(BETWEEN_STEPS_MS);
      if (!alive()) return;
    }
  }

  private render(step: number, t: number): void {
    this._step.set(step);
    this._t.set(t);
  }
}
