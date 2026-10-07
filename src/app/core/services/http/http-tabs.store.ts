import { Injectable, computed, inject, signal } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  HttpTree,
  KeyValueRow,
  OpenedRequest,
  RequestDraft,
  RequestParts,
  toDocument,
} from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { holdsRequest } from './http-tree';

/** In the library's preferences: which requests were open, and which one was in front. */
const TABS_KEY = 'devnotes.notes.http.tabs';

export interface RequestTab {
  /** The request's id once saved; `draft-n` for one that never was. */
  readonly key: string;
  readonly requestId: string | null;
  /** What Rust holds: `null` for a request never saved, which writes nothing until it is. */
  readonly saved: RequestDraft | null;
  readonly draft: RequestDraft;
}

/** Where a request never saved goes the first time it is. */
export interface SaveTarget {
  readonly collectionId: string;
  readonly folderId: string | null;
}

interface StoredTabs {
  readonly open: readonly string[];
  readonly active: string | null;
}

function draftOf(request: OpenedRequest): RequestDraft {
  return { name: request.name, kind: request.kind, method: request.method, parts: request.parts };
}

/** Plain data both: their JSON says whether they differ. */
function same(a: RequestDraft, b: RequestDraft): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * The open requests, one tab each. A tab edits a draft and saving writes it whole; a request
 * made from the `+` is a draft alone until saved, like a new note.
 */
