import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { VaultStore } from '@core/state/vault.store';
import { IconComponent } from '@shared/icon/icon.component';
import { AboutMenuComponent } from './about-menu/about-menu.component';
import { FileMenuComponent } from './file-menu/file-menu.component';

@Component({
  selector: 'app-titlebar',
  imports: [TranslocoPipe, FileMenuComponent, AboutMenuComponent, IconComponent],
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

  protected toggleTheme(): void {
    this.settings.setTheme(this.switchesTo());
  }
}
