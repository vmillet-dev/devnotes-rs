import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  signal,
} from '@angular/core';

import { Language } from '../../core/language';
import { ScrollFrame } from '../../core/scroll-frame';
import { Runs } from '../runs';
import { DayScene, LOOPS } from './day-scene';
import { DAY_TEXT, Scene } from './day-text';

const FIRST = 8 * 60;
const LAST = 19 * 60;

/** The scenes drawn across the row, under their words: the widest pieces, and a change of pace. */
const WIDE: ReadonlySet<Scene> = new Set(['drawing', 'diff']);

const minutes = (time: string) => {
  const [hours = 0, rest = 0] = time.split(':').map(Number);
  return hours * 60 + rest;
};
const hhmm = (at: number) =>
  `${String(Math.floor(at / 60)).padStart(2, '0')}:${String(Math.floor(at % 60)).padStart(2, '0')}`;
/** Where an hour stands on the day's ruler, 0…1. */
const place = (at: number) => (at - FIRST) / (LAST - FIRST);

/**
 * A developer's day, moment by moment. The hour follows the reading point: between two moments
 * it runs from one time to the next. On a wide screen the day's ruler stands in the right margin,
 * each moment named; below, it is a bar under the header.
 */
@Component({
  selector: 'app-day-section',
  imports: [DayScene, Runs],
  templateUrl: './day-section.html',
  styleUrl: './day-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DaySection {
  protected readonly lang = inject(Language).lang;
  protected readonly t = computed(() => DAY_TEXT[this.lang()]);
  protected readonly hours = Array.from({ length: LAST / 60 - FIRST / 60 + 1 }, (_, i) => ({
    label: String(FIRST / 60 + i).padStart(2, '0'),
    minor: i % 2 === 1,
    at: place(FIRST + i * 60),
  }));
  protected readonly marks = computed(() =>
    this.t().moments.map((moment) => ({ kind: moment.kind, at: place(minutes(moment.time)) })),
  );

  /** The moment being read, -1 above the first. */
  protected readonly current = signal(-1);
  protected readonly clock = signal(this.t().moments[0]?.time ?? '');
  protected readonly now = signal(0);
  /** The margin's ruler is shown while the day is on screen. */
  protected readonly shown = signal(false);

  constructor() {
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      const section = host.querySelector('section');
      const rows = [...host.querySelectorAll<HTMLElement>('.mo')];
      const times = this.t().moments.map(({ time }) => minutes(time));
      if (!section || !rows.length) return;

      const stop = frame.watch(() => {
        const line = scrollY + innerHeight * 0.4;
        const tops = rows.map((row) => row.getBoundingClientRect().top + scrollY);
        let at = times[0] ?? FIRST;
        let current = -1;
        for (let i = 0; i < rows.length; i++) {
          const top = tops[i] ?? 0;
          if (line < top) break;
          current = i;
          const next = tops[i + 1];
          const from = times[i] ?? FIRST;
          const to = times[i + 1] ?? from;
          at = next === undefined ? from : from + (to - from) * Math.min(1, (line - top) / (next - top));
        }
        this.current.set(current);
        this.clock.set(hhmm(at));
        this.now.set(Number(place(at).toFixed(4)));
        const box = section.getBoundingClientRect();
        this.shown.set(box.top < innerHeight * 0.25 && box.bottom > innerHeight * 0.5);
      });
      destroyRef.onDestroy(stop);
    });
  }

  protected loop(scene: Scene): number {
    return LOOPS[scene];
  }

  protected wide(scene: Scene): boolean {
    return WIDE.has(scene);
  }
}
