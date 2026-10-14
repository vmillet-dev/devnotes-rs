import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';

import { clamp, ease, field, part, prefersReducedMotion } from '../core/browser';
import { ScrollFrame } from '../core/scroll-frame';

type Folder = 'dep' | 'db' | 'front';

/** The board is drawn at this width and scaled to its column, never below 60 %. */
const BOARD_WIDTH = 1122;
const SMALLEST = 0.6;

/**
 * Below that, the board is not shrunk any further nor scrolled sideways: the screen shows
 * one folder of it and the one note filed there. The same crop as the stylesheet's default.
 */
const CROP = { left: 350, width: 392 } as const;

/** Where the cursor takes a card: a little inside its top left corner. */
const GRIP: readonly [number, number] = [92, 30];

interface Move {
  readonly folder: Folder;
  readonly from: readonly [number, number];
  readonly to: readonly [number, number];
}

/** The three loose notes, in the order the cursor files them, from where they lie to their folder. */
const MOVES: readonly Move[] = [
  { folder: 'db', from: [360, 400], to: [400, 238] },
  { folder: 'front', from: [528, 400], to: [764, 238] },
  { folder: 'dep', from: [192, 400], to: [36, 238] },
];

interface Placed {
  readonly transform: string;
  readonly lifted: boolean;
  /** Its folder lights up while the card hovers over it. */
  readonly dropping: boolean;
  readonly filed: boolean;
}

const AT_REST: Placed = { transform: 'none', lifted: false, dropping: false, filed: false };

/**
 * The hero: a tidy board, and three notes left at the bottom that a cursor files one by
 * one. The one place the page holds a moment: the board stays centred while the scroll
 * moves the cards, then lets go.
 */
@Component({
  selector: 'app-hero-board',
  templateUrl: './hero-board.html',
  styleUrl: './hero-board.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HeroBoard {
  protected readonly field = field;
  /** 0 with every note loose, 1 with all of them filed. */
  private readonly sorted = signal(0);
  /** A phone sees one folder: only the note filed into it moves. */
  private readonly cropped = signal(false);

  private readonly track = viewChild.required<ElementRef<HTMLElement>>('track');
  private readonly pin = viewChild.required<ElementRef<HTMLElement>>('pin');
  private readonly scroller = viewChild.required<ElementRef<HTMLElement>>('scroller');
  private readonly board = viewChild.required<ElementRef<HTMLElement>>('board');

  protected readonly placed = computed(() => {
    const p = this.sorted();
    const moves = this.cropped() ? MOVES.slice(0, 1) : MOVES;
    const placed: Record<Folder, Placed> = { dep: AT_REST, db: AT_REST, front: AT_REST };
    let pointer: readonly [number, number] | null = null;

    for (const [i, move] of moves.entries()) {
      const local = clamp(p * moves.length - i);
      const carried = ease(part(local, 0.3, 0.85));
      const dx = (move.to[0] - move.from[0]) * carried;
      const dy = (move.to[1] - move.from[1]) * carried;
      const lifted = local > 0.25 && local < 0.9;
      placed[move.folder] = {
        transform: `translate(${dx}px, ${dy}px)${lifted ? ' scale(1.04) rotate(1.5deg)' : ''}`,
        lifted,
        dropping: local > 0.6 && local < 0.92,
        filed: local >= 0.9,
      };
      if (pointer || local <= 0 || local >= 1) continue;

      const grab = [move.from[0] + GRIP[0], move.from[1] + GRIP[1]] as const;
      if (local < 0.25) {
        // On its way to the card: from where it dropped the last one, or from above the first.
        const previous = moves[i - 1];
        const before: readonly [number, number] = previous
          ? [previous.to[0] + GRIP[0], previous.to[1] + GRIP[1]]
          : [grab[0] + 220, grab[1] - 150];
        const t = ease(local / 0.25);
        pointer = [before[0] + (grab[0] - before[0]) * t, before[1] + (grab[1] - before[1]) * t];
      } else {
        pointer = [grab[0] + dx, grab[1] + dy];
      }
    }

    return { cards: placed, cursor: pointer ? `translate(${pointer[0]}px, ${pointer[1]}px)` : null };
  });

  constructor() {
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      const fit = () => {
        const board = this.board().nativeElement;
        const width = this.scroller().nativeElement.clientWidth;
        const cropped = width / BOARD_WIDTH < SMALLEST;
        // The zoom applies to the board's own margin too: the crop is written in board pixels.
        board.style.setProperty(
          'zoom',
          String(cropped ? width / CROP.width : Math.min(1, width / BOARD_WIDTH)),
        );
        board.style.setProperty('margin-left', cropped ? `-${CROP.left}px` : 'auto');
        this.cropped.set(cropped);
        this.track().nativeElement.style.setProperty('--bh', `${this.pin().nativeElement.offsetHeight}px`);
      };
      fit();
      addEventListener('resize', fit);
      destroyRef.onDestroy(() => removeEventListener('resize', fit));

      if (prefersReducedMotion()) {
        this.sorted.set(1);
        return;
      }
      destroyRef.onDestroy(frame.watch(() => this.sorted.set(this.progress())));
    });
  }

  /** How far the cards have moved: only while the board is held in the middle of the screen. */
  private progress(): number {
    const pin = this.pin().nativeElement;
    const track = this.track().nativeElement.getBoundingClientRect();
    const stuckAt = parseFloat(getComputedStyle(pin).top) || 80;
    const releasedAt = stuckAt - (track.height - pin.offsetHeight);
    const from = stuckAt + 40;
    return clamp((from - track.top) / (from - releasedAt));
  }
}
