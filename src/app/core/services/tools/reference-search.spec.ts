import { ApplicationRef, Injector, runInInjectionContext, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { HITS_PER_REFERENCE, referenceSearch } from './reference-search';
import { ReferenceRow, ReferenceSource } from './reference.model';
import { ToolDefinition } from './tool.model';

interface Words {
  readonly said: Readonly<Record<string, string>>;
}

interface Fruit extends ReferenceRow {
  readonly said: string;
}

const load = (): Promise<never> => Promise.reject(new Error('a search opens no tool'));

/**
 * The home's search found nothing for « teapot » nor « 418 », though the status reference holds
 * both: it read the tools' names and lines, and nothing inside a reference.
 */
describe('referenceSearch', () => {
  let loads: string[];
  const query = signal('');

  const source = (name: string, keys: readonly string[]): ReferenceSource<Words, Fruit> => ({
    words: {
      fr: () => {
        loads.push(`${name}.fr`);
        return Promise.resolve({ default: { said: { apple: 'pomme', other: 'fruit' } } });
      },
      en: () => {
        loads.push(`${name}.en`);
        return Promise.resolve({ default: { said: { apple: 'apple', other: 'fruit' } } });
      },
    },
    rows: (words, label) =>
      keys.map((key) => {
        const said = words.said[key] ?? words.said['other'] ?? '';
        return { key, group: 'all', said, searchable: [key, said, label('tools.categories.reference')] };
      }),
    title: (row) => row.key.toUpperCase(),
    line: (row) => row.said,
  });

  const reference = (id: string, keys: readonly string[]): ToolDefinition => ({
    id,
    category: 'reference',
    keywords: [],
    load,
    reference: () => {
      loads.push(id);
      return Promise.resolve(source(id, keys));
    },
  });

  const CATALOGUE: readonly ToolDefinition[] = [
    { id: 'case', category: 'text', keywords: [], load },
    reference('fruits', ['apple', '418']),
    reference('many', ['f1', 'f2', 'f3', 'f4', 'f5', 'f6', 'f7']),
    {
      id: 'exact',
      category: 'reference',
      keywords: [],
      load,
      reference: () =>
        Promise.resolve({
          ...source('exact', ['9', '19']),
          matches: (row, typed) => row.key === typed.trim(),
        }),
    },
  ];

  const search = () =>
    runInInjectionContext(TestBed.inject(Injector), () => referenceSearch(CATALOGUE, query.asReadonly()));
  const settle = async () => {
    TestBed.tick();
    await TestBed.inject(ApplicationRef).whenStable();
    TestBed.tick();
  };
  const found = (results: ReturnType<typeof search>) =>
    results.found().map((hits) => ({
      tool: hits.tool.id,
      keys: hits.hits.map((hit) => hit.key),
      more: hits.more,
    }));

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting()] });
    loads = [];
    query.set('');
  });

  it('loads nothing until something is typed, then every reference once', async () => {
    const results = search();
    await settle();
    expect(loads).toEqual([]);
    expect(results.loading()).toBe(false);

    query.set('pomme');
    TestBed.tick();
    expect(results.loading()).toBe(true);
    await settle();
    expect(results.loading()).toBe(false);
    expect(loads.sort()).toEqual(['exact.fr', 'fruits', 'fruits.fr', 'many', 'many.fr']);

    // Typing on, emptying the field and typing again load nothing more.
    for (const typed of ['pomm', '', 'fruit']) {
      query.set(typed);
      await settle();
    }
    expect(loads).toHaveLength(5);
  });

  it('finds a row by its key and by its words, and says what it is called and means', async () => {
    const results = search();
    query.set('418');
    await settle();
    expect(found(results)).toEqual([{ tool: 'fruits', keys: ['418'], more: 0 }]);

    // Case and accents aside, as everywhere.
    query.set('PÔMME');
    await settle();
    expect(results.found()[0]!.hits).toEqual([{ key: 'apple', title: 'APPLE', line: 'pomme' }]);
  });

  it('lists a few rows of a reference, and counts the others', async () => {
    const results = search();
    query.set('fruit');
    await settle();

    const many = found(results).find((hits) => hits.tool === 'many')!;
    expect(many.keys).toEqual(['f1', 'f2', 'f3', 'f4']);
    expect(many.keys).toHaveLength(HITS_PER_REFERENCE);
    expect(many.more).toBe(3);
    // In the catalogue's order, a reference at a time.
    expect(found(results).map((hits) => hits.tool)).toEqual(['fruits', 'many']);
  });

  /** A reference that reads a query its own way reads the home's the same way. */
  it('finds what the reference’s own matcher finds', async () => {
    const results = search();
    query.set('19');
    await settle();

    expect(found(results)).toEqual([{ tool: 'exact', keys: ['19'], more: 0 }]);
  });

  /** One character is nearly every row of every reference. */
  it('reads the references from the second character on', async () => {
    const results = search();
    query.set('f');
    await settle();
    expect(results.found()).toEqual([]);

    query.set('f1');
    await settle();
    expect(found(results)).toEqual([{ tool: 'many', keys: ['f1'], more: 0 }]);
  });

  it('follows the language on screen, and keeps the rows it had until the next land', async () => {
    const results = search();
    query.set('pomme');
    await settle();
    expect(found(results)).toHaveLength(1);

    const transloco = TestBed.inject(TranslocoService);
    transloco.setActiveLang('en');
    TestBed.tick();
    expect(found(results)).toHaveLength(1);
    await settle();
    expect(found(results)).toEqual([]);

    query.set('apple');
    await settle();
    expect(results.found()[0]!.hits[0]).toEqual({ key: 'apple', title: 'APPLE', line: 'apple' });
  });

  it('answers nothing, and stops waiting, when a reference will not load', async () => {
    const broken: readonly ToolDefinition[] = [
      {
        id: 'gone',
        category: 'reference',
        keywords: [],
        load,
        reference: () => Promise.reject(new Error('chunk')),
      },
    ];
    const results = runInInjectionContext(TestBed.inject(Injector), () =>
      referenceSearch(broken, query.asReadonly()),
    );
    query.set('anything');
    await settle();

    expect(results.found()).toEqual([]);
    expect(results.loading()).toBe(false);
  });
});
