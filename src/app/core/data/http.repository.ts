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
} from '@core/model/http.model';

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

  async createRequest(draft: HttpRequestDraft): Promise<HttpRequest> {
    return unwrap('create_http_request', await commands.createHttpRequest(draft));
  }

  async request(id: string): Promise<HttpRequest> {
    return unwrap('get_http_request', await commands.getHttpRequest(id));
  }

  async saveRequest(id: string, patch: HttpRequestPatch): Promise<HttpRequest> {
    return unwrap('save_http_request', await commands.saveHttpRequest(id, patch));
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
