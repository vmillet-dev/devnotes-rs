import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpLeaveStore } from '@core/services/http/http-leave.store';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { SaveAnswer, SaveRequestDialogComponent } from '../save-request/save-request-dialog.component';

/**
 * The question before the page goes away — a quit, another library, a backup restored, an
 * update — with requests not saved in it. One dialog for all of them, in the frame a single
 * tab's closing already has; drawn by the shell, since any area can be the one on screen.
 */
@Component({
  selector: 'app-leave-requests-dialog',
  imports: [DialogComponent, SaveRequestDialogComponent, TranslocoPipe],
  templateUrl: './leave-requests-dialog.component.html',
  styleUrl: './leave-requests-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LeaveRequestsDialogComponent {
  protected readonly leave = inject(HttpLeaveStore);

  protected place(answer: SaveAnswer): void {
    void this.leave.place(answer, answer.name);
  }
}
