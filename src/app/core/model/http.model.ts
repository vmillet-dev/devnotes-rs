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
  RequestDocument,
  RequestKind,
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
  RequestDocument,
  RequestKind,
};

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
