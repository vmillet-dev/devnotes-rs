import { Injectable, Injector, computed, inject, signal } from '@angular/core';
import { AppEventsService } from '@core/ipc/app-events.service';
import { AppWindowService } from '@core/services/window/app-window.service';
import { HttpSocketsStore } from './http-sockets.store';
import { HttpTabsStore, RequestTab, SaveTarget } from './http-tabs.store';

/** What is about to take the page away: each rebuilds the front end, or ends the process. */
export type LeaveReason = 'quit' | 'switch' | 'restore' | 'update';

/**
 * Asks before the page goes away with requests that are not saved in it. A request typed in a
 * tab and never saved, or a saved one with changes, was gone after a restart — and nothing
 * said so. They are not kept across one: the application asks before they are lost.
 *
 * Everything that takes the page away comes through `confirm`, and goes on only on `true`.
 * ⚠️ Nothing is asked when no tab is modified: leaving stays instant.
 */
@Injectable({ providedIn: 'root' })
export class HttpLeaveStore {
  private readonly injector = inject(Injector);
  private readonly window = inject(AppWindowService);

  /**
   * ⚠️ Resolved when asked, never at construction. This store exists from the first render,
   * before the library's preferences are read; the HTTP stores read them as they are built —
   * what is folded, which tabs were open — and would read nothing.
   */
  private get tabs(): HttpTabsStore {
    return this.injector.get(HttpTabsStore);
  }

  private get sockets(): HttpSocketsStore {
    return this.injector.get(HttpSocketsStore);
  }

  private readonly _asking = signal<LeaveReason | null>(null);
  /** What the question is about, while it is out. */
  readonly asking = this._asking.asReadonly();

  private readonly _placing = signal<RequestTab | null>(null);
  /** A request never saved, being given its place on the way to « Tout enregistrer ». */
  readonly placing = this._placing.asReadonly();

  /** Read live: a tab saved while the question is out leaves the list. */
  readonly modified = computed(() => this.tabs.tabs().filter((tab) => this.tabs.isDirty(tab)));
  /** The sockets that leaving cuts: said in the same question, never one of their own. */
  readonly connected = computed(() => this.tabs.tabs().filter((tab) => this.sockets.isLive(tab.key)).length);

  private answer: ((go: boolean) => void) | null = null;

  constructor() {
    // The page lives as long as the window: nothing to unsubscribe from.
    inject(AppEventsService).on((action) => {
      if (action === 'quit') void this.quitAsked();
    });
  }

  /** Whether the caller may go on: at once when nothing would be lost, else once answered. */
  confirm(reason: LeaveReason): Promise<boolean> {
    if (this.modified().length === 0) return Promise.resolve(true);
    // One question at a time: whoever asked the one before stays where it is.
    this.settle(false);
    this._asking.set(reason);
    return new Promise((resolve) => (this.answer = resolve));
  }

  discard(): void {
    this.settle(true);
  }

  cancel(): void {
    this.settle(false);
  }

  /**
   * Saves every modified request that has a place, then walks the ones never saved through
   * the save dialog, one at a time. ⚠️ A write that fails stops it there with the question
   * still out: the banner says why, and nothing is left behind unsaid.
   */
  async saveAll(): Promise<void> {
    for (const tab of this.modified()) {
      if (tab.requestId === null) continue;
      if ((await this.tabs.save(tab.key)) === null) return;
    }
    this.placeNext();
  }

  async place(target: SaveTarget, name: string): Promise<void> {
    const tab = this._placing();
    if (tab === null) return;
    this._placing.set(null);
    this.tabs.edit(tab.key, { name });
    if ((await this.tabs.save(tab.key, target)) !== null) this.placeNext();
  }

  /** Back to the question: a request not placed is still one that would be lost. */
  cancelPlace(): void {
    this._placing.set(null);
  }

  /**
   * A quit asked from outside the page. With nothing to lose it is done at once; otherwise
   * Rust is told the page holds it — it stops waiting and shows the window — and the process
   * ends here, or not at all.
   */
  private async quitAsked(): Promise<void> {
    if (this.modified().length > 0) await this.window.holdQuit();
    if (await this.confirm('quit')) await this.window.quit();
  }

  private placeNext(): void {
    const draft = this.modified().find((tab) => tab.requestId === null) ?? null;
    if (draft === null) this.settle(true);
    else this._placing.set(draft);
  }

  private settle(go: boolean): void {
    const answer = this.answer;
    this.answer = null;
    this._asking.set(null);
    this._placing.set(null);
    answer?.(go);
  }
}
