import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TOOL_CATALOGUE } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { TOOLS } from './catalogue/catalogue';
import { ToolFrameComponent } from './frame/tool-frame.component';
import { ToolsHomeComponent } from './home/tools-home.component';
import { ToolsRailComponent } from './rail/tools-rail.component';

/** The Outils area: its rail beside the page, as the library's is beside the canvas. */
@Component({
  selector: 'app-tools-page',
  imports: [ToolsRailComponent, ToolsHomeComponent, ToolFrameComponent],
  templateUrl: './tools-page.component.html',
  styleUrl: './tools-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [{ provide: TOOL_CATALOGUE, useValue: TOOLS }],
})
export class ToolsPageComponent {
  private readonly store = inject(ToolsStore);
  private readonly catalogue = inject(TOOL_CATALOGUE);

  protected readonly openTool = computed(
    () => this.catalogue.find((tool) => tool.id === this.store.openId()) ?? null,
  );
}
