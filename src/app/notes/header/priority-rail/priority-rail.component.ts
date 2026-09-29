import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FacetCount, Priority } from '@core/model/note.model';

/**
 * The levels from the most pressing down, as the mockup reads them; "none" has no chip, and
 * the rail stays away from a space where nothing has a priority.
 */
@Component({
  selector: 'app-priority-rail',
  imports: [TranslocoPipe],
  templateUrl: './priority-rail.component.html',
  styleUrl: './priority-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PriorityRailComponent {
  readonly counts = input.required<readonly FacetCount<Priority>[]>();
  readonly activePriorities = input.required<ReadonlySet<Priority>>();

  readonly priorityToggled = output<Priority>();
  readonly allChosen = output<void>();

  protected readonly levels = computed(() =>
    this.counts()
      .filter((counted) => counted.value !== 'none')
      .reverse(),
  );
  protected readonly total = computed(() => this.counts().reduce((sum, counted) => sum + counted.count, 0));
  protected readonly everyLevel = computed(() => this.activePriorities().size === 0);
  protected readonly shown = computed(
    () => !this.everyLevel() || this.levels().some((counted) => counted.count > 0),
  );
}
