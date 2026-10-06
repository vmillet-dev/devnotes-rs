import { ChangeDetectionStrategy, Component, computed, inject, viewChild } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { RateUnit, SizeRow, SizeUnit, SizesAnswer } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { formatDecimal, formatScientific } from '@core/utils/decimal-format.util';
import { ChoiceMenuComponent } from '@shared/controls/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';
import { SIZE_GROUPS, SIZE_UNITS } from './size-units';
import { TransferComponent } from './transfer/transfer.component';

type Converted = Extract<SizesAnswer, { kind: 'converted' }>;

const isZero = (value: string): boolean => /^-?0(\.0*)?$/.test(value);

type Tab = 'conversion' | 'transfer';

const TABS: readonly Segment[] = (['conversion', 'transfer'] as const).map((id) => ({
  id,
  labelKey: `tools.sizes.tabs.${id}`,
}));

/** Every value is Rust's, exact; the page rounds nothing, it only writes the digits in its language. */
@Component({
  selector: 'app-sizes-tool',
  imports: [
    ChoiceMenuComponent,
    ResultRowComponent,
    SegmentedChoiceComponent,
    TransferComponent,
    TranslocoPipe,
  ],
  templateUrl: './sizes-tool.component.html',
  styleUrl: './sizes-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SizesToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  protected readonly tab = toolState<Tab>('sizes.tab', 'conversion');
  protected readonly text = toolState('sizes.text', '');
  protected readonly unit = toolState<SizeUnit>('sizes.unit', 'megabyte');
  protected readonly decimals = toolState('sizes.decimals', 3);

  protected readonly units = SIZE_UNITS;
  protected readonly tabs = TABS;

  /** The other tab's fields, filled by the sample and emptied by Vider from here. */
  private readonly transferSize = toolState('sizes.transferSize', '');
  private readonly transferUnit = toolState<SizeUnit>('sizes.transferUnit', 'gigabyte');
  private readonly rate = toolState('sizes.rate', '');
  private readonly rateUnit = toolState<RateUnit>('sizes.rateUnit', 'megabitPerSecond');
  private readonly efficiency = toolState('sizes.efficiency', 90);

  private readonly transfer = viewChild(TransferComponent);

  /** Digits are written in the language on screen, so the rows follow it. */
  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly answer = liveResult(
    () =>
      this.text().trim() === ''
        ? undefined
        : { text: this.text(), unit: this.unit(), decimals: this.decimals() },
    (request) => this.repository.convertSize(request),
  );

  protected readonly converted = computed<Converted | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'converted' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer === null || answer.kind === 'converted' ? null : answer;
  });

  protected readonly groups = computed(() => {
    const converted = this.converted();
    if (!converted) return [];
    const byUnit = new Map(converted.rows.map((row) => [row.unit, row]));
    return SIZE_GROUPS.map((group) => ({
      id: group.id,
      rows: group.units.map((unit) => this.shown(byUnit.get(unit)!)),
    }));
  });

  /** What was read, in the words and digits on screen: "1,5 Go". */
  protected readonly reading = computed(() => {
    const converted = this.converted();
    const row = converted?.rows.find((candidate) => candidate.unit === converted.unit);
    return converted && row
      ? { value: this.format(row.exact), unit: converted.unit, inText: converted.unitInText }
      : null;
  });

  protected readonly gap = computed(() => {
    const gap = this.converted()?.gap;
    return gap ? { ...gap, ratio: this.format(gap.rounded) } : null;
  });

  readonly result = computed<ToolResult | null>(() =>
    this.tab() === 'transfer' ? (this.transfer()?.result() ?? null) : this.conversionResult(),
  );

  private readonly conversionResult = computed<ToolResult | null>(() => {
    const reading = this.reading();
    const converted = this.converted();
    if (!reading || !converted) return null;
    const symbol = (unit: SizeUnit) => this.transloco.translate<string>(`tools.sizes.units.${unit}`);
    return {
      title: { key: 'tools.sizes.noteTitle', params: { value: reading.value, unit: symbol(reading.unit) } },
      kind: 'snippet',
      language: 'txt',
      content: converted.rows.map((row) => `${symbol(row.unit).padEnd(5)} ${row.exact}`).join('\n'),
    };
  });

  sample(): void {
    this.unit.set('gigabyte');
    this.text.set('4,7');
    this.transferSize.set('4,7');
    this.transferUnit.set('gigabyte');
    this.rate.set('100');
    this.rateUnit.set('megabitPerSecond');
    this.efficiency.set(90);
  }

  clear(): void {
    this.text.set('');
    this.transferSize.set('');
    this.rate.set('');
  }

  protected onTab(id: string): void {
    this.tab.set(id as Tab);
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLInputElement).value);
  }

  protected onUnit(id: string | null): void {
    if (id !== null) this.unit.set(id as SizeUnit);
  }

  protected onDecimals(event: Event): void {
    const decimals = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(decimals)) this.decimals.set(Math.min(Math.max(decimals, 0), 12));
  }

  /** « ≈ » before what rounding changed; a power of ten where it would have left a bare 0. */
  private shown(row: SizeRow): { unit: SizeUnit; value: string; exact: string } {
    const scientific = isZero(row.rounded) ? formatScientific(row.exact, this.lang(), this.decimals()) : null;
    const [text, exact] = scientific
      ? [scientific.text, scientific.exact]
      : [this.format(row.rounded), row.rounded === row.exact];
    return { unit: row.unit, value: `${exact ? '' : '≈ '}${text}`, exact: row.exact };
  }

  private format(value: string): string {
    return formatDecimal(value, this.lang());
  }
}
