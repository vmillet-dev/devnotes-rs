import type {
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
  SyncedQuery,
} from '@core/ipc/bindings';

export type {
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
};

/** A row of parameters or headers, every field present: the wire leaves out a default. */
export interface KeyValueRow {
  readonly enabled: boolean;
  readonly key: string;
  readonly value: string;
  readonly description: string;
}

/** A request's document as the editor holds it, every part present. */
export interface RequestParts {
  readonly url: string;
  readonly params: readonly KeyValueRow[];
  readonly headers: readonly KeyValueRow[];
  readonly description: string;
}

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

/** The wire leaves out what is at its default (`#[serde(default)]`): the editor needs it all. */
export function toParts(document: RequestDocument): RequestParts {
  return {
    url: document.url ?? '',
    params: (document.params ?? []).map(toRow),
    headers: (document.headers ?? []).map(toRow),
    description: document.description ?? '',
  };
}

export function toDocument(parts: RequestParts): RequestDocument {
  return {
    url: parts.url,
    params: [...parts.params],
    headers: [...parts.headers],
    description: parts.description,
  };
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
