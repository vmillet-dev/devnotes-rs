import { Injectable, computed, effect, inject, signal, untracked } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  BodiesLeft,
  BodyKind,
  EMPTY_PARTS,
  HttpTree,
  KeyValueRow,
  NewPlace,
  OpenedRequest,
  RequestDraft,
  RequestKind,
  RequestParts,
  bodyOfKind,
  setBodyAside,
  toDocument,
} from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpSendStore } from './http-send.store';
import { HttpSocketsStore } from './http-sockets.store';
import { findRequest } from './http-tree';

/** In the library's preferences: which requests were open, and which one was in front. */
const TABS_KEY = 'devnotes.notes.http.tabs';

export interface RequestTab {
  /** The request's id once saved; `draft-n` for one that never was. */
  readonly key: string;
  readonly requestId: string | null;
  /** Where it is saved, what it inherits from; `null` until a draft is placed. */
  readonly place: SaveTarget | null;
  /** What Rust holds: `null` for a request never saved, which writes nothing until it is. */
  readonly saved: RequestDraft | null;
  readonly draft: RequestDraft;
}

/** Where a request never saved goes the first time it is. */
export interface SaveTarget {
  readonly collectionId: string;
  readonly folderId: string | null;
  /**
   * What the save creates on its way, when the place does not exist yet: a collection, which
   * takes the request at its root whatever the ids above say, or a folder under them.
   */
  readonly create?: NewPlace;
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
 *
 * ⚠️ A saved request's name and place are the tree's: the rail renames, moves and deletes there,
 * and a tab follows each tree that is read. A tab that kept the name it was opened with showed
 * the old one after a rename from the rail — and its next save wrote it back over the new one.
 */
@Injectable({ providedIn: 'root' })
export class HttpTabsStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly transloco = inject(TranslocoService);
  private readonly sending = inject(HttpSendStore);
  private readonly sockets = inject(HttpSocketsStore);
  private readonly collections = inject(HttpCollectionsStore);

  private readonly _tabs = signal<readonly RequestTab[]>([]);
  readonly tabs = this._tabs.asReadonly();

  private readonly _activeKey = signal<string | null>(null);
  readonly activeKey = this._activeKey.asReadonly();

  readonly active = computed(() => this._tabs().find((tab) => tab.key === this._activeKey()) ?? null);
  /** The request the rail lights up. */
  readonly activeRequestId = computed(() => this.active()?.requestId ?? null);

  private drafts = 0;
  private restored = false;
  /** By tab, for as long as it is open: never written, a request saves the body it shows. */
  private readonly bodiesLeft = new Map<string, BodiesLeft>();

  constructor() {
    effect(() => {
      const tree = this.collections.tree();
      // Not before a first read: an empty tree would close every tab.
      if (!this.collections.loaded()) return;
      // The tabs are not a reason to run again: only a tree read after a write is.
      untracked(() => this.follow(tree));
    });
  }

  isDirty(tab: RequestTab): boolean {
    return tab.saved === null || !same(tab.saved, tab.draft);
  }

