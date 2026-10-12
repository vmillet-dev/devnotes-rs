import { Injectable, inject, signal } from '@angular/core';
import { SettingsStore } from '@core/services/settings/settings.store';
import { KEPT_RECENT_TOOLS, ToolCategory } from './tool.model';

/**
 * Where the tools area is: its home, one category of it, or one tool. Kept across a trip to the
 * notes, like everything the tools hold; the recent tools are remembered with the application.
 */
@Injectable({ providedIn: 'root' })
export class ToolsStore {
  private readonly settings = inject(SettingsStore);

  private readonly _openId = signal<string | null>(null);
  private readonly _category = signal<ToolCategory | null>(null);
  private readonly _search = signal('');
  private readonly _searchWanted = signal(false);
  private readonly _entry = signal<string | null>(null);

  readonly openId = this._openId.asReadonly();
  /** The row a reference was opened on, from a search of the home: its key in the table. */
  readonly entry = this._entry.asReadonly();
  readonly category = this._category.asReadonly();
  readonly search = this._search.asReadonly();
  /** Ctrl+Shift+T, from any area: the home takes it, whether it was already there or not. */
  readonly searchWanted = this._searchWanted.asReadonly();
  readonly recents = this.settings.recentTools;

  open(id: string, entry: string | null = null): void {
    this._openId.set(id);
    this._entry.set(entry);
    const others = this.recents().filter((recent) => recent.id !== id);
    this.settings.recentTools.write(
      [{ id, at: new Date().toISOString() }, ...others].slice(0, KEPT_RECENT_TOOLS),
    );
  }

  /** The whole catalogue, or one category of it. */
  home(category: ToolCategory | null = null): void {
    this._openId.set(null);
    this._entry.set(null);
    this._category.set(category);
  }

  /** The reference is being searched by hand: the row it was opened on is one among the others. */
  forgetEntry(): void {
    this._entry.set(null);
  }

  setSearch(query: string): void {
    this._search.set(query);
  }

  requestSearch(): void {
    this.home();
    this._searchWanted.set(true);
  }

  searchTaken(): void {
    this._searchWanted.set(false);
  }
}
