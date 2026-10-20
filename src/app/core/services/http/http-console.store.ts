import { Injectable, inject, signal } from '@angular/core';
import { HTTP_CONSOLE_SUBSCRIBER } from '@core/ipc/http-console-events';
import {
  CONSOLE_LINES_KEPT,
  ConsoleEntry,
  ConsoleEvent,
  runConsoleKey,
} from '@core/model/http-console.model';
import { subscribeCancellable } from '@core/utils/subscription.util';
import { HttpSendStore } from './http-send.store';

const NONE: readonly ConsoleEntry[] = [];

/**
 * Each tab's console: what Rust says as a send goes — its traffic, the scripts' logs, the tests
 * that failed, what stopped —, kept for the session and never written. Folded at first.
 */
@Injectable({ providedIn: 'root' })
export class HttpConsoleStore {
  private readonly sending = inject(HttpSendStore);

  private readonly _lines = signal<ReadonlyMap<string, readonly ConsoleEntry[]>>(new Map());
  readonly lines = this._lines.asReadonly();
  private readonly _open = signal(false);
  /** One for the area: a console unfolded stays so from tab to tab. */
  readonly open = this._open.asReadonly();

  constructor() {
    subscribeCancellable(inject(HTTP_CONSOLE_SUBSCRIBER), (event: ConsoleEvent) => this.take(event));
  }

  linesOf(key: string): readonly ConsoleEntry[] {
    return this._lines().get(key) ?? NONE;
  }

  toggle(): void {
    this._open.update((open) => !open);
  }

  clear(key: string): void {
    this.replace(key, null);
  }

  /** A draft saved for the first time: its tab is known by the request's id from then on. */
  rekey(from: string, to: string): void {
    const lines = this._lines().get(from);
    if (lines === undefined) return;
    const next = new Map(this._lines());
    next.delete(from);
    next.set(to, lines);
    this._lines.set(next);
  }

  forget(key: string): void {
    this.replace(key, null);
  }

  private take(event: ConsoleEvent): void {
    const key = runConsoleKey(event.id) ?? this.sending.keyOf(event.id);
    if (key === null) return;
    const entry: ConsoleEntry = {
      at: Date.parse(event.at),
      sendId: event.id,
      request: event.request,
      iteration: event.iteration,
      line: event.line,
    };
    this.replace(key, [...this.linesOf(key), entry].slice(-CONSOLE_LINES_KEPT));
  }

  private replace(key: string, lines: readonly ConsoleEntry[] | null): void {
    if (lines === null && !this._lines().has(key)) return;
    const next = new Map(this._lines());
    if (lines === null) next.delete(key);
    else next.set(key, lines);
    this._lines.set(next);
  }
}
