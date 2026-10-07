import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

export type CloseChoice = 'save' | 'discard';

/** A modified tab is closed after one question: what becomes of what was typed. */
@Component({
  selector: 'app-close-request-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './close-request-dialog.component.html',
  styleUrl: './close-request-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CloseRequestDialogComponent {
  readonly name = input.required<string>();
  readonly chosen = output<CloseChoice>();
  readonly cancelled = output<void>();
}
