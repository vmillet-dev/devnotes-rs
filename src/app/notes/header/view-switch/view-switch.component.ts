import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NotesViewMode } from '@core/model/board.model';

/**
 * Two views, not a replacement: the list view stays the default, and the board is another
 * way to look at the same notes.
 */
@Component({
  selector: 'app-view-switch',
  imports: [TranslocoPipe],
  templateUrl: './view-switch.component.html',
  styleUrl: './view-switch.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ViewSwitchComponent {
  readonly mode = input.required<NotesViewMode>();
  /** A folder belongs to a space, so there is no board across all of them. */
  readonly boardAvailable = input(true);

  readonly modeChanged = output<NotesViewMode>();
}
