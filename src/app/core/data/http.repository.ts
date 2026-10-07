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
  ): Promise<SentResponse> {
    return unwrap(
      'send_http_request',
      await commands.sendHttpRequest({
        id,
        method: draft.method,
        document: toDocument(draft.parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
      }),
    );
  }

  async cancel(id: string): Promise<boolean> {
    return unwrap('cancel_http_send', await commands.cancelHttpSend(id));
  }

  /** The whole body, not the text shown: `false` when Rust holds none for `id`. */
  async saveResponse(id: string, path: string): Promise<boolean> {
    return unwrap('save_http_response', await commands.saveHttpResponse(id, path));
  }

  async forgetResponse(id: string): Promise<void> {
    unwrap('forget_http_response', await commands.forgetHttpResponse(id));
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
