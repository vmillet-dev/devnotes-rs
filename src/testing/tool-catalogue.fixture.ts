import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';
import { Tool, ToolAction, ToolDefinition, ToolResult } from '@core/services/tools/tool.model';

/** Stands for any tool: the frame, the home and the rail know no more of one than this. */
@Component({
  selector: 'app-fake-tool',
  template: `<p data-testid="fake-tool">{{ text() }}</p>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FakeToolComponent implements Tool {
  readonly text = signal('typed');
  readonly swapped = signal(false);

  readonly result = computed<ToolResult | null>(() =>
    this.text()
      ? { title: { key: 'tools.title' }, kind: 'snippet', language: 'txt', content: this.text() }
      : null,
  );

  readonly actions = computed<readonly ToolAction[]>(() => [
    { id: 'swap', labelKey: 'tools.clear', disabled: this.text() === '', run: () => this.swapped.set(true) },
  ]);

  clear(): void {
    this.text.set('');
  }
}

const load = async () => FakeToolComponent;

export const FAKE_TOOLS: readonly ToolDefinition[] = [
  { id: 'case', category: 'text', keywords: ['camelCase'], load },
  { id: 'slug', category: 'text', keywords: [], load },
  { id: 'hash', category: 'crypto', keywords: ['sha256', 'hmac'], load },
];
