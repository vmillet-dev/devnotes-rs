import { DestroyRef, Directive, inject, input, output } from '@angular/core';

/** Resting on the entry opens its submenu; crossing it on the way to another opens nothing. */
export const SUBMENU_OPEN_DELAY_MS = 150;
/** Another entry hovered folds it after this grace: the way to its last entry crosses those below. */
export const SUBMENU_CLOSE_DELAY_MS = 300;

/**
 * A submenu that follows the pointer, as a desktop menu's does. Set on the panel holding the
 * entry (`aria-haspopup="menu"`), bound to whether the submenu shows; the submenu, a sibling,
 * calls `reached()` on `pointerenter`. A touch has no hover: the tap stays the way in.
 */
@Directive({
  selector: '[appSubmenuHover]',
  exportAs: 'appSubmenuHover',
  host: {
    '(pointerover)': 'over($event)',
    '(pointerleave)': 'left()',
  },
})
export class SubmenuHoverDirective {
  readonly appSubmenuHover = input.required<boolean>();

  readonly submenuOpened = output<void>();
  /** Carries the entry hovered instead: it takes the focus the submenu held. */
  readonly submenuClosed = output<HTMLElement>();

  private timer: ReturnType<typeof setTimeout> | null = null;
  private pending: 'open' | 'close' | null = null;
  private hovered: HTMLElement | null = null;

  constructor() {
    inject(DestroyRef).onDestroy(() => this.cancel());
  }

  /** The pointer is in the submenu: what it crossed on the way folds nothing. */
  reached(): void {
    if (this.pending === 'close') this.cancel();
  }

  protected over(event: PointerEvent): void {
    if (event.pointerType === 'touch') return;
    const entry = (event.target as Element).closest<HTMLElement>('[appMenuItem]');
    if (!entry) return;

    if (entry.getAttribute('aria-haspopup') === 'menu') {
      if (this.pending === 'close') this.cancel();
      if (!this.appSubmenuHover() && this.pending === null) {
        this.schedule('open', SUBMENU_OPEN_DELAY_MS, () => {
          if (!this.appSubmenuHover()) this.submenuOpened.emit();
        });
      }
      return;
    }

    this.hovered = entry;
    if (this.pending === 'open') this.cancel();
    if (this.appSubmenuHover() && this.pending === null) {
      this.schedule('close', SUBMENU_CLOSE_DELAY_MS, () => {
        if (this.appSubmenuHover() && this.hovered) this.submenuClosed.emit(this.hovered);
      });
    }
  }

  protected left(): void {
    if (this.pending === 'open') this.cancel();
  }

  private schedule(kind: 'open' | 'close', delayMs: number, action: () => void): void {
    this.pending = kind;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.pending = null;
      action();
    }, delayMs);
  }

  private cancel(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.pending = null;
  }
}
