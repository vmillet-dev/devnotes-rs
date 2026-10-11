import { ChangeDetectionStrategy, Component, ElementRef, inject, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { REQUEST_KINDS, RequestKind } from '@core/model/http.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** Three entries and the panel's padding, with room to spare. */
const PANEL_HEIGHT = 130;

/**
 * The tab bar's « + »: what kind of request to begin. GraphQL and WebSocket were the last two
 * entries of the method menu, a kind filed among verbs, and were not found. The first entry
 * holds the focus as the menu opens: « + » then Enter is an HTTP request, as fast as before.
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

  protected readonly kinds = REQUEST_KINDS;
  /** Fixed and measured: the strip of tabs scrolls, and a panel inside it would be cut. */
  protected readonly panelStyle = signal<Record<string, string>>({});

  protected toggle(): void {
    if (!this.menu.open()) this.place();
    this.menu.toggle();
  }

  protected choose(kind: RequestKind): void {
    // Focus goes to the request that opens, not back to the « + ».
    this.menu.close(false);
    this.chosen.emit(kind);
  }

  private place(): void {
    const box = this.host.nativeElement.getBoundingClientRect();
    const below = window.innerHeight - box.bottom;
    this.panelStyle.set({
      left: `${Math.round(Math.min(box.left, window.innerWidth - 228))}px`,
      ...(below < PANEL_HEIGHT && box.top > below
        ? { bottom: `${Math.round(window.innerHeight - box.top + 4)}px` }
        : { top: `${Math.round(box.bottom + 4)}px` }),
    });
  }
}
