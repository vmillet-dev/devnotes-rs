import { ClipboardAdapter } from '@core/services/clipboard/clipboard.service';
import { FailsNext, guard } from './fail-next';

/** Under jsdom the real plugin rejects and `ClipboardService` swallows it, silently. */
export class FakeClipboard implements ClipboardAdapter, FailsNext {
  /** What a read hands back, and what the last write left behind. */
  content: string;

  /** When set, the next call rejects like the plugin does outside Tauri. */
  failNext: Error | null = null;

  constructor(content = '') {
    this.content = content;
  }

  readText(): Promise<string | null> {
    return guard(this, () => this.content);
  }

  writeText(value: string): Promise<void> {
    return guard(this, () => {
      this.content = value;
    });
  }
}
