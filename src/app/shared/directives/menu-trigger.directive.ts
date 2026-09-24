import { Directive, ElementRef, inject, output, signal } from '@angular/core';

/**
 * A dropdown's open/closed state. Focus returns to `[appMenuAnchor]` on close, which it
 * would otherwise lose to `<body>`. Escape closes the menu unless the host hands over a
 * handler of its own — a multi-level menu folds its panel first.
 */
@Directive({
  selector: '[appMenuTrigger]',
  exportAs: 'appMenu',
  host: {
    '(document:click)': 'onDocumentClick($event)',
    '(keydown.escape)': 'onEscape($event)',
  },
})
export class MenuTriggerDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly _open = signal(false);
  readonly open = this._open.asReadonly();

  readonly closed = output<void>();

  private escapeHandler: (event: Event) => void = () => this.close();

  /** Replaces what Escape does. One handler, so nothing depends on listener order. */
  handleEscape(handler: (event: Event) => void): void {
    this.escapeHandler = handler;
  }

  toggle(): void {
    if (this._open()) {
      this.close();
      return;
    }
    this._open.set(true);
  }

  /** Left `false` when closing opens something else that will take focus. */
  close(restoreFocus = true): void {
    if (!this._open()) return;

    this._open.set(false);
    this.closed.emit();
    if (restoreFocus) {
      this.focusAnchor();
    }
  }

  focusAnchor(): void {
    this.host.nativeElement.querySelector<HTMLElement>('[appMenuAnchor]')?.focus();
  }

  protected onEscape(event: Event): void {
    this.escapeHandler(event);
  }

  protected onDocumentClick(event: MouseEvent): void {
    if (this._open() && !this.host.nativeElement.contains(event.target as Node)) {
      this.close(false);
    }
  }
}
