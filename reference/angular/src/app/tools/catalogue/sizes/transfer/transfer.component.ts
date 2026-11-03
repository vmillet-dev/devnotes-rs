import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import {
  ConnectionEstimate,
  RateUnit,
  SizeUnit,
  TransferAnswer,
  TransferTime,
} from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';
import { SIZE_UNITS } from '../size-units';

type Estimated = Extract<TransferAnswer, { kind: 'estimated' }>;

const RATE_UNITS: readonly RateUnit[] = [
  'bitPerSecond',
  'kilobitPerSecond',
  'megabitPerSecond',
  'gigabitPerSecond',
  'bytePerSecond',
  'kilobytePerSecond',
  'megabytePerSecond',
  'gigabytePerSecond',
];

const RATE_OPTIONS: readonly ChoiceOption[] = RATE_UNITS.map((unit) => ({
  id: unit,
  name: `tools.sizes.rateUnits.${unit}`,
  nameIsKey: true,
}));

const TIME_UNITS = ['days', 'hours', 'minutes', 'seconds'] as const;

export const EFFICIENCY = { min: 50, max: 100 } as const;

/** How long a quantity takes over a link, Rust's; the page spells the durations it answers. */
@Component({
  selector: 'app-sizes-transfer',
  imports: [ChoiceMenuComponent, ResultRowComponent, TranslocoPipe],
  templateUrl: './transfer.component.html',
  styleUrl: './transfer.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TransferComponent {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly size = toolState('sizes.transferSize', '');
  protected readonly sizeUnit = toolState<SizeUnit>('sizes.transferUnit', 'gigabyte');
  protected readonly rate = toolState('sizes.rate', '');
  protected readonly rateUnit = toolState<RateUnit>('sizes.rateUnit', 'megabitPerSecond');
  protected readonly efficiency = toolState('sizes.efficiency', 90);

  protected readonly sizeUnits = SIZE_UNITS;
  protected readonly rateUnits = RATE_OPTIONS;
  protected readonly bounds = EFFICIENCY;

  protected readonly answer = liveResult(
    () =>
      this.size().trim() === '' || this.rate().trim() === ''
        ? undefined
        : {
            size: this.size(),
            sizeUnit: this.sizeUnit(),
            rate: this.rate(),
            rateUnit: this.rateUnit(),
            efficiency: this.efficiency(),
          },
    (request) => this.repository.estimateTransfer(request),
  );

  protected readonly estimated = computed<Estimated | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'estimated' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer === null || answer.kind === 'estimated' ? null : answer;
  });

  /** « 100 Mbit/s »: the rate the estimate was made at, as it was typed. */
  protected readonly rateLabel = computed(() => {
    const asked = this.answer.answered();
    return asked ? this.rateOf(asked.rate, asked.rateUnit) : '';
  });

  protected readonly connections = computed(() =>
    (this.estimated()?.connections ?? []).map((estimate) => ({
      id: estimate.connection ?? 'typed',
      name: this.connectionName(estimate),
      value: this.time(estimate.span),
    })),
  );

  readonly result = computed<ToolResult | null>(() => {
    const estimated = this.estimated();
    const asked = this.answer.answered();
    if (!estimated || !asked) return null;
    const rows = this.connections();
    const width = Math.max(...rows.map((row) => row.name.length));
    return {
      title: {
        key: 'tools.sizes.noteTransfer',
        params: { size: asked.size.trim(), rate: this.rateLabel(), efficiency: estimated.efficiency },
      },
      kind: 'snippet',
      language: 'txt',
      content: rows.map((row) => `${row.name.padEnd(width)}  ${row.value}`).join('\n'),
    };
  });

  /** « 6 min 58 s », « 3 j 4 h », « < 1 s ». */
  protected time(span: TransferTime): string {
    if (span.underASecond) return this.say('tools.sizes.time.underASecond');
    const parts = TIME_UNITS.filter((unit) => span[unit] > 0).map((unit) =>
      this.say(`tools.sizes.time.${unit}`, {
        value: new Intl.NumberFormat(this.lang()).format(span[unit]),
      }),
    );
    return parts.length === 0 ? this.say('tools.sizes.time.seconds', { value: 0 }) : parts.join(' ');
  }

  protected onSize(event: Event): void {
    this.size.set((event.target as HTMLInputElement).value);
  }

  protected onSizeUnit(id: string | null): void {
    if (id !== null) this.sizeUnit.set(id as SizeUnit);
  }

  protected onRate(event: Event): void {
    this.rate.set((event.target as HTMLInputElement).value);
  }

  protected onRateUnit(id: string | null): void {
    if (id !== null) this.rateUnit.set(id as RateUnit);
  }

  /** The slider and the field move one value, held within its bounds. */
  protected onEfficiency(event: Event): void {
    const value = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(value))
      this.efficiency.set(Math.min(EFFICIENCY.max, Math.max(EFFICIENCY.min, value)));
  }

  private connectionName(estimate: ConnectionEstimate): string {
    return estimate.connection === null
      ? this.rateLabel()
      : this.say(`tools.sizes.connections.${estimate.connection}`);
  }

  private rateOf(rate: string, unit: RateUnit): string {
    return `${rate.trim()} ${this.say(`tools.sizes.rateUnits.${unit}`)}`;
  }

  private say(key: string, params: Record<string, unknown> = {}): string {
    this.lang();
    return this.transloco.translate(key, params);
  }
}
