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
  HeaderFault,
  HttpOrigin,
  ImpliedHeader,
  Inherited,
  InheritedHeader,
  KeyPlace,
  RequestAuth,
  RequestBody,
  RequestScripts,
  ScriptStop,
  FailureKind,
  ScriptLevel,
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
  NewPlace,
  QuerySide,
  RequestDocument,
  RequestHit,
  RequestKind,
  SentResponse,
  SyncedQuery,
  CookieDomain,
  JarCookie,
  StoredCookie,
  Transport,
  TransportSettings,
} from '@core/ipc/bindings';
import { VariableRow, toVariableRow } from './http-environment.model';

export { DEFAULT_TRANSPORT } from '@core/ipc/bindings';

export type {
  BodyAnswer,
  HeaderFault,
  ImpliedHeader,
  CookieDomain,
  JarCookie,
  StoredCookie,
  Transport,
  TransportSettings,
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
  NewPlace,
  QuerySide,
  RequestDocument,
  RequestHit,
  RequestKind,
  SentResponse,
  ScriptStop,
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

/** What each kind of body held when another was chosen. */
export type BodiesLeft = Readonly<Partial<Record<BodyKind, RequestBodyDraft>>>;

/**
 * Sets a body aside under its kind when another is chosen. JSON or plain, a text is one text:
 * it is kept under both, so it travels between the two as it always did.
 */
export function setBodyAside(left: BodiesLeft, body: RequestBodyDraft): BodiesLeft {
  return body.kind === 'json' || body.kind === 'text'
    ? { ...left, json: { kind: 'json', text: body.text }, text: { kind: 'text', text: body.text } }
    : { ...left, [body.kind]: body };
}

/** The body a kind starts with: what it held when it was left, or nothing yet. */
export function bodyOfKind(kind: BodyKind, left: BodiesLeft = {}): RequestBodyDraft {
  const kept = left[kind];
  if (kept !== undefined) return kept;
  switch (kind) {
    case 'json':
    case 'text':
      return { kind, text: '' };
    case 'form':
      return { kind, fields: [] };
    case 'multipart':
      return { kind, parts: [] };
    case 'binary':
      return { kind, path: '' };
    case 'none':
      return { kind };
  }
}

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

/** The JavaScript run around a send, both present: blank runs nothing. */
export interface ScriptParts {
  readonly before: string;
  readonly after: string;
}

export type ScriptPhase = keyof ScriptParts;
export const SCRIPT_PHASES: readonly ScriptPhase[] = ['before', 'after'];

export const NO_SCRIPTS: ScriptParts = { before: '', after: '' };

/** Where « Ouvrir le script » leads: one of the two scripts, on the line that stopped. */
export interface ScriptFocus {
  readonly phase: ScriptPhase;
  /** From 1; `null` when the failure named none. */
  readonly line: number | null;
}

const FAILURE_KINDS: readonly FailureKind[] = ['error', 'syntax', 'timeout', 'memory', 'host'];
const SCRIPT_LEVELS: readonly ScriptLevel[] = ['request', 'folder', 'collection'];

/** A script before the send that stopped it, read back from the values its error carries. */
export function stopOf(params: Readonly<Record<string, string | undefined>>): ScriptStop {
  const kind = FAILURE_KINDS.find((known) => known === params['failure']) ?? 'error';
  const level = SCRIPT_LEVELS.find((known) => known === params['level']) ?? null;
  const line = Number.parseInt(params['line'] ?? '', 10);
  return {
    kind,
    level,
    script: 'before',
    name: params['name'] ?? null,
    owner: params['owner'] ?? null,
    line: Number.isNaN(line) ? null : line,
    message: params['message'] ?? '',
  };
}

/** A folder's or the collection's scripts, and whose they are. */
export interface InheritedScriptsRow {
  readonly from: HttpOrigin;
  readonly scripts: ScriptParts;
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
  readonly transport: TransportSettings;
  /** The nearest level a `{{name}}` resolves from. */
  readonly variables: readonly VariableRow[];
  readonly scripts: ScriptParts;
}

/** A collection's or a folder's own auth, headers and variables, what its requests inherit. */
export interface ContainerSettingsDraft {
  readonly auth: RequestAuth;
  readonly headers: readonly KeyValueRow[];
  readonly transport: TransportSettings;
  readonly variables: readonly VariableRow[];
  readonly scripts: ScriptParts;
}

export interface InheritedHeaderRow {
  readonly header: KeyValueRow;
  readonly from: HttpOrigin;
}

export interface InheritedParts {
  readonly auth: RequestAuth;
  readonly authFrom: HttpOrigin | null;
  readonly headers: readonly InheritedHeaderRow[];
  readonly transport: Transport;
  /** The collection's, then each folder's down to the request's. */
  readonly scripts: readonly InheritedScriptsRow[];
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
  transport: {},
  variables: [],
  scripts: NO_SCRIPTS,
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
    transport: document.transport ?? {},
    variables: (document.variables ?? []).map(toVariableRow),
    scripts: toScripts(document.scripts),
  };
}

export function toScripts(scripts: RequestScripts | undefined): ScriptParts {
  return { before: scripts?.before ?? '', after: scripts?.after ?? '' };
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
    transport: { ...parts.transport },
    variables: [...parts.variables],
    scripts: { ...parts.scripts },
  };
}

export function toSettings(settings: ContainerSettings): ContainerSettingsDraft {
  return {
    auth: settings.auth ?? { kind: 'inherit' },
    headers: (settings.headers ?? []).map(toRow),
    transport: settings.transport ?? {},
    variables: (settings.variables ?? []).map(toVariableRow),
    scripts: toScripts(settings.scripts),
  };
}

export function fromSettings(settings: ContainerSettingsDraft): ContainerSettings {
  return {
    auth: settings.auth,
    headers: [...settings.headers],
    transport: { ...settings.transport },
    variables: [...settings.variables],
    scripts: { ...settings.scripts },
  };
}

export function toInherited(inherited: Inherited): InheritedParts {
  return {
    auth: inherited.auth,
    authFrom: inherited.authFrom,
    headers: inherited.headers.map((known: InheritedHeader) => ({
      header: toRow(known.header),
      from: known.from,
    })),
    transport: inherited.transport,
    scripts: inherited.scripts.map((known) => ({ from: known.from, scripts: toScripts(known.scripts) })),
  };
}

export function toBodyWire(body: RequestBodyDraft): RequestBody {
  return fromBody(body);
}

export function toRows(rows: readonly KeyValue[]): KeyValueRow[] {
  return rows.map(toRow);
}

export function toSynced(answer: SyncedQuery): {
  readonly url: string;
  readonly params: readonly KeyValueRow[];
} {
  return { url: answer.url, params: answer.params.map(toRow) };
}

const SHORT_BADGES: Readonly<Record<string, string>> = { DELETE: 'DEL', OPTIONS: 'OPT' };

/** A long method in three letters, where room is counted: the rail at its floor, a tab. */
export function shortBadge(badge: string): string {
  return SHORT_BADGES[badge] ?? badge;
}

/** The three a request is created as, in the order they are offered: HTTP first. */
export const REQUEST_KINDS: readonly RequestKind[] = ['http', 'graphql', 'websocket'];

/** What the rail writes before a request's name: its method, or the kind's own word. */
export function requestBadge(request: Pick<HttpRequestSummary, 'kind' | 'method'>): string {
  switch (request.kind) {
    case 'graphql':
      return 'GQL';
    case 'websocket':
      return 'WS';
    case 'http':
      return request.method;
  }
}
