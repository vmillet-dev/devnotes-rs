import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import {
  HttpCollection,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpPlace,
  HttpRequest,
  HttpRequestDraft,
  HttpRequestPatch,
  HttpTree,
  GraphqlAnswer,
  GraphqlParts,
  toGraphqlWire,
  HistoryDay,
  HistoryDraft,
  HistoryEntry,
  SentResponse,
  BodyAnswer,
  ContainerSettingsDraft,
  InheritedParts,
  KeyValueRow,
  RequestBodyDraft,
  fromSettings,
  toBodyWire,
  toInherited,
  toSettings,
  OpenedRequest,
  QuerySide,
  RequestDraft,
  RequestParts,
  toDocument,
  toParts,
  toSynced,
} from '@core/model/http.model';

function opened({ document, ...request }: HttpRequest): OpenedRequest {
  return { ...request, parts: toParts(document) };
}

/** The HTTP collections, which belong to the library: the tree is Rust's, built and ordered. */
@Injectable({ providedIn: 'root' })
export class HttpRepository {
  async tree(): Promise<HttpTree> {
    return unwrap('http_tree', await commands.httpTree());
  }

  async createCollection(name: string): Promise<HttpCollection> {
    return unwrap('create_http_collection', await commands.createHttpCollection(name));
  }

  async createFolder(collectionId: string, parentId: string | null, name: string): Promise<HttpFolder> {
    return unwrap('create_http_folder', await commands.createHttpFolder(collectionId, parentId, name));
  }

  async createRequest(draft: HttpRequestDraft): Promise<OpenedRequest> {
    return opened(unwrap('create_http_request', await commands.createHttpRequest(draft)));
  }

  async request(id: string): Promise<OpenedRequest> {
    return opened(unwrap('get_http_request', await commands.getHttpRequest(id)));
  }

  /** Writes the whole draft: a tab saves what it shows. */
  async saveRequest(id: string, draft: RequestDraft): Promise<OpenedRequest> {
    const patch: HttpRequestPatch = {
      name: draft.name,
      kind: draft.kind,
      method: draft.method,
      document: toDocument(draft.parts),
    };
    return opened(unwrap('save_http_request', await commands.saveHttpRequest(id, patch)));
  }

  async syncQuery(
    url: string,
    params: readonly KeyValueRow[],
    edited: QuerySide,
  ): Promise<{ readonly url: string; readonly params: readonly KeyValueRow[] }> {
    return toSynced(unwrap('sync_http_query', await commands.syncHttpQuery(url, [...params], edited)));
  }

  async settings(item: HttpItem): Promise<ContainerSettingsDraft> {
    return toSettings(unwrap('http_settings', await commands.httpSettings(item)));
  }

  async saveSettings(item: HttpItem, settings: ContainerSettingsDraft): Promise<void> {
    unwrap('save_http_settings', await commands.saveHttpSettings(item, fromSettings(settings)));
  }

  /** What a request placed there inherits, and from where. */
  async inherited(collectionId: string, folderId: string | null): Promise<InheritedParts> {
    return toInherited(
      unwrap('inherited_http_settings', await commands.inheritedHttpSettings(collectionId, folderId)),
    );
  }

  async describeBody(body: RequestBodyDraft): Promise<BodyAnswer> {
    return unwrap('describe_http_body', await commands.describeHttpBody(toBodyWire(body)));
  }

  /** `id` is the caller's: what `cancel` and `saveResponse` name the send by. */
  async send(
    id: string,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    requestId: string | null = null,
  ): Promise<SentResponse> {
    return unwrap(
      'send_http_request',
      await commands.sendHttpRequest({
        id,
        kind: draft.kind,
        method: draft.method,
        document: toDocument(draft.parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
        name: draft.name,
        requestId,
      }),
    );
  }

  /** What was sent, newest first, a group a local day. */
  async history(tzOffsetMinutes: number): Promise<HistoryDay[]> {
    return unwrap('http_history', await commands.httpHistory(tzOffsetMinutes));
  }

  async historyEntry(id: string): Promise<HistoryEntry> {
    return unwrap('http_history_entry', await commands.httpHistoryEntry(id));
  }

  async historyDraft(id: string): Promise<HistoryDraft> {
    return unwrap('http_history_draft', await commands.httpHistoryDraft(id));
  }

  async countHistory(): Promise<number> {
    return unwrap('count_http_history', await commands.countHttpHistory());
  }

  /** Answers how many entries went. */
  async clearHistory(): Promise<number> {
    return unwrap('clear_http_history', await commands.clearHttpHistory());
  }

  async cancel(id: string): Promise<boolean> {
    return unwrap('cancel_http_send', await commands.cancelHttpSend(id));
  }

  /** The whole body, not the text shown: `false` when Rust holds none for `id`. */
  async saveResponse(id: string, path: string): Promise<boolean> {
    return unwrap('save_http_response', await commands.saveHttpResponse(id, path));
  }

  /** The last answer to `id` as a `data:` URI, when it is an image. */
  async responseImage(id: string): Promise<string | null> {
    return unwrap('http_response_image', await commands.httpResponseImage(id));
  }

  async forgetResponse(id: string): Promise<void> {
    unwrap('forget_http_response', await commands.forgetHttpResponse(id));
  }

  /** Answers once the socket is open; what follows comes as events, under `id`. */
  async connectWebsocket(
    id: string,
    parts: RequestParts,
    place: { collectionId: string; folderId: string | null } | null,
  ): Promise<void> {
    unwrap(
      'connect_websocket',
      await commands.connectWebsocket({
        id,
        document: toDocument(parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
      }),
    );
  }

  /** `false` when no socket is open under `id`. */
  async sendWebsocket(id: string, text: string): Promise<boolean> {
    return unwrap('send_websocket', await commands.sendWebsocket(id, text));
  }

  async closeWebsocket(id: string): Promise<boolean> {
    return unwrap('close_websocket', await commands.closeWebsocket(id));
  }

  async describeGraphql(graphql: GraphqlParts): Promise<GraphqlAnswer> {
    return unwrap('describe_graphql', await commands.describeGraphql(toGraphqlWire(graphql)));
  }

  async rename(item: HttpItem, name: string): Promise<void> {
    unwrap('rename_http_item', await commands.renameHttpItem(item, name));
  }

  async contents(item: HttpItem): Promise<HttpContents> {
    return unwrap('count_http_contents', await commands.countHttpContents(item));
  }

  async delete(item: HttpItem): Promise<void> {
    unwrap('delete_http_item', await commands.deleteHttpItem(item));
  }

  async duplicate(item: HttpItem, name: string): Promise<HttpItem> {
    return unwrap('duplicate_http_item', await commands.duplicateHttpItem(item, name));
  }

  async move(item: HttpItem, place: HttpPlace): Promise<void> {
    unwrap('move_http_item', await commands.moveHttpItem(item, place));
  }

  async reorderCollection(id: string, index: number): Promise<void> {
    unwrap('reorder_http_collection', await commands.reorderHttpCollection(id, index));
  }
}
