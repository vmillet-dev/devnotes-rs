import { Injectable, computed, inject, resource, signal } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { HistoryEntry, toParts } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { HttpSendStore } from './http-send.store';
import { HttpTabsStore } from './http-tabs.store';

/**
 * What was sent: the workspace shows it in place of the tabs while it is open. Read again each
 * time a send ends, and after it is emptied.
 */
@Injectable({ providedIn: 'root' })
export class HttpHistoryStore {
  private readonly repository = inject(HttpRepository);
  private readonly sending = inject(HttpSendStore);
  private readonly tabs = inject(HttpTabsStore);
  private readonly errors = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);

  private readonly _open = signal(false);
  readonly isOpen = this._open.asReadonly();
  private readonly _selectedId = signal<string | null>(null);
  readonly selectedId = this._selectedId.asReadonly();
  private readonly cleared = signal(0);

  private readonly daysResource = resource({
    params: () => (this._open() ? { sent: this.sending.sent(), cleared: this.cleared() } : undefined),
    // The offset is read when the list is, not in a `computed`, which would keep the first one.
    loader: () => this.repository.history(new Date().getTimezoneOffset()),
  });
  readonly days = computed(() => (this.daysResource.hasValue() ? this.daysResource.value() : []));
  readonly loaded = computed(() => this.daysResource.hasValue());

  private readonly entryResource = resource({
    params: () => this._selectedId() ?? undefined,
    loader: ({ params }) => this.repository.historyEntry(params),
  });
  readonly entry = computed<HistoryEntry | null>(() =>
    this.entryResource.hasValue() ? (this.entryResource.value() ?? null) : null,
  );

  open(): void {
    this._open.set(true);
  }

  close(): void {
    this._open.set(false);
  }

  toggle(): void {
    this._open.update((open) => !open);
  }

  select(id: string): void {
    this._selectedId.set(id);
  }

  /** What « Vider l'historique » says before it runs. */
  async count(): Promise<number | null> {
    return this.errors.attempt('http.history.failed', () => this.repository.countHistory());
  }

  async clear(): Promise<void> {
    const removed = await this.errors.attempt('http.history.failed', () => this.repository.clearHistory());
    if (removed === null) return;
    this._selectedId.set(null);
    this.cleared.update((count) => count + 1);
    this.status.notify({ key: 'http.history.cleared', params: { count: removed } });
  }

  /** The entry as a new draft tab, its secrets left to type again; the history closes on it. */
  async reopen(id: string): Promise<void> {
    const draft = await this.errors.attempt('http.history.failed', () => this.repository.historyDraft(id));
    if (draft === null) return;
    this.tabs.openDraft({
      name: draft.name,
      kind: 'http',
      method: draft.method,
      parts: toParts(draft.document),
    });
    this._open.set(false);
  }
}
