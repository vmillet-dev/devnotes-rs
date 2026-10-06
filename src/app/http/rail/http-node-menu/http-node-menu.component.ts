import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { RailRow } from '@core/services/http/http-tree';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

export type HttpNodeAction = 'newRequest' | 'newFolder' | 'rename' | 'duplicate' | 'delete';

/** A row's `⋯`: one menu a row, open only while it is used. */
@Component({
  selector: 'app-http-node-menu',
  imports: [MenuPanelDirective, TranslocoPipe],
  templateUrl: './http-node-menu.component.html',
  styleUrl: './http-node-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  hostDirectives: [MenuTriggerDirective],
})
export class HttpNodeMenuComponent {
  readonly row = input.required<RailRow>();
  readonly chosen = output<HttpNodeAction>();

  protected readonly menu = inject(MenuTriggerDirective);

  /** A request holds nothing: it is renamed, copied or deleted. */
  protected readonly actions = computed<readonly HttpNodeAction[]>(() =>
    this.row().kind === 'request'
      ? ['rename', 'duplicate', 'delete']
      : ['newRequest', 'newFolder', 'rename', 'duplicate', 'delete'],
  );

  protected choose(action: HttpNodeAction): void {
    // Focus goes to what the action opens, a field or a confirmation, not back to the `⋯`.
    this.menu.close(false);
    this.chosen.emit(action);
  }
}
