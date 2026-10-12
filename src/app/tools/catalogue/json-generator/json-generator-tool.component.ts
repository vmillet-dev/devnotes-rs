import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { JsonSource } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

const SOURCES: readonly Segment[] = (['schema', 'example'] as const).map((id) => ({
  id,
  labelKey: `tools.json-generator.sources.${id}`,
}));

/**
 * One sample a mode: « Exemple » fills what the switch above it expects, never a document
 * under « Schéma » — read as a schema, an example gave nonsense.
 */
const SAMPLES: Record<JsonSource, string> = {
  example: `{
  "id": "0b1c2d3e-4f50-4612-8a7b-9c0d1e2f3a4b",
  "email": "ada@example.com",
  "name": "Ada Lovelace",
  "createdAt": "2026-01-15T09:30:00Z",
  "tags": ["admin", "beta"],
  "score": 42
}`,
  schema: `{
  "type": "object",
  "required": ["id", "email"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "email": { "type": "string", "format": "email" },
    "name": { "type": "string", "minLength": 3, "maxLength": 24 },
    "createdAt": { "type": "string", "format": "date-time" },
    "tags": {
      "type": "array",
      "maxItems": 3,
      "items": { "enum": ["admin", "beta", "staff"] }
    },
    "score": { "type": "integer", "minimum": 0, "maximum": 100 }
  }
}`,
};

/**
 * Without a seed, each draw takes a new one and says which, beside the field: pinned, it makes
 * the draw again. The draw counter is only there to ask twice for the same thing.
 */
@Component({
  selector: 'app-json-generator-tool',
  imports: [CopyValueComponent, IconComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './json-generator-tool.component.html',
  styleUrl: './json-generator-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonGeneratorToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly source = toolState<JsonSource>('json-generator.source', 'example');
  protected readonly text = toolState('json-generator.text', '');
  protected readonly count = toolState('json-generator.count', 3);
  protected readonly seed = toolState<number | null>('json-generator.seed', null);

  private readonly draw = signal(0);

  protected readonly sources = SOURCES;

  protected readonly answer = liveResult(
    () =>
      this.text().trim() === ''
        ? undefined
        : {
            source: this.source(),
            text: this.text(),
            count: this.count(),
            seed: this.seed(),
            draw: this.draw(),
          },
    ({ draw: _draw, ...request }) => this.repository.generateJson(request),
  );

  protected readonly generated = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'generated' ? answer : null;
  });

  protected readonly unreadable = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'unreadable' ? answer : null;
  });

  protected readonly notASchema = computed(() => this.answer.value()?.kind === 'notASchema');

  /** An object that holds nothing of a schema, under « Schéma »: nothing was drawn from it. */
  protected readonly anExample = computed(() => this.answer.value()?.kind === 'anExample');

  /** Never disabled: with a seed pinned it draws the same documents, and the page says so. */
  readonly actions = computed<readonly ToolAction[]>(() => [
    {
      id: 'draw',
      labelKey: 'tools.json-generator.again',
      disabled: false,
      run: () => this.again(),
    },
  ]);

  readonly result = computed<ToolResult | null>(() => {
    const generated = this.generated();
    return generated
      ? {
          title: { key: 'tools.json-generator.noteTitle' },
          kind: 'snippet',
          language: 'json',
          content: generated.text,
        }
      : null;
  });

  sample(): void {
    this.text.set(SAMPLES[this.source()]);
    this.count.set(3);
  }

  clear(): void {
    this.text.set('');
    this.seed.set(null);
  }

  protected again(): void {
    this.draw.update((draw) => draw + 1);
  }

  /** « Le lire comme un modèle »: the same text, under the switch it was written for. */
  protected asExample(): void {
    this.source.set('example');
  }

  /**
   * The seed just used, written down: the same documents come back until it is let go.
   * Pinned already, the pin lets it go.
   */
  protected togglePin(drawn: number | null): void {
    this.seed.set(this.seed() === null ? drawn : null);
  }

  protected onSeed(event: Event): void {
    const value = (event.target as HTMLInputElement).value.trim();
    const seed = Number.parseInt(value, 10);
    this.seed.set(value === '' || !Number.isFinite(seed) ? null : Math.max(0, Math.min(seed, 4_294_967_295)));
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) {
      this.count.set(count);
    }
  }

  protected onSource(id: string): void {
    this.source.set(id as JsonSource);
  }
}
