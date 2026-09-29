import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { ColourReading, Notations } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

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
  imports: [OutputRowComponent, TranslocoPipe],
  templateUrl: './colour-tool.component.html',
  styleUrl: './colour-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ColourToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

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
  /** An `f64` crosses as `number | null`: JSON has no NaN. */
  protected readonly ratio = computed(() => (this.contrast()?.ratio ?? 0).toFixed(2));

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

  clear(): void {
    this.colour.set('');
    this.against.set('');
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
