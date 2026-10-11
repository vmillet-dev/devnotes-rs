import { ChangeDetectionStrategy, Component, ElementRef, inject, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { REQUEST_KINDS, RequestKind, requestBadge } from '@core/model/http.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** Three entries of two lines, the title and the panel's padding, with room to spare. */
const PANEL_HEIGHT = 230;

/**
 * The tab bar's « Nouvelle requête », a split button as the notes' « Nouvelle note » is: an
 * HTTP request keeps one click, picking GraphQL or a WebSocket takes two. They were the last
 * two entries of the method menu — a kind filed among verbs — and were not found; then a bare
 * « + » that said nothing of what it began.
 */
@Component({
  selector: 'app-new-request-menu',
  imports: [MenuPanelDirective, TranslocoPipe],
  templateUrl: './new-request-menu.component.html',
  styleUrl: './new-request-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  hostDirectives: [MenuTriggerDirective],
})
export class NewRequestMenuComponent {
  readonly chosen = output<RequestKind>();

  protected readonly menu = inject(MenuTriggerDirective);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  /** Each kind under the badge its tab and its row will wear. */
  protected readonly kinds = REQUEST_KINDS.map((kind) => ({
    kind,
    badge: requestBadge({ kind, method: 'GET' }),
  }));
  /** Fixed and measured: the strip of tabs scrolls, and a panel inside it would be cut. */
  protected readonly panelStyle = signal<Record<string, string>>({});

  protected toggle(): void {
    if (!this.menu.open()) this.place();
    this.menu.toggle();
  }

  protected choose(kind: RequestKind): void {
    // Focus goes to the request that opens, not back to the button.
    this.menu.close(false);
    this.chosen.emit(kind);
  }

  /** Under the button and flush with its right edge: it stands at the end of the bar. */
  private place(): void {
    const box = this.host.nativeElement.getBoundingClientRect();
    const below = window.innerHeight - box.bottom;
    this.panelStyle.set({
      right: `${Math.round(Math.max(window.innerWidth - box.right, 8))}px`,
      ...(below < PANEL_HEIGHT && box.top > below
        ? { bottom: `${Math.round(window.innerHeight - box.top + 4)}px` }
        : { top: `${Math.round(box.bottom + 4)}px` }),
    });
  }
}
