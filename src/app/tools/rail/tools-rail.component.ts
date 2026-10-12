import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AreaStore } from '@core/services/areas/area.store';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import {
  MAX_RECENT_TOOLS,
  TOOL_CATALOGUE,
  TOOL_CATEGORIES,
  ToolDefinition,
} from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';
import { RailFrameComponent } from '@shared/layout/rail-frame/rail-frame.component';

/**
 * At home, the categories and their counts; in a tool, its category's tools and the recent
 * ones. In the frame the three areas share, so going from one to the other moves nothing.
 */
@Component({
  selector: 'app-tools-rail',
  imports: [AreaSwitchComponent, RailFrameComponent, TranslocoPipe],
  templateUrl: './tools-rail.component.html',
  styleUrl: './tools-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolsRailComponent {
  protected readonly areas = inject(AreaStore);
  protected readonly settings = inject(SettingsStore);
  protected readonly store = inject(ToolsStore);
  protected readonly catalogue = inject(TOOL_CATALOGUE);
  protected readonly bounds = RAIL_WIDTH;

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

  /**
   * The ones not already listed above them — no tool twice in one rail — and five all the
   * same: more are kept than shown, so that the open tool and its neighbours cost none.
   */
  protected readonly recents = computed(() => {
    const listed = new Set(this.siblings().map((tool) => tool.id));
    return this.store
      .recents()
      .map((recent) => this.catalogue.find((tool) => tool.id === recent.id))
      .filter((tool): tool is ToolDefinition => tool !== undefined && !listed.has(tool.id))
      .slice(0, MAX_RECENT_TOOLS);
  });
}
