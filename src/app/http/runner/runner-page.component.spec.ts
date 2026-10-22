import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HTTP_RUN_SUBSCRIBER } from '@core/ipc/http-run-events';
import { RunEvent } from '@core/model/http-runner.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { HttpRunnerStore } from '@core/services/http/http-runner.store';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeHttpEnvironmentsRepository, environment } from '@testing/fake-http-environments-repository';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { RunnerPageComponent } from './runner-page.component';

describe('RunnerPageComponent', () => {
  let fixture: ComponentFixture<RunnerPageComponent>;
  let http: FakeHttpRepository;
  let dialog: FakeFileDialog;
  let play: (event: RunEvent) => void;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    dialog = new FakeFileDialog();
    TestBed.configureTestingModule({
      imports: [RunnerPageComponent],
      providers: [
        provideAppTesting({
          httpRepository: http,
          fileDialog: dialog,
          httpEnvironmentsRepository: new FakeHttpEnvironmentsRepository({
            environments: [environment('Production', { protected: true, colour: 'red' })],
          }),
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
    TestBed.inject(HttpRunnerStore).open({
      kind: 'folder',
      id: 'Auth',
      name: 'Auth',
      depth: 1,
      collectionId: 'API',
      folderId: null,
      expanded: true,
      badge: null,
      requestKind: null,
    });
    fixture = TestBed.createComponent(RunnerPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const root = () => fixture.nativeElement as HTMLElement;
  const el = <E extends HTMLElement = HTMLElement>(testId: string) =>
    root().querySelector<E>(`[data-testid="${testId}"]`);
  const text = (testId: string) => el(testId)?.textContent?.replace(/\s+/g, ' ').trim() ?? null;
  const store = () => TestBed.inject(HttpRunnerStore);

  it('says what it would run before the first run, and its settings change the store', async () => {
    expect(text('http-runner-title')).toBe('Auth');
    expect(text('http-runner-environment')).toBe('Aucun environnement');
    expect(text('http-runner-ready')).toBe('2 requêtes · 1 itération · Aucun environnement');

    const iterations = el<HTMLInputElement>('http-runner-iterations')!;
    iterations.value = '3';
    iterations.dispatchEvent(new Event('change'));
    const delay = el<HTMLInputElement>('http-runner-delay')!;
    delay.value = 'abc';
    delay.dispatchEvent(new Event('change'));
    delay.value = '100';
    delay.dispatchEvent(new Event('change'));
    for (const [testId, read] of [
      ['http-runner-stop', () => store().stopOnFailure()],
      ['http-runner-keep-variables', () => store().keepVariables()],
      ['http-runner-keep-responses', () => store().keepResponses()],
    ] as const) {
      const box = el<HTMLInputElement>(testId)!;
      const before = read();
      box.click();
      expect(read()).toBe(!before);
    }
    await fixture.whenStable();
    expect(store().iterations()).toBe(3);
    expect(store().delayMs()).toBe(100);
    expect(text('http-runner-ready')).toContain('3 itérations');
  });

  it('takes a data file and lets it go', async () => {
    dialog.openPath = 'C:/data/clients.csv';
    http.runData = { columns: ['customer', 'amount'], rows: [{ customer: 'a' }, { customer: 'b' }] };
    el<HTMLButtonElement>('http-runner-data-choose')!.click();
    await vi.waitFor(() => expect(text('http-runner-data')).toContain('clients.csv'));
    expect(text('http-runner-data')).toContain('2 lignes · 2 colonnes');
    expect(el<HTMLInputElement>('http-runner-iterations')!.disabled).toBe(true);
    el<HTMLButtonElement>('http-runner-data-clear')!.click();
    await fixture.whenStable();
    expect(el('http-runner-data')).toBeNull();
  });

  it('asks once under a protected environment, then runs, shows its progress, its report and its console', async () => {
    TestBed.inject(HttpEnvironmentsStore).choose('env-production');
    store().toggle(0);
    store().toggle(0);
    // Login and Logout are GETs: nothing to ask; Health, a POST, is in the collection only.
    store().open({ ...rowOfApi });
    await fixture.whenStable();
    el<HTMLButtonElement>('http-runner-start')!.click();
    await fixture.whenStable();
    expect(text('http-runner-protected-confirm')).toContain('Production est protégé : 1 requête sur 4 peut le modifier');
    expect(text('http-runner-protected-confirm')).toContain('POST Health');

    let finish!: () => void;
    http.pendingRun = new Promise((resolve) => (finish = resolve));
    el<HTMLButtonElement>('http-runner-protected-send')!.click();
    await vi.waitFor(() => expect(http.callsOf('runCollection')).toHaveLength(1));
    const id = (http.callsOf('runCollection')[0]![0] as { id: string }).id;
    play({ id, step: { kind: 'started', iteration: 0, index: 0, name: 'Login' } });
    await fixture.whenStable();
    expect(text('http-runner-progress')).toBe('Itération 1 / 1 · 1 / 4 · Login…');
    el<HTMLButtonElement>('http-runner-cancel')!.click();
    await vi.waitFor(() => expect(http.callsOf('cancelRun')).toHaveLength(1));

    play({
      id,
      step: {
        kind: 'ended',
        result: {
          iteration: 0,
          requestId: 'Login',
          name: 'Login',
          path: ['Auth'],
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
        },
      },
    });
    play({ id, step: { kind: 'finished', cancelled: true, millis: 12 } });
    finish();
    await vi.waitFor(() => expect(el('http-runner-start')).not.toBeNull());
    expect(text('http-runner-start')).toBe('▶ Relancer');
    expect(el('http-runner-results')).not.toBeNull();
    expect(el('http-console-unfold') ?? el('http-console')).not.toBeNull();
  });
});

const rowOfApi = {
  kind: 'collection' as const,
  id: 'API',
  name: 'API',
  depth: 0,
  collectionId: 'API',
  folderId: null,
  expanded: true,
  badge: null,
  requestKind: null,
};
