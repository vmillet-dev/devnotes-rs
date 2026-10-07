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
  GraphqlAnswer,
  GraphqlParts,
  HistoryDay,
  HistoryDraft,
  HistoryEntry,
  SentResponse,
  BodyAnswer,
  ContainerSettingsDraft,
  CookieDomain,
  DEFAULT_TRANSPORT,
  EMPTY_PARTS,
  InheritedParts,
  KeyValueRow,
  RequestBodyDraft,
  OpenedRequest,
  QuerySide,
  RequestDraft,
  RequestParts,
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
  settingsAnswer: ContainerSettingsDraft = { auth: { kind: 'inherit' }, headers: [], transport: {} };
  inheritedAnswer: InheritedParts = {
    auth: { kind: 'none' },
    authFrom: null,
    headers: [],
    transport: DEFAULT_TRANSPORT,
  };
  /** The jar by domain; the deletes take from it as Rust would. */
  jar: CookieDomain[] = [];
  bodyAnswer: BodyAnswer = { contentType: null, problem: null };
  graphqlAnswer: GraphqlAnswer = { operations: [], variablesProblem: null };
  /** What `send` answers; a spec holding `pendingSend` decides when. */
  response: SentResponse = sentResponse();
  pendingSend: Promise<SentResponse> | null = null;
  saveAnswer = true;
  imageAnswer: string | null = null;
  historyAnswer: HistoryDay[] = [];
  /** By entry id; a missing one rejects, as Rust answers a purged entry. */
  readonly entries = new Map<string, HistoryEntry>();
  draftAnswer: HistoryDraft | null = null;
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
      const saved: OpenedRequest = {
        ...request,
        name: draft.name,
        kind: draft.kind,
        method: draft.method,
        parts: draft.parts,
      };
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

  send(
    id: string,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    requestId: string | null = null,
  ): Promise<SentResponse> {
    const pending = this.pendingSend;
    this.pendingSend = null;
    return this.record('send', [id, draft, place, requestId], () => this.response).then(
      (answer) => pending ?? answer,
    );
  }

  history(tzOffsetMinutes: number): Promise<HistoryDay[]> {
    return this.record('history', [tzOffsetMinutes], () => this.historyAnswer);
  }

  historyEntry(id: string): Promise<HistoryEntry> {
    return this.record('historyEntry', [id], () => {
      const entry = this.entries.get(id);
      if (!entry) throw new Error(`no entry ${id}`);
      return entry;
    });
  }

  historyDraft(id: string): Promise<HistoryDraft> {
    return this.record('historyDraft', [id], () => {
      if (!this.draftAnswer) throw new Error(`no draft ${id}`);
      return this.draftAnswer;
    });
  }

  countHistory(): Promise<number> {
    return this.record('countHistory', [], () =>
      this.historyAnswer.reduce((count, day) => count + day.items.length, 0),
    );
  }

  cookies(): Promise<CookieDomain[]> {
    return this.record('cookies', [], () => this.jar);
  }

  deleteCookie(id: string): Promise<number> {
    return this.record('deleteCookie', [id], () => {
      const before = this.countJar();
      this.jar = this.jar
        .map((domain) => ({ ...domain, cookies: domain.cookies.filter((stored) => stored.id !== id) }))
        .filter((domain) => domain.cookies.length > 0);
      return before - this.countJar();
    });
  }

  deleteCookieDomain(domain: string): Promise<number> {
    return this.record('deleteCookieDomain', [domain], () => {
      const before = this.countJar();
      this.jar = this.jar.filter((known) => known.domain !== domain);
      return before - this.countJar();
    });
  }

  countCookies(): Promise<number> {
    return this.record('countCookies', [], () => this.countJar());
  }

  clearCookies(): Promise<number> {
    return this.record('clearCookies', [], () => {
      const count = this.countJar();
      this.jar = [];
      return count;
    });
  }

  private countJar(): number {
    return this.jar.reduce((count, domain) => count + domain.cookies.length, 0);
  }

  clearHistory(): Promise<number> {
    return this.record('clearHistory', [], () => {
      const count = this.historyAnswer.reduce((sum, day) => sum + day.items.length, 0);
      this.historyAnswer = [];
      return count;
    });
  }

  cancel(id: string): Promise<boolean> {
    return this.record('cancel', [id], () => true);
  }

  saveResponse(id: string, path: string): Promise<boolean> {
    return this.record('saveResponse', [id, path], () => this.saveAnswer);
  }

  responseImage(id: string): Promise<string | null> {
    return this.record('responseImage', [id], () => this.imageAnswer);
  }

  forgetResponse(id: string): Promise<void> {
    return this.record('forgetResponse', [id], () => undefined);
  }

  connectWebsocket(
    id: string,
    parts: RequestParts,
    place: { collectionId: string; folderId: string | null } | null,
  ): Promise<void> {
    return this.record('connectWebsocket', [id, parts, place], () => undefined);
  }

  sendWebsocket(id: string, text: string): Promise<boolean> {
    return this.record('sendWebsocket', [id, text], () => true);
  }

  closeWebsocket(id: string): Promise<boolean> {
    return this.record('closeWebsocket', [id], () => true);
  }

  describeGraphql(graphql: GraphqlParts): Promise<GraphqlAnswer> {
    return this.record('describeGraphql', [graphql], () => this.graphqlAnswer);
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

/** An answer as Rust shapes it, `200 OK` with a small JSON body unless told otherwise. */
export function sentResponse(overrides: Partial<SentResponse> = {}): SentResponse {
  return {
    status: 200,
    reason: 'OK',
    millis: 142,
    size: 3277,
    url: 'https://api.exemple.fr/users',
    headers: [{ enabled: true, key: 'content-type', value: 'application/json', description: '' }],
    body: '{"data":[]}',
    binary: false,
    cut: false,
    incomplete: false,
    redirects: [],
    graphql: null,
    language: 'json',
    pretty: '{\n  "data": []\n}',
    cookies: [],
    exchange: {
      method: 'GET',
      url: 'https://api.exemple.fr/users',
      headers: [{ enabled: true, key: 'Accept', value: 'application/json', description: '' }],
      body: { kind: 'none' },
    },
    ...overrides,
  };
}
