import type {
  BodyAnswer,
  ContainerSettings,
  GraphqlAnswer,
  GraphqlDocument,
  GraphqlResult,
  SocketEvent,
  WebsocketDocument,
  WebsocketEvent,
  HistoryDay,
  HistoryDraft,
  HistoryEntry,
  HistoryItem,
  FormPart,
  HttpOrigin,
  Inherited,
  InheritedHeader,
  KeyPlace,
  RequestAuth,
  RequestBody,
  HttpCollection,
  HttpCollectionNode,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpItemKind,
  HttpMethod,
  HttpNode,
  HttpPlace,
  HttpRequest,
  HttpRequestDraft,
  HttpRequestPatch,
  HttpRequestSummary,
  HttpTree,
  KeyValue,
  QuerySide,
  RequestDocument,
  RequestKind,
  SentResponse,
  SyncedQuery,
} from '@core/ipc/bindings';

export type {
  BodyAnswer,
  GraphqlAnswer,
  GraphqlResult,
  SocketEvent,
  WebsocketEvent,
  HistoryDay,
  HistoryDraft,
  HistoryEntry,
  HistoryItem,
  HttpOrigin,
  KeyPlace,
  RequestAuth,
  HttpCollection,
  HttpCollectionNode,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpItemKind,
  HttpMethod,
  HttpNode,
  HttpPlace,
  HttpRequest,
  HttpRequestDraft,
  HttpRequestPatch,
  HttpRequestSummary,
  HttpTree,
  QuerySide,
  RequestDocument,
  RequestKind,
  SentResponse,
};

/** A row of parameters or headers, every field present: the wire leaves out a default. */
export interface KeyValueRow {
  readonly enabled: boolean;
  readonly key: string;
  readonly value: string;
  readonly description: string;
}

/** A multipart field: its text, or the path of a file read when the request is sent. */
export interface FormPartRow extends KeyValueRow {
  readonly file: boolean;
}

export type RequestBodyDraft =
  | { readonly kind: 'none' }
  | { readonly kind: 'json'; readonly text: string }
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'form'; readonly fields: readonly KeyValueRow[] }
  | { readonly kind: 'multipart'; readonly parts: readonly FormPartRow[] }
  | { readonly kind: 'binary'; readonly path: string };

export type BodyKind = RequestBodyDraft['kind'];
export const BODY_KINDS: readonly BodyKind[] = ['none', 'json', 'text', 'form', 'multipart', 'binary'];
export type AuthKind = RequestAuth['kind'];
export const AUTH_KINDS: readonly AuthKind[] = ['inherit', 'none', 'basic', 'bearer', 'apiKey'];

/** A GraphQL request's own part, sent in place of the body. */
export interface GraphqlParts {
  readonly query: string;
  /** A JSON object as typed; empty sends none. */
  readonly variables: string;
  readonly operationName: string | null;
  readonly asGet: boolean;
}

/** A message kept to send again, both fields present. */
export interface SavedMessageRow {
  readonly name: string;
  readonly text: string;
}

/** A WebSocket's own part: the subprotocols it asks for, and the messages kept to send again. */
export interface WebsocketParts {
  readonly protocols: readonly string[];
  readonly messages: readonly SavedMessageRow[];
}

/** A request's document as the editor holds it, every part present. */
export interface RequestParts {
  readonly url: string;
  readonly params: readonly KeyValueRow[];
  readonly headers: readonly KeyValueRow[];
  readonly body: RequestBodyDraft;
  readonly auth: RequestAuth;
  readonly description: string;
  readonly graphql: GraphqlParts;
  readonly websocket: WebsocketParts;
}

/** A collection's or a folder's own auth and headers, what its requests inherit. */
export interface ContainerSettingsDraft {
  readonly auth: RequestAuth;
  readonly headers: readonly KeyValueRow[];
}

export interface InheritedHeaderRow {
  readonly header: KeyValueRow;
  readonly from: HttpOrigin;
}

export interface InheritedParts {
  readonly auth: RequestAuth;
  readonly authFrom: HttpOrigin | null;
  readonly headers: readonly InheritedHeaderRow[];
}

export const EMPTY_PARTS: RequestParts = {
  url: '',
  params: [],
  headers: [],
  body: { kind: 'none' },
  auth: { kind: 'inherit' },
  description: '',
  graphql: { query: '', variables: '', operationName: null, asGet: false },
  websocket: { protocols: [], messages: [] },
};

