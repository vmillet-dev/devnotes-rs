import { DOCUMENT, Directive, ElementRef, effect, inject, output, signal } from '@angular/core';

/**
 * A dropdown's open/closed state. Focus returns to `[appMenuAnchor]` on close, which it
 * would otherwise lose to `<body>`. Escape closes the menu unless the host hands over a
 * handler of its own — a multi-level menu folds its panel first.
 */
@Directive({
  selector: '[appMenuTrigger]',
  exportAs: 'appMenu',
  host: { '(keydown.escape)': 'onEscape($event)' },
})
export class MenuTriggerDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly _open = signal(false);
  readonly open = this._open.asReadonly();

  readonly closed = output<void>();

  constructor() {
    const document = inject(DOCUMENT);

    // ⚠️ Not a host `(document:click)`: every card carries a menu, and Angular marks a
    // listener's view dirty before calling it, so each click re-rendered every card.
    effect((onCleanup) => {
      if (!this._open()) return;

      const closeIfOutside = (event: MouseEvent): void => {
        if (!this.host.nativeElement.contains(event.target as Node)) this.close(false);
      };
      document.addEventListener('click', closeIfOutside);
      onCleanup(() => document.removeEventListener('click', closeIfOutside));
    });
  }

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

  /** An open menu takes the whole keystroke: a dialog behind it, the editor, must not close too. */
  protected onEscape(event: Event): void {
    if (this._open()) event.stopPropagation();
    this.escapeHandler(event);
  }
}
