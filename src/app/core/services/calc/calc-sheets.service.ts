import { Injectable, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { CalcRepository } from '@core/data/calc.repository';
import { CalcDecimals, CalcSheet } from '@core/model/calc.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ClockService } from '@core/services/time/clock.service';
import { CalcFormatter } from './calc-format';

/** A Calcul note's sheet computed by Rust, and written in the language on screen. */
@Injectable({ providedIn: 'root' })
export class CalcSheets {
  private readonly repository = inject(CalcRepository);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);
  private readonly settings = inject(SettingsStore);
  private readonly notifier = inject(ErrorNotifier);

  /** The local day, as the notes' sections reckon it: `aujourd'hui` is the window's. */
  today(): string {
    const now = this.clock.now();
    const pad = (value: number): string => String(value).padStart(2, '0');
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  }

  evaluate(sheet: string): Promise<CalcSheet> {
    return this.repository.evaluate(sheet, this.today());
  }

  formatter(decimals: CalcDecimals = this.settings.calcDecimals()): CalcFormatter {
    return new CalcFormatter(
      this.transloco.getActiveLang(),
      (key, params) => this.transloco.translate(key, params),
      decimals,
    );
  }

  /** What copying a sheet gives: its text, each result after its line as a comment. */
  async copyText(sheet: string): Promise<string | null> {
    const answer = await this.notifier.attempt('errors.calcFailed', () => this.evaluate(sheet));
    return answer && this.formatter().copyText(sheet, answer);
  }
}
