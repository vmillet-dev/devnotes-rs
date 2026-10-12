import { DOCUMENT, DestroyRef, Directive, ElementRef, afterNextRender, inject } from '@angular/core';
import { MenuTriggerDirective } from './menu-trigger.directive';

const GAP_PX = 4;
/** Kept between the panel and the window's edge. */
const MARGIN_PX = 8;

/**
 * Takes a menu's panel out of whatever would cut it — a scrolling editor, a dialog's body —
 * by putting it in the top layer and placing it from its anchor's rectangle: under it, or above
 * when the room under is short. What moves the anchor closes the menu rather than leave it adrift.
 */
@Directive({
  selector: '[appFloatingPanel]',
  host: { popover: 'manual' },
})
export class FloatingPanelDirective {
  private readonly panel = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
  private readonly menu = inject(MenuTriggerDirective);

  constructor() {
    const view = inject(DOCUMENT).defaultView!;

    // ⚠️ The `write` phase, ahead of `MenuPanelDirective`'s focus: until `showPopover` the
    // panel is `display: none`, and a hidden entry refuses the focus.
    afterNextRender({ write: () => this.place(view) });

    const onScroll = (event: Event): void => {
      const anchor = this.menu.anchor();
      // The panel scrolls its own entries, and a list elsewhere moves nothing here.
      if (anchor && event.target instanceof Node && event.target.contains(anchor)) this.leave();
    };
    const onResize = (): void => this.leave();
    view.addEventListener('scroll', onScroll, { capture: true, passive: true });
    view.addEventListener('resize', onResize);
    inject(DestroyRef).onDestroy(() => {
      view.removeEventListener('scroll', onScroll, { capture: true });
      view.removeEventListener('resize', onResize);
    });
  }

  private leave(): void {
    this.menu.close(false);
    // The anchor may be scrolling out of sight: focused where it is, not brought back.
    this.menu.anchor()?.focus({ preventScroll: true });
  }

  private place(view: Window): void {
    const anchor = this.menu.anchor();
    if (!anchor) return;

    // Absent from jsdom; `position: fixed` alone already leaves a scroll container.
    if (typeof this.panel.showPopover === 'function') this.panel.showPopover();

    const box = anchor.getBoundingClientRect();
    const below = view.innerHeight - box.bottom - GAP_PX - MARGIN_PX;
    const above = box.top - GAP_PX - MARGIN_PX;
    const wanted = this.panel.offsetHeight;
    const opensUp = wanted > below && above > below;
    const room = opensUp ? above : below;
    const left = Math.min(box.left, view.innerWidth - this.panel.offsetWidth - MARGIN_PX);

    const style = this.panel.style;
    if (wanted > room) style.maxHeight = `${Math.max(0, Math.floor(room))}px`;
    style.left = `${Math.round(Math.max(MARGIN_PX, left))}px`;
    if (opensUp) {
      style.bottom = `${Math.round(view.innerHeight - box.top + GAP_PX)}px`;
    } else {
      style.top = `${Math.round(box.bottom + GAP_PX)}px`;
    }
    this.panel.dataset['placement'] = opensUp ? 'above' : 'below';
  }
}
