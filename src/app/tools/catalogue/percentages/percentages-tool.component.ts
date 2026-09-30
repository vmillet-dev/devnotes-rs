import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { PercentPair, PercentResult, PercentagesAnswer } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { formatDecimal } from '@core/utils/decimal-format.util';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Question = keyof PercentagesAnswer;
type Answered = Extract<PercentResult, { kind: 'answered' }>;

/** How each line reads: which number comes first, and what the result is written with. */
interface Layout {
  readonly id: Question;
  readonly first: 'x' | 'y';
  /** A word before the first field: « de 80 à 100 ». */
  readonly opening: boolean;
  /** `%` after the second field: « de 15 % ». */
  readonly percentAfter: boolean;
  /** `%` after the result, and a sign when it is a change. */
  readonly percentResult: boolean;
  readonly signed: boolean;
}

interface Formula {
  readonly key: string;
  readonly params: Record<string, string>;
}

interface Shown {
  readonly line: Layout;
  readonly result: PercentResult;
  readonly value: string | null;
  readonly formula: Formula | null;
}

const LINES: readonly Layout[] = [
  { id: 'of', first: 'x', opening: false, percentAfter: false, percentResult: false, signed: false },
  { id: 'share', first: 'x', opening: false, percentAfter: false, percentResult: true, signed: false },
  { id: 'change', first: 'x', opening: true, percentAfter: false, percentResult: true, signed: true },
  { id: 'apply', first: 'y', opening: false, percentAfter: true, percentResult: false, signed: false },
  { id: 'before', first: 'y', opening: false, percentAfter: true, percentResult: false, signed: false },
];

const EMPTY: Record<Question, PercentPair> = {
  of: { x: '', y: '' },
  share: { x: '', y: '' },
  change: { x: '', y: '' },
  apply: { x: '', y: '' },
  before: { x: '', y: '' },
};

const DIRECTIONS: readonly Segment[] = [
  { id: 'raise', labelKey: 'tools.percentages.raise' },
  { id: 'lower', labelKey: 'tools.percentages.lower' },
];

/** Every figure is Rust's, exact; the page writes its digits in the language on screen. */
@Component({
  selector: 'app-percentages-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './percentages-tool.component.html',
  styleUrl: './percentages-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PercentagesToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  protected readonly pairs = toolState<Record<Question, PercentPair>>('percentages.pairs', EMPTY);
  protected readonly lower = toolState('percentages.lower', false);
  protected readonly decimals = toolState('percentages.decimals', 2);

  protected readonly lines = LINES;
  protected readonly directions = DIRECTIONS;

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly answer = liveResult(
    () => {
      const pairs = this.pairs();
      const typed = Object.values(pairs).some((pair) => pair.x.trim() !== '' || pair.y.trim() !== '');
      return typed ? { ...pairs, lower: this.lower(), decimals: this.decimals() } : undefined;
    },
    (request) => this.repository.answerPercentages(request),
  );

  /** Each line's answer, written for the page: the result, its formula, or why there is none. */
  protected readonly shown = computed<readonly Shown[]>(() => {
    const answer = this.answer.value();
    const lang = this.lang();
    const lower = this.answer.answered()?.lower ?? this.lower();
    return LINES.map((line) => {
      const result = answer?.[line.id] ?? { kind: 'empty' };
      if (result.kind !== 'answered') return { line, result, value: null, formula: null };
      return { line, result, ...this.written(line, result, lang, lower) };
    });
  });

  readonly result = computed<ToolResult | null>(() => {
    this.lang();
    const formulas = this.shown()
      .map((shown) => shown.formula)
      .filter((formula): formula is Formula => formula !== null)
      .map((formula) => this.transloco.translate<string>(formula.key, formula.params));
    return formulas.length > 0
      ? {
          title: { key: 'tools.percentages.noteTitle' },
          kind: 'snippet',
          language: 'txt',
          content: formulas.join('\n'),
        }
      : null;
  });

  clear(): void {
    this.pairs.set(EMPTY);
  }

  protected field(question: Question, operand: 'x' | 'y'): string {
    return this.pairs()[question][operand];
  }

  protected onField(question: Question, operand: 'x' | 'y', event: Event): void {
    const text = (event.target as HTMLInputElement).value;
    this.pairs.update((pairs) => ({ ...pairs, [question]: { ...pairs[question], [operand]: text } }));
  }

  protected onDirection(id: string): void {
    this.lower.set(id === 'lower');
  }

  protected onDecimals(event: Event): void {
    const decimals = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(decimals)) this.decimals.set(Math.min(Math.max(decimals, 0), 12));
  }

  private written(
    line: Layout,
    result: Answered,
    lang: string,
    lower: boolean,
  ): Pick<Shown, 'value' | 'formula'> {
    const format = (value: string) => formatDecimal(value, lang);
    const approximate = result.rounded !== result.exact;
    const sign = line.signed && !result.rounded.startsWith('-') && result.rounded !== '0' ? '+' : '';
    const figure = `${sign}${format(result.rounded)}`;
    const key =
      line.id === 'apply'
        ? `tools.percentages.apply.${lower ? 'lowerFormula' : 'raiseFormula'}`
        : `tools.percentages.${line.id}.formula`;
    return {
      value: `${approximate ? '≈ ' : ''}${figure}${line.percentResult ? ' %' : ''}`,
      formula: { key, params: { x: format(result.x), y: format(result.y), result: figure } },
    };
  }
}
