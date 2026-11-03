import { DestroyRef, Directive, ElementRef, afterNextRender, inject, signal } from '@angular/core';

/**
 * A toolbar that is one row while every control holds in it, and two on purpose past that —
 * never one row that grows because a control was squeezed. Its children are the controls, or
 * groups of them marked `data-group`; a control that may narrow says how far in `data-floor`.
 *
 * Decided from floors, not from the widths drawn: those differ between one row and two, and an
 * answer read off them would flip back and forth.
 */
@Directive({
  selector: '[appTwoRows]',
  exportAs: 'appTwoRows',
  host: { '[class.two-rows]': 'twoRows()' },
})
export class TwoRowsDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly _twoRows = signal(false);
  readonly twoRows = this._twoRows.asReadonly();

  constructor() {
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      if (typeof ResizeObserver === 'undefined') return;
      const bar = this.host.nativeElement;
      const measure = (): void => {
        // A hidden page measures nothing: it keeps the answer it had when last shown.
        if (bar.clientWidth > 0) this._twoRows.set(!oneRowHolds(bar));
      };
      const resized = new ResizeObserver(measure);
      // A control appears or goes (a folder opened, the board shown): watch what is there now.
      const watch = (): void => {
        resized.disconnect();
        resized.observe(bar);
        for (const control of controlsOf(bar)) resized.observe(control);
        measure();
      };
      const changed = new MutationObserver(watch);
      changed.observe(bar, { childList: true, subtree: true });
      watch();
      destroyRef.onDestroy(() => {
        resized.disconnect();
        changed.disconnect();
      });
    });
  }
}

function controlsOf(bar: HTMLElement): HTMLElement[] {
  return [...bar.children].flatMap((child) =>
    child instanceof HTMLElement && child.hasAttribute('data-group')
      ? [...child.children].filter((inner): inner is HTMLElement => inner instanceof HTMLElement)
      : child instanceof HTMLElement
        ? [child]
        : [],
  );
}

/** Every control side by side, the narrowing ones at their floor, inside the padding. */
export function oneRowHolds(bar: HTMLElement): boolean {
  const style = getComputedStyle(bar);
  const px = (value: string): number => parseFloat(value) || 0;
  const controls = controlsOf(bar);
  const widths = controls.map(
    (control) => Number(control.dataset['floor']) || control.getBoundingClientRect().width,
  );
  const need =
    widths.reduce((sum, width) => sum + width, 0) + px(style.columnGap) * Math.max(0, controls.length - 1);
  return need <= bar.clientWidth - px(style.paddingLeft) - px(style.paddingRight);
}
