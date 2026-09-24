import { Directive, ElementRef, afterNextRender, inject } from '@angular/core';

/**
 * Keyboard navigation for a `role="menu"` panel. The initial focus is load-bearing —
 * a menu opened without it is unreachable from the keyboard — and is set by
 * `afterNextRender`, once the `@if` has created the panel.
 */
@Directive({
  selector: '[appMenuPanel]',
  host: {
    role: 'menu',
    '(keydown)': 'onKeydown($event)',
  },
})
export class MenuPanelDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  constructor() {
    afterNextRender(() => this.items()[0]?.focus());
  }

  protected onKeydown(event: KeyboardEvent): void {
    const items = this.items();
    if (items.length === 0) return;

    const current = items.indexOf(document.activeElement as HTMLElement);
    const focusAt = (index: number): void => {
      event.preventDefault();
      items[(index + items.length) % items.length]?.focus();
    };

    switch (event.key) {
      case 'ArrowDown':
        focusAt(current + 1);
        break;
      case 'ArrowUp':
        focusAt(current - 1);
        break;
      case 'Home':
        focusAt(0);
        break;
      case 'End':
        focusAt(items.length - 1);
        break;
    }
  }

  private items(): HTMLElement[] {
    return Array.from(this.host.nativeElement.querySelectorAll<HTMLElement>('[appMenuItem]'));
  }
}
