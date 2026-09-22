import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoService, TranslocoPipe } from '@jsverse/transloco';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import { APP_LOCALES } from '@core/services/i18n/locale.model';
import { THEME_CHOICES, ThemeChoice } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { LocaleService } from '@core/services/i18n/locale.service';
import { VaultStore } from '@core/state/vault.store';
import { AboutMenuComponent } from './about-menu/about-menu.component';
import { FileMenuComponent } from './file-menu/file-menu.component';

@Component({
  selector: 'app-titlebar',
  imports: [TranslocoPipe, FileMenuComponent, AboutMenuComponent],
  templateUrl: './titlebar.component.html',
  styleUrl: './titlebar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TitlebarComponent {
  protected readonly title = APP_INFO.name;

  protected readonly locales = APP_LOCALES;
  protected readonly settings = inject(SettingsStore);
  private readonly transloco = inject(TranslocoService);

  /**
   * ⚠️ A glyph and not a word: the language beside it is two letters, and a label here would
   * be the widest thing in a bar whose middle is the application's name. The word reaches a
   * screen reader through the accessible label, and it has to — the glyph carries the whole
   * of the state.
   */
  protected readonly glyphs: Record<ThemeChoice, string> = {
    system: '◐',
    dark: '●',
    light: '○',
  };

  protected readonly themeName = computed(() =>
    this.transloco.translate(`settings.theme.${this.settings.theme()}`),
  );

  /** ⚠️ In `THEME_CHOICES` order, which is the preferences panel's: one list, no drift. */
  protected cycleTheme(): void {
    const at = THEME_CHOICES.indexOf(this.settings.theme());
    const next = THEME_CHOICES[(at + 1) % THEME_CHOICES.length];
    if (next) this.settings.setTheme(next);
  }
  protected readonly localeService = inject(LocaleService);
  protected readonly vault = inject(VaultStore);
}
