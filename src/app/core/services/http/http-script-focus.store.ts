import { Injectable, inject, signal } from '@angular/core';
import { ScriptFocus, ScriptStop } from '@core/model/http.model';
import { HttpCollectionsStore } from './http-collections.store';

/** A tab asked to show one of its own scripts. */
export interface RequestScriptFocus {
  readonly key: string;
  readonly focus: ScriptFocus;
}

/**
 * « Ouvrir le script »: the request's Scripts tab, or the settings of the folder or collection
 * that holds the script, on the line that stopped.
 */
@Injectable({ providedIn: 'root' })
export class HttpScriptFocusStore {
  private readonly collections = inject(HttpCollectionsStore);

  private readonly _request = signal<RequestScriptFocus | null>(null);
  /** Taken by the editor of that tab, which clears it. */
  readonly request = this._request.asReadonly();

  open(tabKey: string, stop: ScriptStop): void {
    const focus: ScriptFocus = {
      phase: stop.script ?? 'before',
      line: stop.line ?? null,
    };
    if (stop.level !== 'request' && stop.owner && this.collections.openSettingsOf(stop.owner, focus)) return;
    this._request.set({ key: tabKey, focus });
  }

  taken(): void {
    this._request.set(null);
  }
}
