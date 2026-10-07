import { guard } from './fail-next';
import { HttpRepository } from '@core/data/http.repository';
import {
  HttpCollection,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpPlace,
  HttpRequestDraft,
  HttpTree,
  BodyAnswer,
  ContainerSettingsDraft,
  EMPTY_PARTS,
  InheritedParts,
  KeyValueRow,
  RequestBodyDraft,
  OpenedRequest,
  QuerySide,
  RequestDraft,
  toParts,
} from '@core/model/http.model';

export interface HttpCall {
  readonly command: string;
  readonly args: readonly unknown[];
}

interface Synced {
  readonly url: string;
  readonly params: readonly KeyValueRow[];
}

/**
 * Answers what a spec set and records what it was asked: the tree and the query's sync are
 * Rust's, so a spec sets `tree$` and `synced` to what Rust would answer rather than this fake
 * computing them.
 */
export class FakeHttpRepository implements Pick<HttpRepository, keyof HttpRepository> {
  tree$: HttpTree = { collections: [] };
  contentsAnswer: HttpContents = { folders: 0, requests: 0 };
  settingsAnswer: ContainerSettingsDraft = { auth: { kind: 'inherit' }, headers: [] };
  inheritedAnswer: InheritedParts = { auth: { kind: 'none' }, authFrom: null, headers: [] };
  bodyAnswer: BodyAnswer = { contentType: null, problem: null };
  /** What `syncQuery` answers; unset, it hands back what it was given. */
  synced: Synced | null = null;
  readonly requests = new Map<string, OpenedRequest>();
  readonly calls: HttpCall[] = [];
  failNext: Error | null = null;
  private nextId = 0;

  /** The calls of one command, their arguments in order. */
  callsOf(command: string): (readonly unknown[])[] {
    return this.calls.filter((call) => call.command === command).map((call) => call.args);
  }

  /** A request Rust would hold, for a spec to open. */
  seed(
    id: string,
    draft: Partial<RequestDraft> = {},
    collectionId = 'API',
    folderId: string | null = null,
  ): OpenedRequest {
    const request: OpenedRequest = {
      id,
      collectionId,
      folderId,
      name: draft.name ?? id,
      kind: draft.kind ?? 'http',
      method: draft.method ?? 'GET',
      parts: draft.parts ?? EMPTY_PARTS,
      createdAt: '2026-10-07T00:00:00.000Z',
      updatedAt: '2026-10-07T00:00:00.000Z',
    };
    this.requests.set(id, request);
    return request;
  }

  tree(): Promise<HttpTree> {
    return this.record('tree', [], () => this.tree$);
  }

  createCollection(name: string): Promise<HttpCollection> {
    return this.record('createCollection', [name], () => ({
      id: `collection-${++this.nextId}`,
      name,
      position: 0,
      createdAt: '2026-10-07T00:00:00.000Z',
    }));
  }

  createFolder(collectionId: string, parentId: string | null, name: string): Promise<HttpFolder> {
    return this.record('createFolder', [collectionId, parentId, name], () => ({
      id: `folder-${++this.nextId}`,
      collectionId,
      parentId,
      name,
      position: 0,
    }));
  }

  createRequest(draft: HttpRequestDraft): Promise<OpenedRequest> {
    return this.record('createRequest', [draft], () =>
      this.seed(
        `request-${++this.nextId}`,
        { name: draft.name, kind: draft.kind, method: draft.method, parts: toParts(draft.document) },
        draft.collectionId,
        draft.folderId,
      ),
    );
  }

  request(id: string): Promise<OpenedRequest> {
    return this.record('request', [id], () => {
      const request = this.requests.get(id);
      if (!request) throw new Error(`no request ${id}`);
      return request;
    });
  }

  saveRequest(id: string, draft: RequestDraft): Promise<OpenedRequest> {
    return this.record('saveRequest', [id, draft], () => {
      const request = this.requests.get(id);
      if (!request) throw new Error(`no request ${id}`);
      const saved: OpenedRequest = { ...request, name: draft.name, method: draft.method, parts: draft.parts };
      this.requests.set(id, saved);
      return saved;
    });
  }

  syncQuery(url: string, params: readonly KeyValueRow[], edited: QuerySide): Promise<Synced> {
    return this.record('syncQuery', [url, params, edited], () => this.synced ?? { url, params });
  }

  settings(item: HttpItem): Promise<ContainerSettingsDraft> {
    return this.record('settings', [item], () => this.settingsAnswer);
  }

  saveSettings(item: HttpItem, settings: ContainerSettingsDraft): Promise<void> {
    return this.record('saveSettings', [item, settings], () => undefined);
  }

  inherited(collectionId: string, folderId: string | null): Promise<InheritedParts> {
    return this.record('inherited', [collectionId, folderId], () => this.inheritedAnswer);
  }

  describeBody(body: RequestBodyDraft): Promise<BodyAnswer> {
    return this.record('describeBody', [body], () => this.bodyAnswer);
  }

  rename(item: HttpItem, name: string): Promise<void> {
    return this.record('rename', [item, name], () => undefined);
  }

  contents(item: HttpItem): Promise<HttpContents> {
    return this.record('contents', [item], () => this.contentsAnswer);
  }

  delete(item: HttpItem): Promise<void> {
    return this.record('delete', [item], () => {
      this.requests.delete(item.id);
    });
  }

  duplicate(item: HttpItem, name: string): Promise<HttpItem> {
    return this.record('duplicate', [item, name], () => ({ kind: item.kind, id: `copy-${++this.nextId}` }));
  }

  move(item: HttpItem, place: HttpPlace): Promise<void> {
    return this.record('move', [item, place], () => undefined);
  }

  reorderCollection(id: string, index: number): Promise<void> {
    return this.record('reorderCollection', [id, index], () => undefined);
  }

  private record<T>(command: string, args: readonly unknown[], answer: () => T): Promise<T> {
    this.calls.push({ command, args });
    return guard(this, answer);
  }
}
