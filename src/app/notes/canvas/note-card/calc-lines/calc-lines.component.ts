import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoService } from '@jsverse/transloco';
import type { CalcCardLine } from '@core/model/calc.model';
import { CalcSheets } from '@core/services/calc/calc-sheets.service';

/**
 * A Calcul note on its card: the last lines that gave something, often the conclusion of the
 * sheet, each with its result — read by Rust from the whole sheet, written here in the
 * language on screen.
 */
@Component({
  selector: 'app-calc-lines',
  templateUrl: './calc-lines.component.html',
  styleUrl: './calc-lines.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CalcLinesComponent {
  readonly lines = input.required<readonly CalcCardLine[]>();

  private readonly sheets = inject(CalcSheets);
  private readonly transloco = inject(TranslocoService);
  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly shown = computed(() => {
    this.lang();
    const formatter = this.sheets.formatter();
    return this.lines().map((line) => {
      const result = formatter.value(line.value);
      return {
        expression: line.expression,
        result: `${result.approx ? '≈ ' : ''}${result.text}`,
        total: line.aggregate,
      };
    });
  });
}
