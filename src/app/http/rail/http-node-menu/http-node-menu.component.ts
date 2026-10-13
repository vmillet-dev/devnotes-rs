import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { REQUEST_KINDS, RequestKind } from '@core/model/http.model';
import { RailRow } from '@core/services/http/http-tree';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';
import { SubmenuHoverDirective } from '@shared/directives/submenu-hover.directive';

/** A request is begun as one of its three kinds: there is no « Nouvelle requête » to turn into one later. */
export type HttpNodeAction =
  'newHttp' | 'newGraphql' | 'newWebsocket' | 'newFolder' | 'settings' | 'rename' | 'duplicate' | 'delete';

/** What the panel lists: `newRequest` is no action, it opens the three kinds beside it. */
type Entry = HttpNodeAction | 'newRequest';

/** Six entries and the panel's padding, with room to spare. */
const PANEL_HEIGHT = 220;
/** Three entries and its padding. */
const SUBMENU = { width: 190, height: 110 };
const MARGIN = 8;

const NEW_ACTIONS: Readonly<Record<RequestKind, HttpNodeAction>> = {
  http: 'newHttp',
  graphql: 'newGraphql',
  websocket: 'newWebsocket',
};

/**
 * A row's `⋯`: one menu a row, open only while it is used. The three kinds of request sit
 * behind one entry, as the levels of a card's priority do: listed flat, they made eight
 * entries of a menu a reader opens to rename something.
 */
@Component({
  selector: 'app-http-node-menu',
  imports: [MenuPanelDirective, SubmenuHoverDirective, TranslocoPipe],
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
  private readonly parent = viewChild<ElementRef<HTMLElement>>('parent');

  /** Fixed and measured: the rail scrolls, and a panel inside it would be cut at its edge. */
  protected readonly panelStyle = signal<Record<string, string>>({});
  protected readonly submenuStyle = signal<Record<string, string>>({});
  /** Folded with the menu, and first on Escape. */
  protected readonly submenuOpen = linkedSignal({ source: this.menu.open, computation: () => false });

  protected readonly kinds = REQUEST_KINDS.map((kind) => ({ kind, action: NEW_ACTIONS[kind] }));

  /** A request holds nothing: it is renamed, copied or deleted. */
  protected readonly entries = computed<readonly Entry[]>(() =>
    this.row().kind === 'request'
      ? ['rename', 'duplicate', 'delete']
      : ['newRequest', 'newFolder', 'settings', 'rename', 'duplicate', 'delete'],
  );

  constructor() {
    this.menu.handleEscape(() => (this.submenuOpen() ? this.closeSubmenu() : this.menu.close()));
  }

  protected toggle(): void {
    if (!this.menu.open()) this.place();
    this.menu.toggle();
  }

  protected choose(action: HttpNodeAction): void {
    // Focus goes to what the action opens, a field or a confirmation, not back to the `⋯`.
    this.menu.close(false);
    this.chosen.emit(action);
  }

  protected openSubmenu(event?: Event): void {
    event?.preventDefault();
    this.placeSubmenu();
    this.submenuOpen.set(true);
  }

  protected closeSubmenu(focus = this.parent()?.nativeElement): void {
    this.submenuOpen.set(false);
    focus?.focus();
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

  /** Beside the panel, on the workspace's side: the rail is at the window's left edge. */
  private placeSubmenu(): void {
    const item = this.parent()?.nativeElement.getBoundingClientRect();
    if (!item) return;
    const right = item.right + 10;
    const fits = right + SUBMENU.width + MARGIN <= window.innerWidth;
    const top = Math.max(MARGIN, Math.min(item.top - 6, window.innerHeight - SUBMENU.height - MARGIN));
    this.submenuStyle.set({
      left: `${Math.round(fits ? right : Math.max(MARGIN, item.left - SUBMENU.width - 10))}px`,
      top: `${Math.round(top)}px`,
    });
  }
}
