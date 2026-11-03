import { InjectionToken, Injectable, inject } from '@angular/core';
import { LanguageTag } from '@core/model/language.model';
import { FormatAnswer, FormatRequest, PRETTIER_PARSERS, prettierOptions } from './format.model';
import { PrettierSettingsStore } from './prettier-settings.store';

/** A token, like the clipboard's: jsdom has no `Worker`, and specs answer for Prettier. */
export interface PrettierAdapter {
  run(request: FormatRequest): Promise<FormatAnswer>;
}

/**
 * Created on the first format and kept: loading Prettier is most of a first call's cost. A
 * worker that dies answers every pending request `failed`, and the next format starts another.
 */
export class PrettierWorker implements PrettierAdapter {
  private worker: Worker | null = null;
  private nextId = 0;
  private readonly pending = new Map<number, (answer: FormatAnswer) => void>();

  run(request: FormatRequest): Promise<FormatAnswer> {
    const worker = (this.worker ??= this.start());
    const id = ++this.nextId;
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      worker.postMessage({ id, request });
    });
  }

  private start(): Worker {
    const worker = new Worker(new URL('./prettier.worker', import.meta.url), { type: 'module' });
    worker.addEventListener('message', (event: MessageEvent<{ id: number; answer: FormatAnswer }>) => {
      this.pending.get(event.data.id)?.(event.data.answer);
      this.pending.delete(event.data.id);
    });
    worker.addEventListener('error', () => {
      worker.terminate();
      this.worker = null;
      for (const resolve of this.pending.values()) resolve({ kind: 'failed' });
      this.pending.clear();
    });
    return worker;
  }
}

export const PRETTIER_ADAPTER = new InjectionToken<PrettierAdapter>('PRETTIER_ADAPTER', {
  providedIn: 'root',
  factory: () => new PrettierWorker(),
});

/**
 * ⚠️ An exception to "data processing belongs to Rust", for the reason highlighting is one:
 * it works on the unsaved draft, and Prettier is JavaScript.
 */
@Injectable({ providedIn: 'root' })
export class FormatterService {
  private readonly prettier = inject(PRETTIER_ADAPTER);
  private readonly settings = inject(PrettierSettingsStore);

  canFormat(language: LanguageTag): boolean {
    return PRETTIER_PARSERS[language] !== null;
  }

  /** `editorIndent` is one level as the editor's Tab key writes it. */
  format(text: string, language: LanguageTag, cursor: number, editorIndent: string): Promise<FormatAnswer> {
    const parser = PRETTIER_PARSERS[language];
    if (parser === null) return Promise.resolve({ kind: 'failed' });

    const options = prettierOptions(this.settings.settings(), editorIndent);
    return this.prettier.run({ text, cursor, parser, options });
  }
}
