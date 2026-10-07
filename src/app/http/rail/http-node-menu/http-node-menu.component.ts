import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { RailRow } from '@core/services/http/http-tree';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

export type HttpNodeAction = 'newRequest' | 'newFolder' | 'rename' | 'duplicate' | 'delete';

/** Five entries and the panel's padding, with room to spare. */
const PANEL_HEIGHT = 190;

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
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  /** Fixed and measured: the rail scrolls, and a panel inside it would be cut at its edge. */
  protected readonly panelStyle = signal<Record<string, string>>({});

  /** A request holds nothing: it is renamed, copied or deleted. */
  protected readonly actions = computed<readonly HttpNodeAction[]>(() =>
    this.row().kind === 'request'
      ? ['rename', 'duplicate', 'delete']
      : ['newRequest', 'newFolder', 'rename', 'duplicate', 'delete'],
  );

  protected toggle(): void {
    if (!this.menu.open()) this.place();
    this.menu.toggle();
  }

  protected choose(action: HttpNodeAction): void {
    // Focus goes to what the action opens, a field or a confirmation, not back to the `⋯`.
    this.menu.close(false);
    this.chosen.emit(action);
  }

  private place(): void {
    const box = this.host.nativeElement.getBoundingClientRect();
    const below = window.innerHeight - box.bottom;
    this.panelStyle.set({
      left: `${Math.round(box.left)}px`,
      ...(below < PANEL_HEIGHT && box.top > below
        ? { bottom: `${Math.round(window.innerHeight - box.top + 4)}px` }
        : { top: `${Math.round(box.bottom + 4)}px` }),
    });
  }
}
