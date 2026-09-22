import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoService, TranslocoPipe } from '@jsverse/transloco';
import {
  DENSITIES,
  Density,
  LOCALE_CHOICES,
  LocaleChoice,
  THEME_CHOICES,
  ThemeChoice,
} from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

function checkedValue(event: Event): boolean {
  return (event.target as HTMLInputElement).checked;
}

/**
 * What is left once the keys and the library's own protection have pages of their own:
 * how the application looks, how it behaves, and what it says.
 */
@Component({
  selector: 'app-settings-page',
  imports: [TranslocoPipe, ChoiceMenuComponent, SegmentedChoiceComponent],
  templateUrl: './settings-page.component.html',
  styleUrl: './settings-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsPageComponent {
  protected readonly settings = inject(SettingsStore);

  private readonly transloco = inject(TranslocoService);

  protected readonly localeChoices = computed<readonly ChoiceOption[]>(() =>
    LOCALE_CHOICES.map((choice) => ({ id: choice, name: this.transloco.translate(`locale.${choice}`) })),
  );

  protected readonly themeSegments = computed<readonly Segment[]>(() =>
    THEME_CHOICES.map((choice) => ({
      id: choice,
      label: this.transloco.translate(`settings.theme.${choice}`),
    })),
  );

  protected readonly densitySegments = computed<readonly Segment[]>(() =>
    DENSITIES.map((choice) => ({
      id: choice,
      label: this.transloco.translate(`settings.density.${choice}`),
    })),
  );

  protected onUpdateNotifications(event: Event): void {
    this.settings.setUpdateNotifications(checkedValue(event));
  }

  protected forgetSkippedUpdate(): void {
    this.settings.setSkippedUpdate('');
  }

  protected onLocale(locale: string | null): void {
    if (locale) this.settings.setLocale(locale as LocaleChoice);
  }

  protected onTheme(theme: string): void {
    this.settings.setTheme(theme as ThemeChoice);
  }

  protected onDensity(density: string): void {
    this.settings.setDensity(density as Density);
  }

  protected onStartWithSystem(event: Event): void {
    this.settings.setStartWithSystem(checkedValue(event));
  }

  protected onMinimizeToTray(event: Event): void {
    this.settings.setMinimizeToTray(checkedValue(event));
  }

  protected onCloseToTray(event: Event): void {
    this.settings.setCloseToTray(checkedValue(event));
  }

  protected onShowPinnedFirst(event: Event): void {
    this.settings.setShowPinnedFirst(checkedValue(event));
  }

  protected onCopyConfirmation(event: Event): void {
    this.settings.setCopyConfirmation(checkedValue(event));
  }
}
