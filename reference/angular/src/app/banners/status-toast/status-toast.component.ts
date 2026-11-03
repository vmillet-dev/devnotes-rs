import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { StatusNotifier } from '@core/services/notifications/status.service';

@Component({
  selector: 'app-status-toast',
  imports: [TranslocoPipe],
  templateUrl: './status-toast.component.html',
  styleUrl: './status-toast.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StatusToastComponent {
  protected readonly notifier = inject(StatusNotifier);
}
