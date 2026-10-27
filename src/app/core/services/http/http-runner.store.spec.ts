import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HTTP_RUN_SUBSCRIBER } from '@core/ipc/http-run-events';
import { IpcError } from '@core/ipc/ipc.error';
import { RunEvent, RunResult } from '@core/model/http-runner.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeHttpEnvironmentsRepository, environment } from '@testing/fake-http-environments-repository';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { request, sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpEnvironmentsStore } from './http-environments.store';
import { HttpRunnerStore } from './http-runner.store';
import { RailRow } from './http-tree';
import { HttpWorkspaceStore } from './http-workspace.store';

const API: RailRow = {
  kind: 'collection',
  id: 'API',
  name: 'API',
  depth: 0,
  collectionId: 'API',
  folderId: null,
  expanded: true,
  badge: null,
  requestKind: null,
};

function ended(name: string, overrides: Partial<RunResult> = {}): RunResult {
  return {
    iteration: 0,
    requestId: name,
    name,
    path: [],
    kind: 'http',
    method: 'GET',
    status: 200,
    reason: 'OK',
    millis: 3,
    tests: [],
    error: null,
    stop: null,
    skipped: false,
    response: null,
    ...overrides,
  };
}

describe('HttpRunnerStore', () => {
  let http: FakeHttpRepository;
  let environments: FakeHttpEnvironmentsRepository;
  let dialog: FakeFileDialog;
  let play: (event: RunEvent) => void;

  const store = () => TestBed.inject(HttpRunnerStore);

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    environments = new FakeHttpEnvironmentsRepository({
      environments: [environment('Production', { protected: true })],
    });
    dialog = new FakeFileDialog();
    TestBed.configureTestingModule({
      providers: [
        provideAppTesting({
          httpRepository: http,
          httpEnvironmentsRepository: environments,
          fileDialog: dialog,
        }),
        {
          provide: HTTP_RUN_SUBSCRIBER,
          useValue: (handler: (event: RunEvent) => void) => {
            play = handler;
            return Promise.resolve(() => undefined);
          },
        },
      ],
    });
    await TestBed.inject(HttpCollectionsStore).load();
    await TestBed.inject(HttpEnvironmentsStore).load();
    store();
  });

  it('opens on what was chosen, its requests in the tree’s order, on a page of its own', () => {
    store().open(API);
    expect(TestBed.inject(HttpWorkspaceStore).page()).toBe('runner');
    expect(store().target()?.name).toBe('API');
    expect(
      store()
        .rows()
        .map((row) => row.name),
    ).toEqual(['Login', 'Logout', 'Old', 'Health']);

    store().move(3, 0);
    store().toggle(1);
    expect(
      store()
        .rows()
        .map((row) => row.name),
    ).toEqual(['Health', 'Login', 'Logout', 'Old']);
    expect(
      store()
        .enabled()
        .map((row) => row.name),
    ).toEqual(['Health', 'Logout', 'Old']);
    // The same one again keeps the order as it was left.
    store().open(API);
    expect(store().rows()[0]?.name).toBe('Health');
  });

  /** An order rearranged was lost at restart, and a request added showed only after it (HSC-20). */
  it('keeps the order through Rust, and follows the requests added under the target', async () => {
    const names = () =>
      store()
        .rows()
        .map((row) => `${row.name}${row.enabled ? '' : ' (off)'}`);
    http.runOrders.set('API', [
      { id: 'Health', enabled: true },
      { id: 'Login', enabled: false },
      { id: 'Logout', enabled: true },
      { id: 'Old', enabled: true },
    ]);

    store().open(API);
    await vi.waitFor(() => expect(names()).toEqual(['Health', 'Login (off)', 'Logout', 'Old']));

    store().move(0, 3);
    store().toggle(0);
    await vi.waitFor(() => expect(http.callsOf('saveRunOrder')).toHaveLength(2));
    expect(http.callsOf('saveRunOrder')[1]).toEqual([
      { kind: 'collection', id: 'API' },
      [
        { id: 'Login', enabled: true },
        { id: 'Logout', enabled: true },
        { id: 'Old', enabled: true },
        { id: 'Health', enabled: true },
      ],
    ]);

    // Rust answers the order with the request added at its end; the rows follow the tree's change.
    http.runOrders.set('API', [...http.runOrders.get('API')!, { id: 'Ping', enabled: true }]);
    const grown = sampleTree();
    grown.collections[0]!.children.push(request('Ping', 'API'));
    http.tree$ = grown;
    await TestBed.inject(HttpCollectionsStore).load();
    TestBed.tick();
    await vi.waitFor(() => expect(names()).toEqual(['Login', 'Logout', 'Old', 'Health', 'Ping']));
  });

  it('runs what is enabled with its settings, and follows the steps Rust says', async () => {
    store().open(API);
    store().iterations.set(2);
    store().delayMs.set(50);
    store().stopOnFailure.set(true);
    store().keepResponses.set(true);
    let finish!: () => void;
    http.pendingRun = new Promise((resolve) => (finish = resolve));
    const running = store().start();
    await vi.waitFor(() => expect(http.callsOf('runCollection')).toHaveLength(1));
    const [asked] = http.callsOf('runCollection')[0] as [Record<string, unknown>];
    expect(asked).toMatchObject({
      item: { kind: 'collection', id: 'API' },
      order: ['Login', 'Logout', 'Old', 'Health'],
      iterations: 2,
      delayMs: 50,
      stopOnFailure: true,
      keepVariables: true,
      keepResponses: true,
      environmentId: null,
      confirmed: false,
    });
    expect(store().phase()).toBe('running');
    const id = asked['id'] as string;
    expect(store().consoleKey()).toBe(`run:${id}`);

    play({ id, step: { kind: 'started', iteration: 0, index: 0, name: 'Login' } });
    play({ id: 'another', step: { kind: 'ended', result: ended('Elsewhere') } });
    play({ id, step: { kind: 'ended', result: ended('Login') } });
    play({
      id,
      step: { kind: 'ended', result: ended('Logout', { error: { code: 'httpRefused', params: {} } }) },
    });
    expect(store().progress()?.name).toBe('Login');
    expect(
      store()
        .results()
        .map((result) => result.name),
    ).toEqual(['Login', 'Logout']);
    store().filter.set('failed');
    expect(
      store()
        .shown()
        .map((result) => result.name),
    ).toEqual(['Logout']);
    store().filter.set('passed');
    expect(
      store()
        .shown()
        .map((result) => result.name),
    ).toEqual(['Login']);

    await store().cancel();
    expect(http.callsOf('cancelRun')).toEqual([[id]]);
    play({ id, step: { kind: 'finished', cancelled: true, millis: 2400 } });
    finish();
    await running;
    expect(store().phase()).toBe('done');
    expect(store().ended()).toEqual({ cancelled: true, millis: 2400 });
    expect(store().progress()).toBeNull();
    expect(environments.callsTo('list').length).toBeGreaterThan(1);
  });

  it('asks once before a run under a protected environment, then runs with the yes', async () => {
    TestBed.inject(HttpEnvironmentsStore).choose('env-production');
    store().open(API);
    await store().start();
    expect(store().phase()).toBe('asking');
    expect(
      store()
        .changing()
        .map((row) => row.name),
    ).toEqual(['Health']);
    expect(http.callsOf('runCollection')).toEqual([]);
    store().stopAsking();
    expect(store().phase()).toBe('idle');

    await store().start(true);
    expect((http.callsOf('runCollection')[0]![0] as { confirmed: boolean }).confirmed).toBe(true);
    expect(store().phase()).toBe('done');
  });

  it('reads a data file chosen, iterates over its rows, and lets it go', async () => {
    store().open(API);
    dialog.openPath = 'C:/data/clients.csv';
    http.runData = { columns: ['customer'], rows: [{ customer: 'a' }, { customer: 'b' }, { customer: 'c' }] };
    await store().chooseData();
    expect(http.callsOf('readRunData')).toEqual([['C:/data/clients.csv']]);
    expect(store().data()?.name).toBe('clients.csv');
    expect(store().runs()).toBe(3);
    store().clearData();
    expect(store().runs()).toBe(1);

    dialog.openPath = null;
    await store().chooseData();
    expect(http.callsOf('readRunData')).toHaveLength(1);
  });

  it('says a run that could not go to its end, but not a cancel', async () => {
    store().open(API);
    http.failNext = new Error('down');
    await store().start();
    expect(TestBed.inject(ErrorNotifier).notice()?.ref.key).toBe('http.runner.failed');
    TestBed.inject(ErrorNotifier).dismiss();
    http.failNext = new IpcError('run_http_collection', { code: 'httpCancelled', params: {}, detail: '' });
    await store().start();
    expect(TestBed.inject(ErrorNotifier).notice()).toBeNull();
    await store().cancel();
    expect(http.callsOf('cancelRun')).toEqual([]);
  });
});
