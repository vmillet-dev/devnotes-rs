import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import {
  CALC_DATE_FORMATS,
  CALC_DECIMALS,
  CALC_NUMBER_FORMATS,
  CalcDateFormat,
  CalcDecimals,
  CalcNumberFormat,
} from '@core/model/calc.model';
import type { NoteKind } from '@core/model/checklist.model';
import {
  DENSITIES,
  Density,
  INDENT_CHOICES,
  IndentChoice,
  LOCALE_CHOICES,
  LocaleChoice,
  NOTE_KIND_CHOICES,
  THEME_CHOICES,
  ThemeChoice,
} from '@core/services/settings/app-settings.model';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

/** The switches, which differ only by the key they write. */
type BooleanSetting =
  'codeWrap' | 'startWithSystem' | 'minimizeToTray' | 'closeToTray' | 'showPinnedFirst' | 'copyConfirmation';

function checkedValue(event: Event): boolean {
  return (event.target as HTMLInputElement).checked;
}

/** What each locale makes of one figure, beside its name: the choice is seen, not guessed. */
const SAMPLE = 1578.13;

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
  private readonly transloco = inject(TranslocoService);
  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

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

  protected readonly noteKindSegments: readonly Segment[] = NOTE_KIND_CHOICES.map((choice) => ({
    id: choice,
    labelKey: `settings.noteKind.${choice}`,
  }));

  protected readonly indentChoices: readonly ChoiceOption[] = INDENT_CHOICES.map((choice) => ({
    id: choice,
    name: `settings.indent.${choice}`,
    nameIsKey: true,
  }));

  protected readonly calcDecimalChoices: readonly ChoiceOption[] = CALC_DECIMALS.map((choice) =>
    choice === 'auto'
      ? { id: choice, name: 'settings.calc.decimalsAuto', nameIsKey: true }
      : { id: choice, name: choice },
  );

  protected readonly calcNumberFormats = computed<readonly ChoiceOption[]>(() => {
    const lang = this.lang();
    return CALC_NUMBER_FORMATS.map((choice) => {
      const locale = choice === 'language' ? lang : choice;
      const hint = new Intl.NumberFormat(locale, { minimumFractionDigits: 2 }).format(SAMPLE);
      return choice === 'language'
        ? { id: choice, name: 'settings.calc.language', nameIsKey: true, hint }
        : { id: choice, name: `settings.calc.locales.${choice}`, nameIsKey: true, hint };
    });
  });

  protected readonly calcDateSegments: readonly Segment[] = CALC_DATE_FORMATS.map((choice) => ({
    id: choice,
    labelKey: `settings.calc.dates.${choice}`,
  }));

  protected onCalcDecimals(choice: string | null): void {
    if (choice) this.draft.set('calcDecimals', choice as CalcDecimals);
  }

  protected onCalcNumberFormat(choice: string | null): void {
    if (choice) this.draft.set('calcNumberFormat', choice as CalcNumberFormat);
  }

  protected onCalcDateFormat(choice: string): void {
    this.draft.set('calcDateFormat', choice as CalcDateFormat);
  }

  protected onUpdateNotifications(event: Event): void {
    const wanted = checkedValue(event);
    this.draft.set('updateNotifications', wanted);
    // Asking to be told about updates is exactly what taking a skip back means.
    if (wanted) this.draft.set('skippedUpdate', '');
  }

  protected forgetSkippedUpdate(): void {
    this.draft.set('skippedUpdate', '');
  }

  protected onNoteKind(kind: string): void {
    this.draft.set('defaultNoteKind', kind as NoteKind);
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
