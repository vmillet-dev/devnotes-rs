import { Injectable, computed, effect, inject, signal, untracked } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { HTTP_RUN_SUBSCRIBER } from '@core/ipc/http-run-events';
import { IpcError } from '@core/ipc/ipc.error';
import { HttpItem } from '@core/model/http.model';
import {
  RunEvent,
  RunResult,
  RunnerData,
  RunnerFilter,
  RunnerRow,
  changes,
  failed,
  moved,
  rowsUnder,
  stepped,
  summaryOf,
} from '@core/model/http-runner.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { subscribeCancellable } from '@core/utils/subscription.util';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpConsoleStore } from './http-console.store';
import { HttpEnvironmentsStore } from './http-environments.store';
import { RailRow } from './http-tree';
import { HttpWorkspaceStore } from './http-workspace.store';

/** What a run is of: a collection or a folder, by its name. */
export interface RunnerTarget {
  readonly item: HttpItem;
  readonly name: string;
}

export type RunnerPhase = 'idle' | 'asking' | 'running' | 'done';

/** Where a run stands while it goes: which iteration, which request. */
export interface RunnerProgress {
  readonly iteration: number;
  readonly index: number;
  readonly name: string;
}

/**
 * The runner: what is run and how, its order, and the report of the last run — kept for the
 * session, never in the history. Rust runs; this follows its steps.
 */
