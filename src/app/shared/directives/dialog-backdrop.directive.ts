import { Directive, output } from '@angular/core';

/**
 * Emits `dismissed` when the click lands on the backdrop and not the panel above it.
 * Here rather than in the template, which would need `click-events-have-key-events`
 * disabled: the keyboard equivalent is Escape.
 */
@Directive({
  selector: '[appDialogBackdrop]',
  host: {
    '(click)': 'onClick($event)',
  },
})
export class DialogBackdropDirective {
  /** The click itself: what lies under the scrim where it landed may be what was meant. */
  readonly dismissed = output<MouseEvent>();

  protected onClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      this.dismissed.emit(event);
    }
  }
}
