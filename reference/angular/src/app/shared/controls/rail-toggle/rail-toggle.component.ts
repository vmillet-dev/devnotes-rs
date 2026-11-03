import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { IconComponent } from '@shared/icon/icon.component';

/**
 * Hides and shows an area's rail: the same button at the left end of each area's top bar,
 * since the rail it brings back is not there to carry one.
 */
@Component({
  selector: 'app-rail-toggle',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './rail-toggle.component.html',
  styleUrl: './rail-toggle.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RailToggleComponent {
  readonly shown = input.required<boolean>();
  /** One a page: the notes page stays in the document behind the other areas. */
  readonly testId = input('rail-toggle');

  readonly toggled = output<void>();
}
