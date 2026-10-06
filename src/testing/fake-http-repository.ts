import { guard } from './fail-next';
import { HttpRepository } from '@core/data/http.repository';
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

export interface HttpCall {
  readonly command: string;
  readonly args: readonly unknown[];
}

/**
 * Answers what a spec set and records what it was asked: the tree is Rust's to build, so a spec
 * sets `tree` to what Rust would answer after a write rather than this fake rebuilding it.
 */
export class FakeHttpRepository implements Pick<HttpRepository, keyof HttpRepository> {
  tree$: HttpTree = { collections: [] };
  contentsAnswer: HttpContents = { folders: 0, requests: 0 };
  readonly requests = new Map<string, HttpRequest>();
  readonly calls: HttpCall[] = [];
  failNext: Error | null = null;
  private nextId = 0;

  /** The calls of one command, their arguments in order. */
  callsOf(command: string): (readonly unknown[])[] {
    return this.calls.filter((call) => call.command === command).map((call) => call.args);
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

  createRequest(draft: HttpRequestDraft): Promise<HttpRequest> {
    return this.record('createRequest', [draft], () => {
      const request: HttpRequest = {
        id: `request-${++this.nextId}`,
        collectionId: draft.collectionId,
        folderId: draft.folderId,
        name: draft.name,
        kind: draft.kind,
        method: draft.method,
        document: draft.document,
        createdAt: '2026-10-07T00:00:00.000Z',
        updatedAt: '2026-10-07T00:00:00.000Z',
      };
      this.requests.set(request.id, request);
      return request;
    });
  }

  request(id: string): Promise<HttpRequest> {
    return this.record('request', [id], () => {
      const request = this.requests.get(id);
      if (!request) throw new Error(`no request ${id}`);
      return request;
    });
  }

  saveRequest(id: string, patch: HttpRequestPatch): Promise<HttpRequest> {
    return this.record('saveRequest', [id, patch], () => {
      const request = this.requests.get(id);
      if (!request) throw new Error(`no request ${id}`);
      const saved = { ...request, ...patch } as HttpRequest;
      this.requests.set(id, saved);
      return saved;
    });
  }

  rename(item: HttpItem, name: string): Promise<void> {
    return this.record('rename', [item, name], () => undefined);
  }

  contents(item: HttpItem): Promise<HttpContents> {
    return this.record('contents', [item], () => this.contentsAnswer);
  }

  delete(item: HttpItem): Promise<void> {
    return this.record('delete', [item], () => undefined);
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
