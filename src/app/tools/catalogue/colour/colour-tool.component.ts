import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { ColourReading, Notations } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type Read = Extract<ColourReading, { kind: 'read' }>;

const ROWS: readonly { readonly id: keyof Notations; readonly name: string }[] = [
  { id: 'hex', name: 'HEX' },
  { id: 'rgb', name: 'RGB' },
  { id: 'hsl', name: 'HSL' },
  { id: 'oklch', name: 'OKLCH' },
];

function read(reading: ColourReading | undefined): Read | null {
  return reading?.kind === 'read' ? reading : null;
}

/** Everything is computed in Rust; the swatches are all the page paints. */
@Component({
  selector: 'app-colour-tool',
  imports: [ResultRowComponent, TranslocoPipe],
  templateUrl: './colour-tool.component.html',
  styleUrl: './colour-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ColourToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly colour = toolState('colour.colour', '#1e90ff');
  protected readonly against = toolState('colour.against', '#ffffff');

  protected readonly rows = ROWS;

  protected readonly answer = liveResult(
    () => ({ colour: this.colour(), against: this.against() }),
    (request) => this.repository.describeColour(request),
  );

  protected readonly first = computed(() => read(this.answer.value()?.colour));
  protected readonly second = computed(() => read(this.answer.value()?.against));
  protected readonly contrast = computed(() => this.answer.value()?.contrast ?? null);
  /** An `f64` crosses as `number | null`: JSON has no NaN. « 3,24 » in French. */
  protected readonly ratio = computed(() =>
    new Intl.NumberFormat(this.lang(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(
      this.contrast()?.ratio ?? 0,
    ),
  );

  /** The one line a reader wants: whether body text can sit on this background. */
  protected readonly verdict = computed(() => {
    const contrast = this.contrast();
    if (!contrast) return 'failsAll';
    if (!contrast.aa.large) return 'failsAll';
    if (!contrast.aa.normal) return 'failsNormal';
    return contrast.aaa.normal ? 'aaa' : 'aa';
  });

  protected readonly fixLevel = computed(() => this.contrast()?.fix?.level ?? 'aa');

  protected readonly invalid = computed(() => this.answer.value()?.colour.kind === 'invalid');
  protected readonly againstInvalid = computed(() => this.answer.value()?.against.kind === 'invalid');

  readonly result = computed<ToolResult | null>(() => {
    const first = this.first();
    return first
      ? {
          title: { key: 'tools.colour.noteTitle', params: { hex: first.notations.hex } },
          kind: 'snippet',
          language: 'css',
          content: ROWS.map(({ id }) => first.notations[id]).join('\n'),
        }
      : null;
  });

  sample(): void {
    this.colour.set('#3b82f6');
    this.against.set('#ffffff');
  }

  clear(): void {
    this.colour.set('');
    this.against.set('');
  }

  /** The text and the background trade places. */
  protected swap(): void {
    const [colour, against] = [this.colour(), this.against()];
    this.colour.set(against);
    this.against.set(colour);
  }

  /** The nearest text colour Rust found that reaches the next level, in the « Texte » field. */
  protected applyFix(): void {
    const fix = this.contrast()?.fix;
    if (fix?.kind === 'found') this.colour.set(fix.hex);
  }

  protected onText(which: 'colour' | 'against', event: Event): void {
    const text = (event.target as HTMLInputElement).value;
    (which === 'colour' ? this.colour : this.against).set(text);
  }

  /** The system's picker, for the colour to be seen before it is written. */
  protected onPicked(which: 'colour' | 'against', event: Event): void {
    this.onText(which, event);
  }

  /** What `<input type="color">` takes: six hexadecimal digits and nothing else. */
  protected pickerValue(reading: Read | null): string {
    return reading ? reading.swatch.slice(0, 7) : '#000000';
  }
}