@Injectable({ providedIn: 'root' })
export class HttpTabsStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly transloco = inject(TranslocoService);

  private readonly _tabs = signal<readonly RequestTab[]>([]);
  readonly tabs = this._tabs.asReadonly();

  private readonly _activeKey = signal<string | null>(null);
  readonly activeKey = this._activeKey.asReadonly();

  readonly active = computed(() => this._tabs().find((tab) => tab.key === this._activeKey()) ?? null);
  /** The request the rail lights up. */
  readonly activeRequestId = computed(() => this.active()?.requestId ?? null);

  private drafts = 0;
  private restored = false;

  isDirty(tab: RequestTab): boolean {
    return tab.saved === null || !same(tab.saved, tab.draft);
  }

  /** Once a page's life: the tabs left open last time, those still in the library. */
  async restore(): Promise<void> {
    if (this.restored) return;
    this.restored = true;
    const stored = this.readStored();
    for (const id of stored.open) {
      const request = await this.repository.request(id).catch(() => null);
      if (request) this.adopt(request);
    }
    const active = stored.active ?? stored.open.at(-1) ?? null;
    if (active !== null && this._tabs().some((tab) => tab.key === active)) this._activeKey.set(active);
    else this._activeKey.set(this._tabs()[0]?.key ?? null);
  }

  async open(requestId: string): Promise<void> {
    if (!this._tabs().some((tab) => tab.key === requestId)) {
      const request = await this.errors.attempt('http.failed', () => this.repository.request(requestId));
      if (!request) return;
      this.adopt(request);
    }
    this.activate(requestId);
  }

  newRequest(): void {
    const key = `draft-${++this.drafts}`;
    const draft: RequestDraft = {
      name: this.transloco.translate('http.tabs.untitled'),
      kind: 'http',
      method: 'GET',
      parts: { url: '', params: [], headers: [], description: '' },
    };
    this._tabs.update((tabs) => [...tabs, { key, requestId: null, saved: null, draft }]);
    this.activate(key);
  }

  activate(key: string): void {
    this._activeKey.set(key);
    this.persist();
  }

  edit(key: string, change: Partial<Pick<RequestDraft, 'name' | 'method'>>): void {
    this.update(key, (draft) => ({ ...draft, ...change }));
  }

  editParts(key: string, change: Partial<Pick<RequestParts, 'headers' | 'description'>>): void {
    this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, ...change } }));
  }

  /** The URL as typed, then the table rewritten from it by Rust, unless typing went on. */
  async editUrl(key: string, url: string): Promise<void> {
    this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, url } }));
    const params = this.find(key)?.draft.parts.params ?? [];
    const synced = await this.errors.attempt('http.failed', () =>
      this.repository.syncQuery(url, params, 'url'),
    );
    if (synced && this.find(key)?.draft.parts.url === url) {
      this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, params: synced.params } }));
    }
  }

  /** The table as edited, then the query string rewritten from it, unless editing went on. */
  async editParams(key: string, params: readonly KeyValueRow[]): Promise<void> {
    this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, params } }));
    const url = this.find(key)?.draft.parts.url ?? '';
    const synced = await this.errors.attempt('http.failed', () =>
      this.repository.syncQuery(url, params, 'params'),
    );
    if (synced && this.find(key)?.draft.parts.params === params) {
      this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, url: synced.url } }));
    }
  }

  /**
   * Answers the tab's key once written — a request never saved takes its id as one — or `null`
   * when nothing was: such a request needs a `target`.
   */
  async save(key: string, target?: SaveTarget): Promise<string | null> {
    const tab = this.find(key);
    if (!tab) return null;
    if (tab.requestId !== null) {
      const saved = await this.errors.attempt('http.failed', () =>
        this.repository.saveRequest(tab.requestId!, tab.draft),
      );
      if (!saved) return null;
      // Typing during the write keeps what was typed; otherwise the draft is what Rust kept.
      const current = this.find(key)?.draft ?? tab.draft;
      this.replace(key, {
        ...tab,
        saved: draftOf(saved),
        draft: current === tab.draft ? draftOf(saved) : current,
      });
      return key;
    }
    if (!target) return null;
    const created = await this.errors.attempt('http.failed', () =>
      this.repository.createRequest({
        collectionId: target.collectionId,
        folderId: target.folderId,
        name: tab.draft.name,
        kind: tab.draft.kind,
        method: tab.draft.method,
        document: toDocument(tab.draft.parts),
      }),
    );
    if (!created) return null;
    this.replace(key, {
      key: created.id,
      requestId: created.id,
      saved: draftOf(created),
      draft: draftOf(created),
    });
    if (this._activeKey() === key) this._activeKey.set(created.id);
    this.persist();
    return created.id;
  }

  close(key: string): void {
    const tabs = this._tabs();
    const at = tabs.findIndex((tab) => tab.key === key);
    if (at < 0) return;
    const rest = tabs.filter((tab) => tab.key !== key);
    this._tabs.set(rest);
    if (this._activeKey() === key) this._activeKey.set(rest[Math.min(at, rest.length - 1)]?.key ?? null);
    this.persist();
  }

  /** Closes the tabs of requests the tree no longer holds: deleted from the rail. */
  prune(tree: HttpTree): void {
    for (const tab of this._tabs()) {
      if (tab.requestId !== null && !holdsRequest(tree, tab.requestId)) this.close(tab.key);
    }
  }

  private adopt(request: OpenedRequest): void {
    const draft = draftOf(request);
    this._tabs.update((tabs) => [...tabs, { key: request.id, requestId: request.id, saved: draft, draft }]);
  }

  private find(key: string): RequestTab | undefined {
    return this._tabs().find((tab) => tab.key === key);
  }

  private update(key: string, change: (draft: RequestDraft) => RequestDraft): void {
    this._tabs.update((tabs) =>
      tabs.map((tab) => (tab.key === key ? { ...tab, draft: change(tab.draft) } : tab)),
    );
  }

  private replace(key: string, next: RequestTab): void {
    this._tabs.update((tabs) => tabs.map((tab) => (tab.key === key ? next : tab)));
  }

  private persist(): void {
    const stored: StoredTabs = {
      open: this._tabs().flatMap((tab) => (tab.requestId === null ? [] : [tab.requestId])),
      active: this.activeRequestId(),
    };
    this.preferences.write(TABS_KEY, JSON.stringify(stored));
  }

  /** Written by hand, it is read like any input: what is not a list of ids opens nothing. */
  private readStored(): StoredTabs {
    try {
      const stored = JSON.parse(this.preferences.read(TABS_KEY) ?? '{}') as Partial<StoredTabs>;
      const open = Array.isArray(stored.open) ? stored.open.filter((id) => typeof id === 'string') : [];
      return { open, active: typeof stored.active === 'string' ? stored.active : null };
    } catch {
      return { open: [], active: null };
    }
  }
}
