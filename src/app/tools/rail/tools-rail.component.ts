import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { TOOL_CATALOGUE, TOOL_CATEGORIES, ToolDefinition } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';

/**
 * At home, the categories and their counts; in a tool, its category's tools and the recent
 * ones. The library rail's width, so going from one area to the other moves nothing.
 */
@Component({
  selector: 'app-tools-rail',
  imports: [AreaSwitchComponent, TranslocoPipe],
  templateUrl: './tools-rail.component.html',
  styleUrl: './tools-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolsRailComponent {
  protected readonly areas = inject(AreaStore);
  protected readonly settings = inject(SettingsStore);
  protected readonly store = inject(ToolsStore);
  protected readonly catalogue = inject(TOOL_CATALOGUE);

  protected readonly openTool = computed(
    () => this.catalogue.find((tool) => tool.id === this.store.openId()) ?? null,
  );

  /** A category with nothing in it is not offered. */
  protected readonly categories = TOOL_CATEGORIES.map((id) => ({
    id,
    count: this.catalogue.filter((tool) => tool.category === id).length,
  })).filter((category) => category.count > 0);

  protected readonly siblings = computed(() => {
    const open = this.openTool();
    return open ? this.catalogue.filter((tool) => tool.category === open.category) : [];
  });

  /** The ones not already listed above them. */
  protected readonly recents = computed(() => {
    const listed = new Set(this.siblings().map((tool) => tool.id));
    return this.store
      .recents()
      .map((recent) => this.catalogue.find((tool) => tool.id === recent.id))
      .filter((tool): tool is ToolDefinition => tool !== undefined && !listed.has(tool.id));
  });
}