/** What a request tab edits, and what saving writes. */
export interface RequestDraft {
  readonly name: string;
  readonly kind: RequestKind;
  readonly method: HttpMethod;
  readonly parts: RequestParts;
}

/** A request read back, its document filled. */
export type OpenedRequest = Omit<HttpRequest, 'document'> & { readonly parts: RequestParts };

export const HTTP_METHODS: readonly HttpMethod[] = [
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
];

function toRow(row: KeyValue): KeyValueRow {
  return {
    enabled: row.enabled ?? true,
    key: row.key ?? '',
    value: row.value ?? '',
    description: row.description ?? '',
  };
}

function toPart(part: FormPart): FormPartRow {
  return { ...toRow(part), file: part.file ?? false };
}

function toBody(body: RequestBody | undefined): RequestBodyDraft {
  if (body === undefined) return { kind: 'none' };
  switch (body.kind) {
    case 'none':
      return { kind: 'none' };
    case 'form':
      return { kind: 'form', fields: body.fields.map(toRow) };
    case 'multipart':
      return { kind: 'multipart', parts: body.parts.map(toPart) };
    case 'json':
    case 'text':
    case 'binary':
      return body;
  }
}

function fromBody(body: RequestBodyDraft): RequestBody {
  switch (body.kind) {
    case 'form':
      return { kind: 'form', fields: [...body.fields] };
    case 'multipart':
      return { kind: 'multipart', parts: [...body.parts] };
    case 'none':
    case 'json':
    case 'text':
    case 'binary':
      return body;
  }
}

/** The wire leaves out what is at its default (`#[serde(default)]`): the editor needs it all. */
export function toParts(document: RequestDocument): RequestParts {
  return {
    url: document.url ?? '',
    params: (document.params ?? []).map(toRow),
    headers: (document.headers ?? []).map(toRow),
    body: toBody(document.body),
    auth: document.auth ?? { kind: 'inherit' },
    description: document.description ?? '',
    graphql: toGraphql(document.graphql),
    websocket: toWebsocket(document.websocket),
  };
}

function toWebsocket(websocket: WebsocketDocument | undefined): WebsocketParts {
  return {
    protocols: websocket?.protocols ?? [],
    messages: (websocket?.messages ?? []).map((message) => ({
      name: message.name ?? '',
      text: message.text ?? '',
    })),
  };
}

function toGraphql(graphql: GraphqlDocument | undefined): GraphqlParts {
  return {
    query: graphql?.query ?? '',
    variables: graphql?.variables ?? '',
    operationName: graphql?.operationName ?? null,
    asGet: graphql?.asGet ?? false,
  };
}

export function toGraphqlWire(graphql: GraphqlParts): GraphqlDocument {
  return { ...graphql };
}

export function toDocument(parts: RequestParts): RequestDocument {
  return {
    url: parts.url,
    params: [...parts.params],
    headers: [...parts.headers],
    body: fromBody(parts.body),
    auth: parts.auth,
    description: parts.description,
    graphql: toGraphqlWire(parts.graphql),
    websocket: { protocols: [...parts.websocket.protocols], messages: [...parts.websocket.messages] },
  };
}

export function toSettings(settings: ContainerSettings): ContainerSettingsDraft {
  return { auth: settings.auth ?? { kind: 'inherit' }, headers: (settings.headers ?? []).map(toRow) };
}

export function fromSettings(settings: ContainerSettingsDraft): ContainerSettings {
  return { auth: settings.auth, headers: [...settings.headers] };
}

export function toInherited(inherited: Inherited): InheritedParts {
  return {
    auth: inherited.auth,
    authFrom: inherited.authFrom,
    headers: inherited.headers.map((known: InheritedHeader) => ({
      header: toRow(known.header),
      from: known.from,
    })),
  };
}

export function toBodyWire(body: RequestBodyDraft): RequestBody {
  return fromBody(body);
}

export function toSynced(answer: SyncedQuery): {
  readonly url: string;
  readonly params: readonly KeyValueRow[];
} {
  return { url: answer.url, params: answer.params.map(toRow) };
}

/** What the rail writes before a request's name: its method, or the kind's own word. */
export function requestBadge(request: Pick<HttpRequestSummary, 'kind' | 'method'>): string {
  switch (request.kind) {
    case 'graphql':
      return 'QUERY';
    case 'websocket':
      return 'WS';
    case 'http':
      return request.method;
  }
}
