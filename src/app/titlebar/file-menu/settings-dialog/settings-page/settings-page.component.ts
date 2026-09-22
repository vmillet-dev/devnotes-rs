import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoService, TranslocoPipe } from '@jsverse/transloco';
import { ChangePassphraseDialogComponent } from '../change-passphrase-dialog/change-passphrase-dialog.component';
import {
  DENSITIES,
  Density,
  LOCALE_CHOICES,
  LocaleChoice,
  THEME_CHOICES,
  ThemeChoice,
} from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { DEFAULT_SHORTCUTS, acceleratorFromEvent } from '@core/services/shortcuts/shortcut.model';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

function checkedValue(event: Event): boolean {
  return (event.target as HTMLInputElement).checked;
}

@Component({
  selector: 'app-settings-page',
  imports: [TranslocoPipe, ChangePassphraseDialogComponent, ChoiceMenuComponent, SegmentedChoiceComponent],
  templateUrl: './settings-page.component.html',
  styleUrl: './settings-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsPageComponent {
  protected readonly settings = inject(SettingsStore);

  protected readonly locales = LOCALE_CHOICES;
  protected readonly themes = THEME_CHOICES;
  protected readonly densities = DENSITIES;
  protected readonly defaultShortcut = DEFAULT_SHORTCUTS.palette;

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

  protected readonly isChangingPassphrase = signal(false);

  protected openPassphraseChange(): void {
    this.isChangingPassphrase.set(true);
  }

  protected closePassphraseChange(): void {
    this.isChangingPassphrase.set(false);
  }

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

  protected onAutomaticBackups(event: Event): void {
    this.settings.setAutomaticBackups(checkedValue(event));
  }

  protected onCopyConfirmation(event: Event): void {
    this.settings.setCopyConfirmation(checkedValue(event));
  }

  /**
   * The field listens for a keystroke rather than accepting text, which would let through
   * combinations the native side cannot read back. A keystroke with no modifier goes back
   * to the dialog, which is what leaves Tab and Escape working inside the field.
   */
  protected onShortcutKeydown(event: KeyboardEvent): void {
    const accelerator = acceleratorFromEvent(event);
    if (!accelerator) return;

    event.preventDefault();
    this.settings.setPaletteShortcut(accelerator);
  }

  protected resetShortcut(): void {
    this.settings.setPaletteShortcut(this.defaultShortcut);
  }
}
