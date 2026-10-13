import { Injectable, signal } from '@angular/core';

/** What the HTTP workspace shows: the open requests, or one of the pages from the rail's foot. */
export type HttpWorkspacePage = 'requests' | 'history' | 'environments';

/** One page at a time: opening the history leaves the environments, and a request leaves both. */
@Injectable({ providedIn: 'root' })
export class HttpWorkspaceStore {
  private readonly _page = signal<HttpWorkspacePage>('requests');
  readonly page = this._page.asReadonly();

  show(page: HttpWorkspacePage): void {
    this._page.set(page);
  }

  /** From the rail's foot: a page shown is put away by the same button. */
  toggle(page: HttpWorkspacePage): void {
    this._page.update((current) => (current === page ? 'requests' : page));
  }

  /** Put away `page` if it is the one shown: a request opened from the tree leaves it. */
  leave(page: HttpWorkspacePage): void {
    if (this._page() === page) this._page.set('requests');
  }
}
