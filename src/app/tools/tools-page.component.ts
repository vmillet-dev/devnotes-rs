import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';

/** The Outils area: its rail beside the page, as the library's is beside the canvas. */
@Component({
  selector: 'app-tools-page',
  imports: [AreaSwitchComponent, TranslocoPipe],
  templateUrl: './tools-page.component.html',
  styleUrl: './tools-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolsPageComponent {
  protected readonly areas = inject(AreaStore);
  /** The library rail's width: going from one area to the other moves nothing. */
  protected readonly settings = inject(SettingsStore);
}
