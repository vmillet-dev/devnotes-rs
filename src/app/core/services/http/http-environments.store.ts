import { Injectable, computed, inject, signal } from '@angular/core';
import { HttpEnvironmentsRepository } from '@core/data/http-environments.repository';
import type { FolderColour, HttpMethod, RequestKind } from '@core/ipc/bindings';
import {
  EnvironmentPatch,
  EnvironmentSets,
  EnvironmentTarget,
  HttpEnvironment,
  VariableRow,
  asksBeforeSending,
} from '@core/model/http-environment.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { debounced } from '@core/services/time/debounce';
import { HttpWorkspaceStore } from './http-workspace.store';

/** In the library's preferences: the environment a request is sent with belongs to its collections. */
const ACTIVE_KEY = 'devnotes.notes.http.environment';

/** A table being typed in is written once the keys rest, and on every way out. */
export const VARIABLES_SAVE_MS = 400;

interface PendingVariables {
  readonly target: EnvironmentTarget;
  readonly rows: readonly VariableRow[];
}

function keyOf(target: EnvironmentTarget): string {
  return target.kind === 'globals' ? 'globals' : target.id;
}

/**
 * The globals and the environments, and which one is active. Every write is persisted, then
 * Rust's answer adopted; a table being typed in keeps its rows as typed until they are written.
 */
