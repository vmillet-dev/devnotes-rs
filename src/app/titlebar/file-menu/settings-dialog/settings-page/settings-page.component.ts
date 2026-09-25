import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  DENSITIES,
  Density,
  INDENT_CHOICES,
  IndentChoice,
  LOCALE_CHOICES,
  LocaleChoice,
  THEME_CHOICES,
  ThemeChoice,
} from '@core/services/settings/app-settings.model';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

/** The switches, which differ only by the key they write. */
type BooleanSetting =
  'startWithSystem' | 'minimizeToTray' | 'closeToTray' | 'showPinnedFirst' | 'copyConfirmation';

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
  protected readonly draft = inject(SettingsDraftStore);

  protected readonly localeChoices: readonly ChoiceOption[] = LOCALE_CHOICES.map((choice) => ({
    id: choice,
    name: `locale.${choice}`,
    nameIsKey: true,
  }));

  protected readonly themeSegments: readonly Segment[] = THEME_CHOICES.map((choice) => ({
    id: choice,
    labelKey: `settings.theme.${choice}`,
  }));

  protected readonly densitySegments: readonly Segment[] = DENSITIES.map((choice) => ({
    id: choice,
    labelKey: `settings.density.${choice}`,
  }));

  protected readonly indentChoices: readonly ChoiceOption[] = INDENT_CHOICES.map((choice) => ({
    id: choice,
    name: `settings.indent.${choice}`,
    nameIsKey: true,
  }));

  protected onUpdateNotifications(event: Event): void {
    const wanted = checkedValue(event);
    this.draft.set('updateNotifications', wanted);
    // Asking to be told about updates is exactly what taking a skip back means.
    if (wanted) this.draft.set('skippedUpdate', '');
  }

  protected forgetSkippedUpdate(): void {
    this.draft.set('skippedUpdate', '');
  }

  protected onLocale(locale: string | null): void {
    if (locale) this.draft.set('locale', locale as LocaleChoice);
  }

  protected onTheme(theme: string): void {
    this.draft.set('theme', theme as ThemeChoice);
  }

  protected onDensity(density: string): void {
    this.draft.set('density', density as Density);
  }

  protected onIndent(choice: string | null): void {
    if (choice) this.draft.set('codeIndent', choice as IndentChoice);
  }

  protected onToggle(key: BooleanSetting, event: Event): void {
    this.draft.set(key, checkedValue(event));
  }
}
