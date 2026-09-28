import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Priority } from '@core/model/note.model';
import { PRIORITY_BARS } from '@core/model/priority.model';

/** Nothing at `none`: a card says a priority only when it has one. */
@Component({
  selector: 'app-priority-pill',
  imports: [TranslocoPipe],
  templateUrl: './priority-pill.component.html',
  styleUrl: './priority-pill.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PriorityPillComponent {
  readonly priority = input.required<Priority>();

  protected readonly bars = computed(() => [0, 1, 2].map((bar) => bar < PRIORITY_BARS[this.priority()]));
}
