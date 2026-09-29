import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FormatAnswer, FormatRequest } from './format.model';
import { FormatterService, PRETTIER_ADAPTER, PrettierAdapter, PrettierWorker } from './formatter.service';

/** jsdom has no `Worker`: this one answers what the test tells it to. */
class FakeWorker extends EventTarget {
  static created: FakeWorker[] = [];
  readonly posted: { id: number; request: FormatRequest }[] = [];
  terminated = false;

  constructor() {
    super();
    FakeWorker.created.push(this);
  }

  postMessage(message: { id: number; request: FormatRequest }): void {
    this.posted.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }

  answer(id: number, answer: FormatAnswer): void {
    this.dispatchEvent(new MessageEvent('message', { data: { id, answer } }));
  }

  crash(): void {
    this.dispatchEvent(new Event('error'));
  }
}

const REQUEST: FormatRequest = {
  text: 'a',
  cursor: 0,
  parser: 'babel',
  options: {
    printWidth: 100,
    tabWidth: 2,
    useTabs: false,
    singleQuote: true,
    semi: true,
    trailingComma: 'all',
  },
};

describe('PrettierWorker', () => {
  beforeEach(() => {
    FakeWorker.created = [];
    vi.stubGlobal('Worker', FakeWorker);
  });

  afterEach(() => vi.unstubAllGlobals());

  it('starts nothing until the first format', () => {
    new PrettierWorker();

    expect(FakeWorker.created).toHaveLength(0);
  });

  it('matches each answer to its request, on one worker', async () => {
    const prettier = new PrettierWorker();
    const first = prettier.run(REQUEST);
    const second = prettier.run({ ...REQUEST, text: 'b' });
    const [worker] = FakeWorker.created;

    worker!.answer(2, { kind: 'unchanged' });
    worker!.answer(1, { kind: 'failed' });

    expect(await first).toEqual({ kind: 'failed' });
    expect(await second).toEqual({ kind: 'unchanged' });
    expect(FakeWorker.created).toHaveLength(1);
    expect(worker!.posted.map((message) => message.request.text)).toEqual(['a', 'b']);
  });

  it('fails what was pending when the worker dies, and starts another next time', async () => {
    const prettier = new PrettierWorker();
    const pending = prettier.run(REQUEST);
    FakeWorker.created[0]!.crash();

    expect(await pending).toEqual({ kind: 'failed' });
    expect(FakeWorker.created[0]!.terminated).toBe(true);

    void prettier.run(REQUEST);
    expect(FakeWorker.created).toHaveLength(2);
  });
});

describe('FormatterService', () => {
  const run = vi.fn<PrettierAdapter['run']>(() => Promise.resolve({ kind: 'unchanged' } as const));

  beforeEach(() => {
    run.mockClear();
    TestBed.configureTestingModule({ providers: [{ provide: PRETTIER_ADAPTER, useValue: { run } }] });
  });

  it('formats the languages Prettier knows, and only those', () => {
    const formatter = TestBed.inject(FormatterService);

    expect(formatter.canFormat('ts')).toBe(true);
    expect(formatter.canFormat('scss')).toBe(true);
    expect(formatter.canFormat('sql')).toBe(false);
  });

  it('sends the language parser and the editor indentation', async () => {
    await TestBed.inject(FormatterService).format('x', 'ts', 1, '\t');

    expect(run).toHaveBeenCalledWith(
      expect.objectContaining({
        text: 'x',
        cursor: 1,
        parser: 'typescript',
        options: expect.objectContaining({ useTabs: true, tabWidth: 4 }),
      }),
    );
  });

  it('asks nothing of Prettier for a language it does not format', async () => {
    expect(await TestBed.inject(FormatterService).format('x', 'sql', 0, '  ')).toEqual({ kind: 'failed' });
    expect(run).not.toHaveBeenCalled();
  });
});
