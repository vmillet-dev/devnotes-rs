import { Injectable, computed, inject, resource, signal } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { HistoryEntry, HistoryItem, HttpRequestSummary, toParts } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { debounced } from '@core/services/time/debounce';
import { UNDO_WINDOW_MS } from '@core/state/undo.store';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpSendStore } from './http-send.store';
import { HttpTabsStore } from './http-tabs.store';
import { HttpWorkspaceStore } from './http-workspace.store';
import { findRequest } from './http-tree';

/** What « Rouvrir » waits on: the request's tab is open with changes of its own. */
export interface Replacing {
  readonly entryId: string;
  readonly name: string;
}

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
  private readonly collections = inject(HttpCollectionsStore);
  private readonly transloco = inject(TranslocoService);
  private readonly workspace = inject(HttpWorkspaceStore);

  readonly isOpen = computed(() => this.workspace.page() === 'history');
  private readonly _selectedId = signal<string | null>(null);
  readonly selectedId = this._selectedId.asReadonly();
  private readonly cleared = signal(0);

  /** The send just deleted, for as long as its undo is offered. */
  private readonly _removed = signal<HistoryEntry | null>(null);
  readonly removed = this._removed.asReadonly();
  private readonly forgetRemoved = debounced<void>(() => this._removed.set(null), UNDO_WINDOW_MS);

  private readonly _replacing = signal<Replacing | null>(null);
  readonly replacing = this._replacing.asReadonly();

  private readonly daysResource = resource({
    params: () => (this.isOpen() ? { sent: this.sending.sent(), cleared: this.cleared() } : undefined),
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
    this.workspace.show('history');
  }

  close(): void {
    this.workspace.leave('history');
  }

  toggle(): void {
    this.workspace.toggle('history');
  }

  select(id: string): void {
    this._selectedId.set(id);
    this._replacing.set(null);
  }

  /**
   * One send out of the history — the one made with the wrong token. Not asked first: it is
   * put back from the bar that follows, as a note comes back from the trash.
   */
  async delete(id: string): Promise<void> {
    const removed = await this.errors.attempt('http.history.failed', () =>
      this.repository.deleteHistoryEntry(id),
    );
    if (removed === null) return;
    if (this._selectedId() === id) this._selectedId.set(null);
    this._removed.set(removed);
    this.forgetRemoved();
    this.cleared.update((count) => count + 1);
  }

  async undoDelete(): Promise<void> {
    const removed = this._removed();
    if (removed === null) return;
    this.dismissUndo();
    const restored = await this.errors.attempt('errors.undoFailed', async () => {
      await this.repository.restoreHistoryEntry(removed);
      return true;
    });
    if (restored) this.cleared.update((count) => count + 1);
  }

  dismissUndo(): void {
    this.forgetRemoved.cancel();
    this._removed.set(null);
  }

  /** The request a send came from, while the tree still holds it: where « Rouvrir » puts it back. */
  requestOf(item: HistoryItem): HttpRequestSummary | null {
    return item.requestId === null ? null : findRequest(this.collections.tree(), item.requestId);
  }

  cancelReplace(): void {
    this._replacing.set(null);
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

  /**
   * A send goes back **onto the request it came from**: its own tab, the send's values as
   * unsaved changes — « Enregistrer » writes them, closing asks as for any change. It used to
   * open a stranger, a draft with no tie to it. A tab already open with changes of its own is
   * not written over without asking (`replace`). The request gone, or the send made from a
   * draft, it is a draft as before, named after the send and its time.
   */
  async reopen(entry: HistoryEntry, replace = false): Promise<void> {
    const id = entry.item.id;
    const held = this.requestOf(entry.item);
    if (held !== null) {
      const tab = this.tabs.tabs().find((open) => open.key === held.id);
      if (tab !== undefined && this.tabs.isDirty(tab) && !replace) {
        this._replacing.set({ entryId: id, name: held.name });
        return;
      }
      // Wrapped: `null` is an answer here — the request went meanwhile — and not a failure.
      const answer = await this.errors.attempt('http.history.failed', async () => ({
        onto: await this.repository.historyOnto(id),
      }));
      if (answer === null) return;
      if (answer.onto !== null) {
        await this.tabs.open(held.id);
        this.tabs.replaceDraft(held.id, {
          method: answer.onto.method,
          parts: toParts(answer.onto.document),
        });
        this.leave();
        return;
      }
    }
    const draft = await this.errors.attempt('http.history.failed', () => this.repository.historyDraft(id));
    if (draft === null) return;
    const time = new Intl.DateTimeFormat(this.transloco.getActiveLang(), {
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(entry.item.sentAt));
    this.tabs.openDraft({
      name: `${draft.name} · ${time}`,
      kind: draft.kind,
      method: draft.method,
      parts: toParts(draft.document),
    });
    this.leave();
  }

  private leave(): void {
    this._replacing.set(null);
    this.workspace.leave('history');
  }
}
