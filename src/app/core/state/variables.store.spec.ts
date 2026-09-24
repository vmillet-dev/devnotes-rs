import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { VariablesStore } from './variables.store';

describe('VariablesStore', () => {
  let repository: FakeNotesRepository;
  let store: VariablesStore;

  beforeEach(() => {
    TestBed.resetTestingModule();
    repository = new FakeNotesRepository();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });
    store = TestBed.inject(VariablesStore);
  });

  it('reads the stored set as an editable list', async () => {
    await repository.saveVariables({ host: 'db.internal' });

    await store.load();

    expect(store.variables()).toEqual([{ name: 'host', value: 'db.internal' }]);
    expect(store.isEmpty()).toBe(false);
  });

  it('reports an empty set rather than a missing one', async () => {
    await store.load();

    expect(store.isEmpty()).toBe(true);
  });

  it('adds a blank row without writing anything', async () => {
    await store.load();

    store.add();

    expect(store.variables()).toEqual([{ name: '', value: '' }]);
    expect(await repository.loadVariables()).toEqual({});
  });

  it('writes the whole set on commit, dropping the half-filled rows', async () => {
    store.add();
    store.rename(0, 'host');
    store.setValue(0, 'db.internal');
    store.add();

    await store.commit();

    expect(await repository.loadVariables()).toEqual({ host: 'db.internal' });
    // The unfinished row stays on screen: it is waiting to be finished.
    expect(store.variables()).toHaveLength(2);
  });

  /** Staged like every other change in the panel: a removal cannot be taken back. */
  it('removing a row waits for the commit, like the rest', async () => {
    store.add();
    store.rename(0, 'host');
    store.setValue(0, 'db.internal');
    await store.commit();

    store.remove(0);

    expect(store.variables()).toEqual([]);
    expect(store.isDirty()).toBe(true);
    expect(await repository.loadVariables()).toEqual({ host: 'db.internal' });

    await store.commit();

    expect(await repository.loadVariables()).toEqual({});
  });

  it('reads the corpus again on discard, dropping what was typed', async () => {
    await repository.saveVariables({ host: 'db.internal' });
    await store.load();
    store.setValue(0, 'somewhere.else');

    await store.discard();

    expect(store.variables()).toEqual([{ name: 'host', value: 'db.internal' }]);
    expect(store.isDirty()).toBe(false);
  });

  /** The page is recreated every time the rail changes section. */
  it('refuses to reload over edits in hand', async () => {
    await repository.saveVariables({ host: 'db.internal' });
    await store.load();
    store.setValue(0, 'somewhere.else');

    await store.load();

    expect(store.variables()).toEqual([{ name: 'host', value: 'somewhere.else' }]);
  });

  it('trims a name, which would otherwise never match a token', () => {
    store.add();

    store.rename(0, '  host  ');

    expect(store.variables()[0].name).toBe('host');
  });

  it('flags two rows sharing a name', () => {
    store.add();
    store.rename(0, 'host');
    store.add();
    store.rename(1, 'host');

    expect([...store.duplicates()]).toEqual(['host']);
  });

  it('reports a failed read rather than emptying the panel in silence', async () => {
    repository.failNext = new Error('bridge down');

    await store.load();

    expect(TestBed.inject(ErrorNotifier).notice()?.ref.key).toBe('errors.variablesLoadFailed');
  });

  it('reports a failed write', async () => {
    repository.failNext = new Error('disk full');

    await store.commit();

    expect(TestBed.inject(ErrorNotifier).notice()?.ref.key).toBe('errors.variablesSaveFailed');
  });
});