@Injectable({ providedIn: 'root' })
export class HttpEnvironmentsStore {
  private readonly repository = inject(HttpEnvironmentsRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly workspace = inject(HttpWorkspaceStore);

  private readonly _sets = signal<EnvironmentSets>({ globals: [], environments: [] });
  readonly globals = computed(() => this._sets().globals);
  readonly environments = computed(() => this._sets().environments);

  private readonly _loaded = signal(false);
  readonly loaded = this._loaded.asReadonly();

  private readonly _activeId = signal<string | null>(this.preferences.read(ACTIVE_KEY) || null);
  /** `null` for none — the globals alone — and for one deleted since it was chosen. */
  readonly active = computed(
    () => this.environments().find((environment) => environment.id === this._activeId()) ?? null,
  );

  readonly isOpen = computed(() => this.workspace.page() === 'environments');

  private readonly _target = signal<EnvironmentTarget>({ kind: 'globals' });
  readonly target = this._target.asReadonly();
  /** The environment the page shows, `null` while it shows the globals. */
  readonly shown = computed<HttpEnvironment | null>(() => {
    const target = this._target();
    if (target.kind === 'globals') return null;
    return this.environments().find((environment) => environment.id === target.id) ?? null;
  });

  private readonly _drafts = signal<ReadonlyMap<string, readonly VariableRow[]>>(new Map());
  private pending: PendingVariables | null = null;
  private readonly saveLater = debounced<void>(() => void this.flush(), VARIABLES_SAVE_MS);

  /** What the page's table shows: the rows as typed, until they are written. */
  readonly rows = computed<readonly VariableRow[]>(() => {
    const target = this._target();
    const draft = this._drafts().get(keyOf(target));
    if (draft !== undefined) return draft;
    return target.kind === 'globals' ? this.globals() : (this.shown()?.variables ?? []);
  });

  private readonly _asking = signal<string | null>(null);
  /** The tab whose send waits for a yes, under a protected environment. */
  readonly asking = this._asking.asReadonly();

  /**
   * Whether a tab's send may leave now. Under a protected environment, what could change
   * something waits for a yes: the tab asks, and `confirmed` is that yes.
   */
  mayLeave(key: string, kind: RequestKind, method: HttpMethod, confirmed: boolean): boolean {
    if (confirmed || !this.active()?.protected || !asksBeforeSending(kind, method)) {
      if (this._asking() === key) this._asking.set(null);
      return true;
    }
    this._asking.set(key);
    return false;
  }

  stopAsking(): void {
    this._asking.set(null);
  }

  /** `false` when they could not be read: what was shown stays. */
  async load(): Promise<boolean> {
    const sets = await this.errors.attempt('http.environments.failed', () => this.repository.list());
    if (sets === null) return false;
    this._sets.set(sets);
    this._loaded.set(true);
    return true;
  }

  open(): void {
    this.workspace.show('environments');
  }

  toggle(): void {
    if (this.isOpen()) void this.flush();
    this.workspace.toggle('environments');
  }

  /** Remembered with the library; `null` sends with the globals alone. */
  choose(id: string | null): void {
    this._asking.set(null);
    this._activeId.set(id);
    this.preferences.write(ACTIVE_KEY, id ?? '');
  }

  select(target: EnvironmentTarget): void {
    void this.flush();
    this._target.set(target);
  }

  async create(name: string): Promise<boolean> {
    const created = await this.errors.attempt('http.environments.failed', () => this.repository.create(name));
    if (created === null) return false;
    this.adopt(created);
    this._target.set({ kind: 'environment', id: created.id });
    return true;
  }

  async rename(id: string, name: string): Promise<boolean> {
    return this.patch(id, { name });
  }

  async recolour(id: string, colour: FolderColour): Promise<boolean> {
    return this.patch(id, { colour });
  }

  async protect(id: string, protect: boolean): Promise<boolean> {
    return this.patch(id, { protected: protect });
  }

  async duplicate(id: string, name: string): Promise<boolean> {
    await this.flush();
    const copy = await this.errors.attempt('http.environments.failed', () =>
      this.repository.duplicate(id, name),
    );
    if (copy === null) return false;
    await this.load();
    this._target.set({ kind: 'environment', id: copy.id });
    return true;
  }

  async delete(id: string): Promise<boolean> {
    if (this.pending?.target.kind === 'environment' && this.pending.target.id === id) {
      this.saveLater.cancel();
      this.pending = null;
    }
    const deleted = await this.errors.attempt('http.environments.failed', async () => {
      await this.repository.delete(id);
      return true;
    });
    if (deleted === null) return false;
    this.forgetDraft(id);
    if (this._activeId() === id) this.choose(null);
    const target = this._target();
    if (target.kind === 'environment' && target.id === id) this._target.set({ kind: 'globals' });
    this._sets.update((sets) => ({
      ...sets,
      environments: sets.environments.filter((environment) => environment.id !== id),
    }));
    return true;
  }

  /**
   * A name nothing sets, given a row in the active environment — the globals with none — and
   * the page opened on it, its value to type.
   */
  define(name: string): void {
    const active = this.active();
    this.select(active === null ? { kind: 'globals' } : { kind: 'environment', id: active.id });
    if (!this.rows().some((row) => row.name === name)) {
      this.editVariables([...this.rows(), { enabled: true, name, value: '', kind: 'text' }]);
    }
    this.open();
  }

  /** The rows as typed are shown at once and written once the keys rest. */
  editVariables(rows: readonly VariableRow[]): void {
    const target = this._target();
    if (this.pending !== null && keyOf(this.pending.target) !== keyOf(target)) void this.flush();
    this._drafts.update((drafts) => new Map(drafts).set(keyOf(target), rows));
    this.pending = { target, rows };
    this.saveLater();
  }

  /** What is still waiting is written now: the page is left, another set chosen, a send made. */
  async flush(): Promise<void> {
    this.saveLater.cancel();
    const pending = this.pending;
    if (pending === null) return;
    this.pending = null;
    const { target, rows } = pending;
    if (target.kind === 'globals') {
      const saved = await this.errors.attempt('http.environments.failed', () =>
        this.repository.saveGlobals(rows),
      );
      if (saved !== null) this._sets.update((sets) => ({ ...sets, globals: saved }));
    } else {
      const saved = await this.errors.attempt('http.environments.failed', () =>
        this.repository.save(target.id, { variables: [...rows] }),
      );
      if (saved !== null) this.adopt(saved);
    }
    // Typed on meanwhile: those rows are the next write, and stay shown.
    if (this._drafts().get(keyOf(target)) === rows) this.forgetDraft(keyOf(target));
  }

  private async patch(id: string, patch: EnvironmentPatch): Promise<boolean> {
    const saved = await this.errors.attempt('http.environments.failed', () =>
      this.repository.save(id, patch),
    );
    if (saved === null) return false;
    this.adopt(saved);
    return true;
  }

  private adopt(environment: HttpEnvironment): void {
    this._sets.update((sets) => {
      const at = sets.environments.findIndex((held) => held.id === environment.id);
      const environments = [...sets.environments];
      if (at < 0) environments.push(environment);
      else environments[at] = environment;
      return { ...sets, environments };
    });
  }

  private forgetDraft(key: string): void {
    this._drafts.update((drafts) => {
      const next = new Map(drafts);
      next.delete(key);
      return next;
    });
  }
}
