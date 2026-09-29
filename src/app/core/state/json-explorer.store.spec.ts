import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { JsonRepository } from '@core/data/json.repository';
import { FakeJsonRepository } from '@testing/fake-json-repository';
import { JSON_TEXT, jsonView } from '@testing/json-view.fixture';
import { JsonExplorerStore } from './json-explorer.store';

describe('JsonExplorerStore', () => {
  let repository: FakeJsonRepository;
  let store: JsonExplorerStore;

  beforeEach(() => {
    repository = new FakeJsonRepository();
    repository.view = jsonView({ matches: ['$.id', '$.data.lines[0].a'], matchCount: 2 });
    TestBed.configureTestingModule({
      providers: [JsonExplorerStore, { provide: JsonRepository, useValue: repository }],
    });
    store = TestBed.inject(JsonExplorerStore);
  });

  afterEach(() => vi.useRealTimers());

  async function settle(): Promise<void> {
    await TestBed.inject(ApplicationRef).whenStable();
  }

  function lastOpening() {
    return repository.queries.at(-1)?.opening;
  }

  it('explores nothing until it is given a text', async () => {
    await settle();

    expect(repository.queries).toEqual([]);
    expect(store.view()).toBeNull();
  });

  it('explores a text it loads at once, from its first level', async () => {
    store.load(JSON_TEXT);
    await settle();

    expect(repository.queries).toEqual([{ text: JSON_TEXT, search: '', opening: { kind: 'initial' } }]);
    expect(store.view()?.graph.nodes).toHaveLength(4);
    expect(store.text()).toBe(JSON_TEXT);
  });

  it('waits for the typing to pause before exploring the draft again', async () => {
    store.load(JSON_TEXT);
    await settle();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });

    store.setText('{"a":');
    store.setText('{"a":1}');
    vi.advanceTimersByTime(250);
    vi.useRealTimers();
    await settle();

    expect(repository.queries.map((query) => query.text)).toEqual([JSON_TEXT, '{"a":1}']);
  });

  it('loads at once a text given after it was cleared', async () => {
    store.load(JSON_TEXT);
    store.clear();
    store.setText('[]');
    await settle();

    expect(repository.queries.at(-1)?.text).toBe('[]');
  });

  it('opens a closed container and selects it, and closes an open one', async () => {
    store.load(JSON_TEXT);
    await settle();

    store.toggle({ path: '$.more', opens: true, open: false });
    await settle();
    expect(lastOpening()).toEqual({
      kind: 'paths',
      paths: ['$', '$.data', '$.data.lines', '$.data.lines[0]', '$.more'],
    });
    expect(store.selectedPath()).toBe('$.more');

    store.toggle({ path: '$.data.lines', opens: true, open: true });
    await settle();
    expect(lastOpening()).toEqual({ kind: 'paths', paths: ['$', '$.data', '$.data.lines[0]'] });
  });

  it('selects a value without opening anything', async () => {
    store.load(JSON_TEXT);
    await settle();

    store.toggle({ path: '$.id', opens: false, open: false });
    await settle();

    expect(store.selectedPath()).toBe('$.id');
    expect(repository.queries).toHaveLength(1);
  });

  it('opens everything on asking', async () => {
    store.load(JSON_TEXT);
    store.openAll();
    await settle();

    expect(lastOpening()).toEqual({ kind: 'all' });
  });

  it('searches once the typing pauses, and steps through the matches around', async () => {
    store.load(JSON_TEXT);
    await settle();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    store.setSearch(' evt ');
    expect(store.search()).toBe(' evt ');
    vi.advanceTimersByTime(150);
    vi.useRealTimers();
    await settle();
    expect(repository.queries.at(-1)?.search).toBe('evt');

    store.step(1);
    expect(store.selectedPath()).toBe('$.id');
    expect(store.matchIndex()).toBe(0);
    await settle();
    expect(lastOpening()).toMatchObject({ kind: 'paths', reveal: '$.id' });

    store.step(1);
    store.step(1);
    expect(store.matchIndex()).toBe(0);
    store.step(-1);
    expect(store.selectedPath()).toBe('$.data.lines[0].a');
  });

  it('steps back to the last match first', async () => {
    store.load(JSON_TEXT);
    await settle();

    store.step(-1);

    expect(store.matchIndex()).toBe(1);
  });

  it('does not step without matches', async () => {
    repository.view = jsonView();
    store.load(JSON_TEXT);
    await settle();

    store.step(1);

    expect(store.selectedPath()).toBeNull();
  });

  it('selects what it is told to', () => {
    store.select('$.data');

    expect(store.selectedPath()).toBe('$.data');
  });
});