@Injectable({ providedIn: 'root' })
export class HttpRunnerStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly dialogs = inject(FileDialogService);
  private readonly collections = inject(HttpCollectionsStore);
  private readonly environments = inject(HttpEnvironmentsStore);
  private readonly workspace = inject(HttpWorkspaceStore);
  private readonly console = inject(HttpConsoleStore);

  private readonly _target = signal<RunnerTarget | null>(null);
  readonly target = this._target.asReadonly();
  private readonly _rows = signal<readonly RunnerRow[]>([]);
  readonly rows = this._rows.asReadonly();

  readonly iterations = signal(1);
  readonly delayMs = signal(0);
  readonly stopOnFailure = signal(false);
  readonly keepVariables = signal(true);
  readonly keepResponses = signal(false);
  private readonly _data = signal<RunnerData | null>(null);
  readonly data = this._data.asReadonly();

  private readonly _phase = signal<RunnerPhase>('idle');
  readonly phase = this._phase.asReadonly();
  private readonly _results = signal<readonly RunResult[]>([]);
  readonly results = this._results.asReadonly();
  private readonly _progress = signal<RunnerProgress | null>(null);
  readonly progress = this._progress.asReadonly();
  private readonly _ended = signal<{ readonly cancelled: boolean; readonly millis: number } | null>(null);
  readonly ended = this._ended.asReadonly();
  private readonly _planned = signal(0);
  /** The requests the last run was to send, iterations included: an order changed since is not it. */
  readonly planned = this._planned.asReadonly();
  private readonly _runId = signal<string | null>(null);
  /** The console of the last run, under its own key. */
  readonly consoleKey = computed(() => {
    const id = this._runId();
    return id === null ? null : `run:${id}`;
  });
  readonly filter = signal<RunnerFilter>('all');

  readonly enabled = computed(() => this._rows().filter((row) => row.enabled));
  /** With a data file, one iteration a row: the field says so, greyed. */
  readonly runs = computed(() => this._data()?.data.rows.length ?? Math.max(1, this.iterations()));
  readonly summary = computed(() => summaryOf(this._results()));
  readonly shown = computed(() => {
    const filter = this.filter();
    return this._results().filter((result) => filter === 'all' || (filter === 'failed') === failed(result));
  });
  /** What a protected environment asks about before the run: the requests that could change it. */
  readonly changing = computed(() => this.enabled().filter(changes));

  private runs_ = 0;
  /** The last order written: an order read meanwhile would be the one before it. */
  private saving: Promise<unknown> = Promise.resolve();

  constructor() {
    subscribeCancellable(inject(HTTP_RUN_SUBSCRIBER), (event: RunEvent) => this.take(event));
    // A request added or deleted under the target while it is open: the order follows.
    effect(() => {
      this.collections.tree();
      const target = untracked(this._target);
      if (target !== null) void this.follow(target.item);
    });
  }

  /**
   * « Exécuter… » on a collection or a folder: its requests in the tree's order at once, then in
   * the order last arranged, kept by Rust with the requests added since at its end.
   */
  open(row: RailRow): void {
    const item = { kind: row.kind, id: row.id };
    const same = this._target()?.item.id === item.id;
    this._target.set({ item, name: row.name });
    if (!same) {
      this._rows.set(rowsUnder(this.collections.tree(), item));
      this.clearReport();
    }
    void this.follow(item);
    this.workspace.show('runner');
  }

  move(from: number, to: number): void {
    this._rows.set(moved(this._rows(), from, to));
    this.keep();
  }

  toggle(index: number): void {
    this._rows.update((rows) =>
      rows.map((row, at) => (at === index ? { ...row, enabled: !row.enabled } : row)),
    );
    this.keep();
  }

  private async follow(item: HttpItem): Promise<void> {
    await this.saving;
    const order = await this.errors.attempt('http.runner.failed', () => this.repository.runOrder(item));
    if (order === null || this._target()?.item.id !== item.id) return;
    const known = new Map(rowsUnder(this.collections.tree(), item).map((row) => [row.id, row]));
    this._rows.set(
      order.flatMap(({ id, enabled }) => {
        const row = known.get(id);
        return row === undefined ? [] : [{ ...row, enabled }];
      }),
    );
  }

  private keep(): void {
    const target = this._target();
    if (target === null) return;
    const order = this._rows().map(({ id, enabled }) => ({ id, enabled }));
    this.saving = this.saving.then(() =>
      this.errors.attempt('http.runner.failed', () => this.repository.saveRunOrder(target.item, order)),
    );
  }

  async chooseData(): Promise<void> {
    const path = await this.dialogs.pickFile();
    if (path === null) return;
    const data = await this.errors.attempt('http.runner.dataFailed', () => this.repository.readRunData(path));
    if (data !== null) this._data.set({ name: path.split(/[\\/]/).pop() ?? path, data });
  }

  clearData(): void {
    this._data.set(null);
  }

  /** `confirmed`: the one yes a protected environment asks for, before anything leaves. */
  async start(confirmed = false): Promise<void> {
    const target = this._target();
    if (target === null || this._phase() === 'running' || this.enabled().length === 0) return;
    if (!confirmed && this.environments.active()?.protected && this.changing().length > 0) {
      this._phase.set('asking');
      return;
    }
    const id = `${++this.runs_}-${Date.now().toString(36)}`;
    const previous = this.consoleKey();
    if (previous !== null) this.console.forget(previous);
    this._runId.set(id);
    this.clearReport();
    this._planned.set(this.enabled().length * this.runs());
    this._phase.set('running');
    try {
      await this.repository.runCollection({
        id,
        item: target.item,
        order: this.enabled().map((row) => row.id),
        iterations: this.runs(),
        delayMs: Math.max(0, this.delayMs()),
        data: this._data()?.data ?? null,
        stopOnFailure: this.stopOnFailure(),
        keepVariables: this.keepVariables(),
        keepResponses: this.keepResponses(),
        environmentId: this.environments.active()?.id ?? null,
        confirmed,
      });
    } catch (error) {
      console.error(error);
      if (!(error instanceof IpcError && error.code === 'httpCancelled')) {
        this.errors.reportFailure('http.runner.failed', error);
      }
    } finally {
      this._phase.set('done');
      this._progress.set(null);
      // What the scripts and the extractions wrote, read again: kept, it is the library's now.
      if (this.keepVariables()) {
        await this.environments.load();
        this.collections.settingsSaved();
      }
    }
  }

  stopAsking(): void {
    if (this._phase() === 'asking') this._phase.set(this._results().length > 0 ? 'done' : 'idle');
  }

  async cancel(): Promise<void> {
    const id = this._runId();
    if (id === null || this._phase() !== 'running') return;
    await this.errors.attempt('http.runner.failed', () => this.repository.cancelRun(id));
  }

  private clearReport(): void {
    this._results.set([]);
    this._progress.set(null);
    this._ended.set(null);
    this.filter.set('all');
    if (this._phase() !== 'running') this._phase.set('idle');
  }

  private take(event: RunEvent): void {
    if (event.id !== this._runId()) return;
    const { step } = event;
    switch (step.kind) {
      case 'started':
        this._progress.set({ iteration: step.iteration, index: step.index, name: step.name });
        return;
      case 'ended':
        this._results.update((results) => stepped(results, step));
        return;
      case 'finished':
        this._ended.set({ cancelled: step.cancelled, millis: step.millis });
        return;
    }
  }
}
