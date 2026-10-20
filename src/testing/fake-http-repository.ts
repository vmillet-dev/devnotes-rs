import { guard } from './fail-next';
import { HttpRepository } from '@core/data/http.repository';
import {
  HttpCollection,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpMethod,
  HttpPlace,
  HttpRequestDraft,
  HttpTree,
  RequestHit,
  NewPlace,
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
  NO_SCRIPTS,
  EMPTY_PARTS,
  HeaderFault,
  ImpliedHeader,
  InheritedParts,
  KeyValueRow,
  RequestBodyDraft,
  OpenedRequest,
  QuerySide,
  RequestDraft,
  RequestParts,
  toParts,
} from '@core/model/http.model';
import { UsedVariable } from '@core/model/http-environment.model';
import { RunData, RunRequest } from '@core/model/http-runner.model';
import { CodeOptions, CodeTarget, GeneratedCode } from '@core/model/http-code.model';

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
  /** What Rust ranked, as a spec sets it: the double does not rank. */
  hits: readonly RequestHit[] = [];
  contentsAnswer: HttpContents = { folders: 0, requests: 0 };
  settingsAnswer: ContainerSettingsDraft = {
    auth: { kind: 'inherit' },
    headers: [],
    transport: {},
    variables: [],
    scripts: NO_SCRIPTS,
  };
  inheritedAnswer: InheritedParts = {
    auth: { kind: 'none' },
    authFrom: null,
    headers: [],
    transport: DEFAULT_TRANSPORT,
    scripts: [],
  };
  /** The jar by domain; the deletes take from it as Rust would. */
  jar: CookieDomain[] = [];
  bodyAnswer: BodyAnswer = { contentType: null, problem: null, unusual: false };
  /** What `generateCode` answers, the target written before it so a spec can tell which. */
  codeAnswer: GeneratedCode = { code: 'GET', language: 'sh', secrets: [], unresolved: [], notice: null };
  /** What `variables` answers: Rust resolves, the spec says what it would. */
  usedAnswer: UsedVariable[] = [];
  /** The environment each send and each connection named, in order. */
  readonly sentWith: (string | null)[] = [];
  /** Whether each send carried the yes a protected environment asks for. */
  readonly confirmedSends: boolean[] = [];
  /** What `checkHeaders` answers, by rank; and the rows `readHeaders` reads from a text. */
  headerFaults: (HeaderFault | null)[] = [];
  headersRead: KeyValueRow[] = [];
  /** What a request sends unasked; a spec holding `pendingImplied` decides when it is answered. */
  impliedAnswer: ImpliedHeader[] = [];
  pendingImplied: Promise<ImpliedHeader[]> | null = null;
  graphqlAnswer: GraphqlAnswer = {
    operations: [],
    variablesProblem: null,
    queryProblem: null,
    missing: [],
    unknown: [],
  };
  /** What « Ajouter » answers: the variables as Rust would write them. */
  variablesAdded = '';
  /** What `send` answers; a spec holding `pendingSend` decides when. */
  response: SentResponse = sentResponse();
  pendingSend: Promise<SentResponse> | null = null;
  saveAnswer = true;
  imageAnswer: string | null = null;
  historyAnswer: HistoryDay[] = [];
  /** By entry id; a missing one rejects, as Rust answers a purged entry. */
  readonly entries = new Map<string, HistoryEntry>();
  draftAnswer: HistoryDraft | null = null;
  /** What a send put back onto its request reads as; `null` for a request that is gone. */
  ontoAnswer: HistoryDraft | null = null;
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

  search(search: string): Promise<readonly RequestHit[]> {
    return this.record('search', [search], () => this.hits);
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

  createRequestIn(draft: HttpRequestDraft, create: NewPlace): Promise<OpenedRequest> {
    return this.record('createRequestIn', [draft, create], () => {
      const collectionId = create.collection ? `collection-${++this.nextId}` : draft.collectionId;
      const folderId = create.folder ? `folder-${++this.nextId}` : create.collection ? null : draft.folderId;
      return this.seed(
        `request-${++this.nextId}`,
        { name: draft.name, kind: draft.kind, method: draft.method, parts: toParts(draft.document) },
        collectionId,
        folderId,
      );
    });
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

  describeBody(body: RequestBodyDraft, method: HttpMethod): Promise<BodyAnswer> {
    return this.record('describeBody', [body, method], () => this.bodyAnswer);
  }

  checkHeaders(rows: readonly KeyValueRow[]): Promise<(HeaderFault | null)[]> {
    return this.record('checkHeaders', [rows], () => this.headerFaults);
  }

  readHeaders(text: string): Promise<KeyValueRow[]> {
    return this.record('readHeaders', [text], () => this.headersRead);
  }

  async impliedHeaders(
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
  ): Promise<ImpliedHeader[]> {
    const answer = await this.record('impliedHeaders', [draft, place], () => this.impliedAnswer);
    return this.pendingImplied ?? answer;
  }

  generateCode(
    target: CodeTarget,
    options: CodeOptions,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    environmentId: string | null,
  ): Promise<GeneratedCode> {
    return this.record('generateCode', [target, options, draft, place, environmentId], () => ({
      ...this.codeAnswer,
      code: `${target} ${this.codeAnswer.code}`,
    }));
  }

  variables(
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    environmentId: string | null,
  ): Promise<UsedVariable[]> {
    return this.record('variables', [draft, place, environmentId], () => this.usedAnswer);
  }

  send(
    id: string,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    requestId: string | null = null,
    environmentId: string | null = null,
    confirmed = false,
  ): Promise<SentResponse> {
    this.sentWith.push(environmentId);
    this.confirmedSends.push(confirmed);
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

  historyOnto(id: string): Promise<HistoryDraft | null> {
    return this.record('historyOnto', [id], () => this.ontoAnswer);
  }

  /** Takes the entry out of the days, as Rust would; a missing one rejects. */
  deleteHistoryEntry(id: string): Promise<HistoryEntry> {
    return this.record('deleteHistoryEntry', [id], () => {
      const entry = this.entries.get(id);
      if (!entry) throw new Error(`no history entry ${id}`);
      this.historyAnswer = this.historyAnswer
        .map((day) => ({ ...day, items: day.items.filter((item) => item.id !== id) }))
        .filter((day) => day.items.length > 0);
      return entry;
    });
  }

  restoreHistoryEntry(entry: HistoryEntry): Promise<void> {
    return this.record('restoreHistoryEntry', [entry], () => undefined);
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

  /** What a run answers once over; its steps are the spec's to play. */
  pendingRun: Promise<void> | null = null;
  runData: RunData = { columns: [], rows: [] };

  runCollection(run: RunRequest): Promise<void> {
    const pending = this.pendingRun;
    return this.record('runCollection', [run], () => undefined).then(() => pending ?? undefined);
  }

  cancelRun(id: string): Promise<boolean> {
    return this.record('cancelRun', [id], () => true);
  }

  readRunData(path: string): Promise<RunData> {
    return this.record('readRunData', [path], () => this.runData);
  }

  forgetResponse(id: string): Promise<void> {
    return this.record('forgetResponse', [id], () => undefined);
  }

  connectWebsocket(
    id: string,
    parts: RequestParts,
    place: { collectionId: string; folderId: string | null } | null,
    environmentId: string | null = null,
  ): Promise<void> {
    this.sentWith.push(environmentId);
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

  addGraphqlVariables(graphql: GraphqlParts): Promise<string> {
    return this.record('addGraphqlVariables', [graphql], () => this.variablesAdded);
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
