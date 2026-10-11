import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

export type CloseChoice = 'save' | 'discard';

/**
 * A tab is closed after one question when closing it costs something: what was typed and not
 * saved, a WebSocket still connected — or both, said in one dialog and not two in a row.
 */
@Component({
  selector: 'app-close-request-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './close-request-dialog.component.html',
  styleUrl: './close-request-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CloseRequestDialogComponent {
  readonly name = input.required<string>();
  readonly modified = input(true);
  /** Its socket is open or opening: closing the tab cuts it. */
  readonly connected = input(false);
  readonly chosen = output<CloseChoice>();
  readonly cancelled = output<void>();
}
