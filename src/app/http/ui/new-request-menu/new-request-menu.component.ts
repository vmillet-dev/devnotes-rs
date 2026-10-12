import { ChangeDetectionStrategy, Component, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { REQUEST_KINDS, RequestKind, requestBadge } from '@core/model/http.model';
import { FloatingPanelDirective } from '@shared/directives/floating-panel.directive';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/**
 * The tab bar's « Nouvelle requête », a split button as the notes' « Nouvelle note » is: an
 * HTTP request keeps one click, picking GraphQL or a WebSocket takes two. They were the last
 * two entries of the method menu — a kind filed among verbs — and were not found; then a bare
 * « + » that said nothing of what it began.
 */
@Component({
  selector: 'app-new-request-menu',
  imports: [FloatingPanelDirective, MenuPanelDirective, TranslocoPipe],
  templateUrl: './new-request-menu.component.html',
  styleUrl: './new-request-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  hostDirectives: [MenuTriggerDirective],
})
export class NewRequestMenuComponent {
  readonly chosen = output<RequestKind>();

  protected readonly menu = inject(MenuTriggerDirective);

  /** Each kind under the badge its tab and its row will wear. */
  protected readonly kinds = REQUEST_KINDS.map((kind) => ({
    kind,
    badge: requestBadge({ kind, method: 'GET' }),
  }));

  protected choose(kind: RequestKind): void {
    // Focus goes to the request that opens, not back to the button.
    this.menu.close(false);
    this.chosen.emit(kind);
  }
}
