import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { VaultStore } from '@core/state/vault.store';
import { IconComponent } from '@shared/icon/icon.component';
import { AboutMenuComponent } from './about-menu/about-menu.component';
import { FileMenuComponent } from './file-menu/file-menu.component';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';

@Component({
  selector: 'app-titlebar',
  imports: [TranslocoPipe, FileMenuComponent, AboutMenuComponent, IconComponent, AreaSwitchComponent],
  templateUrl: './titlebar.component.html',
  styleUrl: './titlebar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TitlebarComponent {
  protected readonly title = APP_INFO.name;

  protected readonly settings = inject(SettingsStore);
  /** The opposite of what is **on screen**, which on "system" is whatever the OS resolved. */
  protected readonly switchesTo = computed(() =>
    this.settings.resolvedTheme() === 'dark' ? 'light' : 'dark',
  );

  protected readonly switchLabel = computed(() =>
    this.switchesTo() === 'dark' ? 'settings.theme.toDark' : 'settings.theme.toLight',
  );

  protected readonly vault = inject(VaultStore);
  protected readonly areas = inject(AreaStore);

  /** Only the notes can hide their rail, and the switch heads it. */
  protected readonly switchInBar = computed(
    () => this.vault.isUnlocked() && this.areas.current() === 'notes' && !this.settings.showLibraryRail(),
  );

  protected toggleTheme(): void {
    this.settings.setTheme(this.switchesTo());
  }
}
