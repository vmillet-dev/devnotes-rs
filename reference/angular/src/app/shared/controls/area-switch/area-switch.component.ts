import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AREAS, Area } from '@core/services/areas/area.model';
import { IconComponent, IconName } from '@shared/icon/icon.component';

const ICONS: Record<Area, IconName> = { notes: 'file-text', tools: 'wrench', http: 'arrow-left-right' };

/**
 * Heads each area's rail, a label under each icon; `bar` is the same switch in the titlebar,
 * while the rail it headed is hidden.
 */
@Component({
  selector: 'app-area-switch',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './area-switch.component.html',
  styleUrl: './area-switch.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[class.bar]': "layout() === 'bar'" },
})
export class AreaSwitchComponent {
  readonly current = input.required<Area>();
  readonly layout = input<'rail' | 'bar'>('rail');

  readonly chosen = output<Area>();

  protected readonly areas = AREAS.map((id, index) => ({ id, icon: ICONS[id], key: `Ctrl+${index + 1}` }));

  protected choose(area: Area): void {
    if (area !== this.current()) {
      this.chosen.emit(area);
    }
  }
}
