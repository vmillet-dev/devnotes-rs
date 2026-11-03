import { ChangeDetectionStrategy, Component, ElementRef, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { variableCounts } from '@core/model/http-environment.model';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** One line a set, the title and « Gérer… », with room to spare before it opens upward. */
const PANEL_HEIGHT = 260;

/**
 * The environment every tab is sent with, at the end of the tab bar: one for the area, not
 * one a tab. « Aucun » sends with the globals alone.
 */
@Component({
  selector: 'app-environment-picker',
  imports: [MenuPanelDirective, TranslocoPipe],
  templateUrl: './environment-picker.component.html',
  styleUrl: './environment-picker.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  hostDirectives: [MenuTriggerDirective],
})
export class EnvironmentPickerComponent {
  protected readonly store = inject(HttpEnvironmentsStore);
  protected readonly menu = inject(MenuTriggerDirective);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  /** Fixed and measured: the strip of tabs scrolls, and a panel inside it would be cut. */
  protected readonly panelStyle = signal<Record<string, string>>({});

  protected toggle(): void {
    if (!this.menu.open()) {
      this.place();
      if (!this.store.loaded()) void this.store.load();
    }
    this.menu.toggle();
  }

  protected count(id: string): number {
    const environment = this.store.environments().find((held) => held.id === id);
    return environment === undefined ? 0 : variableCounts(environment.variables).count;
  }

  protected choose(id: string | null): void {
    this.menu.close();
    this.store.choose(id);
  }

  protected manage(): void {
    this.menu.close(false);
    this.store.open();
  }

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
