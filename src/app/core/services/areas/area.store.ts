import { Injectable, Signal, inject } from '@angular/core';
import { SettingsStore } from '@core/services/settings/settings.store';
import { Area } from './area.model';

/** A state the window is in, like the rail: remembered with the application, not the library. */
@Injectable({ providedIn: 'root' })
export class AreaStore {
  private readonly settings = inject(SettingsStore);

  readonly current: Signal<Area> = this.settings.area;

  show(area: Area): void {
    if (area !== this.current()) {
      this.settings.area.write(area);
    }
  }
}
