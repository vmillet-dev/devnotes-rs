import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  OnInit,
  computed,
  inject,
  input,
  output,
} from '@angular/core';
import { DialogBackdropDirective } from '@shared/directives/dialog-backdrop.directive';
import { DialogLayer, DialogVariant, dialogRung } from './dialog.model';
import { DialogStack } from './dialog-stack';
import { FocusTrapDirective } from '@shared/directives/focus-trap.directive';

/**
 * The frame every modal shares: scrim, focus trap, `role="dialog"`, Escape and the
 * backdrop click. What a shell cannot guess — width, gap, padding — are CSS custom
 * properties a consumer sets on the `app-dialog` element, never an `input()`.
 */
@Component({
  selector: 'app-dialog',
  imports: [DialogBackdropDirective, FocusTrapDirective],
  templateUrl: './dialog.component.html',
  styleUrl: './dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(document:keydown.escape)': 'dismiss()',
  },
})
export class DialogComponent implements OnInit {
  private readonly stack = inject(DialogStack);

  readonly layer = input.required<DialogLayer>();
  readonly variant = input<DialogVariant>('fitted');
  /** Fills the window: the same panel, without the frame that bounds it. */
  readonly fullscreen = input(false);

  readonly labelledBy = input<string>();
  readonly label = input<string>();

  /** `false` while an operation cannot be interrupted. */
  readonly dismissible = input(true);

  readonly closed = output<void>();
  /** Before `closed`, when the backdrop was clicked: where. */
  readonly dismissedAt = output<MouseEvent>();

  protected readonly rung = computed(() => dialogRung(this.layer()));

  constructor() {
    inject(DestroyRef).onDestroy(() => this.stack.remove(this));
  }

  /**
   * Joining the stack is the moment the dialog opens: an `@if` in the page is what puts
   * one on screen. ⚠️ `ngOnInit` and not the constructor — `layer` is a required input
   * and has no value yet while the component is being built.
   */
  ngOnInit(): void {
    this.stack.push(this, this.rung());
  }

  protected dismiss(click?: MouseEvent): void {
    if (this.dismissible() && this.stack.isFront(this)) {
      if (click) this.dismissedAt.emit(click);
      this.closed.emit();
    }
  }
}
