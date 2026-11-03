import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NoteLifecycle } from '@core/model/note.model';
import { LifecycleBadgeComponent } from '../lifecycle-badge/lifecycle-badge.component';

/** The folded header: what its bands hold, minus what is empty, and the way back to them. */
@Component({
  selector: 'app-header-summary',
  imports: [LifecycleBadgeComponent, TranslocoPipe],
  templateUrl: './header-summary.component.html',
  styleUrl: './header-summary.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HeaderSummaryComponent {
  readonly source = input('');
  readonly space = input.required<string>();
  readonly folder = input<string | null>(null);
  readonly tagCount = input(0);
  readonly attachmentCount = input(0);
  readonly lifecycle = input.required<NoteLifecycle>();
  readonly expiringSoon = input(false);

  readonly unfold = output<void>();
}