  /**
   * The names of the open tabs, among these requests, whose changes are not saved: what a
   * deletion would lose. Only the front knows it — Rust counts what is deleted, not what is typed.
   */
  unsavedAmong(requestIds: ReadonlySet<string>): string[] {
    return this._tabs()
      .filter((tab) => tab.requestId !== null && requestIds.has(tab.requestId) && this.isDirty(tab))
      .map((tab) => tab.draft.name);
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

  /** Of the kind asked, for good: a request is not turned into a socket afterwards. */
  newRequest(kind: RequestKind = 'http'): void {
    const key = `draft-${++this.drafts}`;
    const draft: RequestDraft = {
      name: this.transloco.translate('http.tabs.untitled'),
      kind,
      method: 'GET',
      parts: EMPTY_PARTS,
    };
    this._tabs.update((tabs) => [...tabs, { key, requestId: null, place: null, saved: null, draft }]);
    this.activate(key);
  }

  /** A request rebuilt from elsewhere — the history — as a draft: nothing written until saved. */
  openDraft(draft: RequestDraft): void {
    const key = `draft-${++this.drafts}`;
    this._tabs.update((tabs) => [...tabs, { key, requestId: null, place: null, saved: null, draft }]);
    this.activate(key);
  }

  activate(key: string): void {
    this._activeKey.set(key);
    this.persist();
  }

  edit(key: string, change: Partial<Pick<RequestDraft, 'name' | 'method'>>): void {
    this.update(key, (draft) => ({ ...draft, ...change }));
  }

  editParts(
    key: string,
    change: Partial<
      Pick<RequestParts, 'headers' | 'description' | 'body' | 'auth' | 'graphql' | 'websocket' | 'transport'>
    >,
  ): void {
    this.update(key, (draft) => ({ ...draft, parts: { ...draft.parts, ...change } }));
  }

  /**
   * Another kind of body. The one left is set aside, and comes back as it was when its kind is
   * chosen again: « Passer à Aucun » on a `GET` would otherwise cost the text it warns about.
   */
  chooseBodyKind(key: string, kind: BodyKind): void {
    const current = this.find(key)?.draft.parts.body;
    if (current === undefined || current.kind === kind) return;
    const left = setBodyAside(this.bodiesLeft.get(key) ?? {}, current);
    this.bodiesLeft.set(key, left);
    this.editParts(key, { body: bodyOfKind(kind, left) });
  }

  /**
   * What a tab edits, replaced by a send put back onto its request: what is saved is not
   * touched, so the tab reads as modified and « Enregistrer » is what writes it.
   */
  replaceDraft(key: string, change: Pick<RequestDraft, 'method' | 'parts'>): void {
    this.update(key, (draft) => ({ ...draft, ...change }));
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
      // The rail shows its name and its method: it reads what was just written.
      await this.collections.load();
      return key;
    }
    if (!target) return null;
    const draft = {
      collectionId: target.collectionId,
      folderId: target.folderId,
      name: tab.draft.name,
      kind: tab.draft.kind,
      method: tab.draft.method,
      document: toDocument(tab.draft.parts),
    };
    const create = target.create;
    // One write for the request and what it lacks: a request that is refused leaves no
    // empty collection behind it.
    const created = await this.errors.attempt('http.failed', () =>
      create ? this.repository.createRequestIn(draft, create) : this.repository.createRequest(draft),
    );
    if (!created) return null;
    this.replace(key, {
      key: created.id,
      requestId: created.id,
      place: { collectionId: created.collectionId, folderId: created.folderId },
      saved: draftOf(created),
      draft: draftOf(created),
    });
    this.sending.rekey(key, created.id);
    this.sockets.rekey(key, created.id);
    const left = this.bodiesLeft.get(key);
    if (left !== undefined) {
      this.bodiesLeft.delete(key);
      this.bodiesLeft.set(created.id, left);
    }
    if (this._activeKey() === key) this._activeKey.set(created.id);
    this.persist();
    await this.collections.load();
    this.collections.reveal(created.id);
    return created.id;
  }

  close(key: string): void {
    const tabs = this._tabs();
    const at = tabs.findIndex((tab) => tab.key === key);
    if (at < 0) return;
    const rest = tabs.filter((tab) => tab.key !== key);
    this._tabs.set(rest);
    this.sending.forget(key);
    this.sockets.forget(key);
    this.bodiesLeft.delete(key);
    if (this._activeKey() === key) this._activeKey.set(rest[Math.min(at, rest.length - 1)]?.key ?? null);
    this.persist();
  }

  /**
   * What the rail did to an open request reaches its tab: deleted, the tab closes; moved, it
   * inherits from where it now sits; renamed, it takes the name — in what is saved and in what
   * is being edited, so the next save does not write the old one back.
   */
  private follow(tree: HttpTree): void {
    for (const tab of this._tabs()) {
      if (tab.requestId === null || tab.saved === null) continue;
      const held = findRequest(tree, tab.requestId);
      if (held === null) {
        this.close(tab.key);
        continue;
      }
      const at = tab.place;
      const moved = at === null || at.collectionId !== held.collectionId || at.folderId !== held.folderId;
      const renamed = tab.saved.name !== held.name;
      if (!moved && !renamed) continue;
      this.replace(tab.key, {
        ...tab,
        place: { collectionId: held.collectionId, folderId: held.folderId },
        saved: renamed ? { ...tab.saved, name: held.name } : tab.saved,
        draft: renamed ? { ...tab.draft, name: held.name } : tab.draft,
      });
    }
  }

  private adopt(request: OpenedRequest): void {
    const draft = draftOf(request);
    this._tabs.update((tabs) => [
      ...tabs,
      {
        key: request.id,
        requestId: request.id,
        place: { collectionId: request.collectionId, folderId: request.folderId },
        saved: draft,
        draft,
      },
    ]);
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
