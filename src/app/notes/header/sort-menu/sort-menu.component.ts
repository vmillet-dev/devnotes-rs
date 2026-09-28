import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  Arrangement,
  GROUPINGS,
  SORT_DIRECTIONS,
  SORT_KEYS,
  naturalOrder,
} from '@core/model/arrangement.model';
import { Grouping, NoteOrder, SortDirection, SortKey } from '@core/model/note.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

const HINTS: Readonly<Record<SortKey, string | null>> = {
  modified: 'sort.hints.modified',
  created: null,
  priority: null,
  format: 'sort.hints.format',
  title: 'sort.hints.title',
};

/**
 * "Trier et regrouper": the button names the order the canvas is in, the menu changes it. It
 * stays open on a choice — order, direction and grouping are set together — and a new sort key
 * starts the way it reads first.
 */
@Component({
  selector: 'app-sort-menu',
  imports: [TranslocoPipe, MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './sort-menu.component.html',
  styleUrl: './sort-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SortMenuComponent {
  readonly arrangement = input.required<Arrangement>();

  readonly orderChosen = output<NoteOrder>();
  readonly groupingChosen = output<Grouping>();
  readonly pinnedFirstChosen = output<boolean>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly keys = SORT_KEYS;
  protected readonly directions = SORT_DIRECTIONS;
  protected readonly groupings = GROUPINGS;
  protected readonly hints = HINTS;

  protected chooseKey(key: SortKey): void {
    if (key !== this.arrangement().order.key) this.orderChosen.emit(naturalOrder(key));
  }

  protected chooseDirection(direction: SortDirection): void {
    const order = this.arrangement().order;
    if (direction !== order.direction) this.orderChosen.emit({ key: order.key, direction });
  }

  protected chooseGrouping(grouping: Grouping): void {
    if (grouping !== this.arrangement().grouping) this.groupingChosen.emit(grouping);
  }
}
