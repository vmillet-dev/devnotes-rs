import { AfterViewInit, Directive, ElementRef, OnDestroy, inject } from '@angular/core';

/**
 * ⚠️ `:not([tabindex="-1"])` on every candidate, not only the last: a `<button tabindex="-1">`
 * is out of the browser's tab order, and the trap has to agree.
 */
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'textarea:not([disabled])',
  'select:not([disabled])',
  '[tabindex]',
]
  .map((selector) => `${selector}:not([tabindex="-1"])`)
  .join(',');

/** Written by hand rather than pulling in `@angular/cdk` for one directive. */
@Directive({
  selector: '[appFocusTrap]',
  host: {
    '(keydown.tab)': 'onTab($event, false)',
    '(keydown.shift.tab)': 'onTab($event, true)',
  },
})
export class FocusTrapDirective implements AfterViewInit, OnDestroy {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private previouslyFocused: HTMLElement | null = null;

  ngAfterViewInit(): void {
    this.previouslyFocused = document.activeElement as HTMLElement | null;
    this.focusableElements()[0]?.focus();
  }

  ngOnDestroy(): void {
    this.previouslyFocused?.focus();
  }

  // `Event` and not `KeyboardEvent`: modifier pseudo-events are absent from the
  // host-binding type table, so `typeCheckHostBindings` hands over an `Event`.
  protected onTab(event: Event, backwards: boolean): void {
    // Something inside has already answered for this key — the palette takes Tab as an
    // arrow — and moving the focus on top of that is the trap undoing what it wrapped.
    if (event.defaultPrevented) return;

    const elements = this.focusableElements();
    if (elements.length === 0) return;

    const first = elements[0];
    const last = elements.at(-1);
    if (!first || !last) return;

    const active = document.activeElement;

    // Only the two edges of the trap need intervention: native navigation does the rest.
    if (backwards && active === first) {
      event.preventDefault();
      last.focus();
    } else if (!backwards && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  private focusableElements(): HTMLElement[] {
    return Array.from(this.host.nativeElement.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
  }
}
