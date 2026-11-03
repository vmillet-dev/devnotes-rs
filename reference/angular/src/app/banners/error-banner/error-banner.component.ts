import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';

@Component({
  selector: 'app-error-banner',
  imports: [TranslocoPipe],
  templateUrl: './error-banner.component.html',
  styleUrl: './error-banner.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ErrorBannerComponent {
  protected readonly notifier = inject(ErrorNotifier);
}
