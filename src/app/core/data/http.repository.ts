import { Injectable } from '@angular/core';
import { commands, RunData, RunRequest } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import {
  CookieDomain,
  HttpCollection,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpMethod,
  HttpPlace,
  HttpRequest,
  HttpRequestDraft,
  HttpRequestPatch,
  HttpTree,
  NewPlace,
  GraphqlAnswer,
  GraphqlParts,
  toGraphqlWire,
  HistoryDay,
  HistoryDraft,
  HistoryEntry,
  SentResponse,
  BodyAnswer,
  ContainerSettingsDraft,
  HeaderFault,
  ImpliedHeader,
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
  RequestHit,
  RequestParts,
  toDocument,
  toParts,
  toRows,
  toSynced,
} from '@core/model/http.model';
import { UsedVariable } from '@core/model/http-environment.model';
import { CodeOptions, CodeTarget, GeneratedCode } from '@core/model/http-code.model';

function opened({ document, ...request }: HttpRequest): OpenedRequest {
  return { ...request, parts: toParts(document) };
}

/** The HTTP collections, which belong to the library: the tree is Rust's, built and ordered. */
@Injectable({ providedIn: 'root' })
export class HttpRepository {
  async tree(): Promise<HttpTree> {
    return unwrap('http_tree', await commands.httpTree());
  }

  /** Ranked by Rust; nothing typed answers the requests edited last. */
  async search(search: string): Promise<readonly RequestHit[]> {
    return unwrap('search_http_requests', await commands.searchHttpRequests(search));
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

  /** With the collection or the folder it is saved into, created in the same write. */
  async createRequestIn(draft: HttpRequestDraft, create: NewPlace): Promise<OpenedRequest> {
    return opened(unwrap('create_http_request_in', await commands.createHttpRequestIn(draft, create)));
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

  /** With its method: a body on a `GET` is sent all the same, and Rust says it is unusual. */
  async describeBody(body: RequestBodyDraft, method: HttpMethod): Promise<BodyAnswer> {
    return unwrap('describe_http_body', await commands.describeHttpBody(toBodyWire(body), method));
  }

  /** What stops each header from being sent, by its rank: the rule is Rust's. */
  async checkHeaders(rows: readonly KeyValueRow[]): Promise<(HeaderFault | null)[]> {
    return unwrap('check_http_headers', await commands.checkHttpHeaders([...rows]));
  }

  /** `Nom: valeur` typed in a name, or lines pasted from a browser's network tab, as rows. */
  async readHeaders(text: string): Promise<KeyValueRow[]> {
    return toRows(unwrap('read_http_headers', await commands.readHttpHeaders(text)));
  }

  /** What it sends without a row of its own, and who put it there: composed as a send is. */
  async impliedHeaders(
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
  ): Promise<ImpliedHeader[]> {
    return unwrap(
      'implied_http_headers',
      await commands.impliedHttpHeaders(
        draft.kind,
        toDocument(draft.parts),
        place?.collectionId ?? null,
        place?.folderId ?? null,
      ),
    );
  }

  /** Each `{{name}}` the request holds, with what it resolves to and from where: Rust's. */
  async variables(
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    environmentId: string | null,
  ): Promise<UsedVariable[]> {
    return unwrap(
      'http_variables',
      await commands.httpVariables({
        kind: draft.kind,
        document: toDocument(draft.parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
        environmentId,
      }),
    );
  }

  /** The request written as code for `target`, resolved as a send would be: Rust's. */
  async generateCode(
    target: CodeTarget,
    options: CodeOptions,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    environmentId: string | null,
  ): Promise<GeneratedCode> {
    return unwrap(
      'generate_http_code',
      await commands.generateHttpCode({
        target,
        options,
        kind: draft.kind,
        method: draft.method,
        document: toDocument(draft.parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
        environmentId,
      }),
    );
  }

  /** `id` is the caller's: what `cancel` and `saveResponse` name the send by. */
  async send(
    id: string,
    draft: RequestDraft,
    place: { collectionId: string; folderId: string | null } | null,
    requestId: string | null = null,
    environmentId: string | null = null,
    confirmed = false,
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
        environmentId,
        confirmed,
      }),
    );
  }

  /** Answers once the run is over; its steps come as events, under `run.id`. */
  async runCollection(run: RunRequest): Promise<void> {
    unwrap('run_http_collection', await commands.runHttpCollection(run));
  }

  /** `false` when no run of that id was under way. */
  async cancelRun(id: string): Promise<boolean> {
    return unwrap('cancel_http_run', await commands.cancelHttpRun(id));
  }

  async readRunData(path: string): Promise<RunData> {
    return unwrap('read_http_run_data', await commands.readHttpRunData(path));
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

  /** A send put back onto the request it came from; `null` when that request is gone. */
  async historyOnto(id: string): Promise<HistoryDraft | null> {
    return unwrap('http_history_onto_request', await commands.httpHistoryOntoRequest(id));
  }

  /** One send gone, answered whole: what its undo hands back. */
  async deleteHistoryEntry(id: string): Promise<HistoryEntry> {
    return unwrap('delete_http_history_entry', await commands.deleteHttpHistoryEntry(id));
  }

  async restoreHistoryEntry(entry: HistoryEntry): Promise<void> {
    unwrap('restore_http_history_entry', await commands.restoreHttpHistoryEntry(entry));
  }

  async countHistory(): Promise<number> {
    return unwrap('count_http_history', await commands.countHttpHistory());
  }

  async cookies(): Promise<CookieDomain[]> {
    return unwrap('http_cookies', await commands.httpCookies());
  }

  /** Each answers how many cookies went. */
  async deleteCookie(id: string): Promise<number> {
    return unwrap('delete_http_cookie', await commands.deleteHttpCookie(id));
  }

  async deleteCookieDomain(domain: string): Promise<number> {
    return unwrap('delete_http_cookie_domain', await commands.deleteHttpCookieDomain(domain));
  }

  async countCookies(): Promise<number> {
    return unwrap('count_http_cookies', await commands.countHttpCookies());
  }

  async clearCookies(): Promise<number> {
    return unwrap('clear_http_cookies', await commands.clearHttpCookies());
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
    environmentId: string | null = null,
  ): Promise<void> {
    unwrap(
      'connect_websocket',
      await commands.connectWebsocket({
        id,
        document: toDocument(parts),
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
        environmentId,
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

  /** The variables with a `null` for each one the operation declares and they lack. */
  async addGraphqlVariables(graphql: GraphqlParts): Promise<string> {
    return unwrap('add_graphql_variables', await commands.addGraphqlVariables(toGraphqlWire(graphql)));
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
