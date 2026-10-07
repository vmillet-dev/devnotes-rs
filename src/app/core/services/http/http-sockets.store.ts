import { Injectable, inject, signal } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { IpcError } from '@core/ipc/ipc.error';
import { WEBSOCKET_SUBSCRIBER } from '@core/ipc/websocket-events';
import { RequestParts, SocketEvent, WebsocketEvent } from '@core/model/http.model';
import { AppNotice, ipcNotice } from '@core/services/errors/error-notifier.service';
import { subscribeCancellable } from '@core/utils/subscription.util';

/** What a log keeps: past it, the oldest go. */
export const LOG_KEPT = 500;

export interface SocketEntry {
  /** In arrival order, to track a row by. */
  readonly number: number;
  readonly at: Date;
  readonly event: SocketEvent;
}

export interface SocketState {
  /** What Rust names the socket by, fixed at connection: a draft saved meanwhile is rekeyed. */
  readonly socketId: string;
  readonly phase: 'connecting' | 'open' | 'closing' | 'closed';
  readonly protocol: string | null;
  /** Why it would not open, or why it broke. */
  readonly notice: AppNotice | null;
  readonly log: readonly SocketEntry[];
}

/** What a connection needs of its tab. */
export interface Connectable {
  readonly key: string;
  readonly place: { readonly collectionId: string; readonly folderId: string | null } | null;
  readonly draft: { readonly parts: RequestParts };
}

/** The sockets, one a tab: Rust holds them, this keeps where each stands and its log. */
@Injectable({ providedIn: 'root' })
export class HttpSocketsStore {
  private readonly repository = inject(HttpRepository);

  private readonly _states = signal<ReadonlyMap<string, SocketState>>(new Map());
  readonly states = this._states.asReadonly();

  private connections = 0;
  private entries = 0;

  constructor() {
    subscribeCancellable(inject(WEBSOCKET_SUBSCRIBER), (event: WebsocketEvent) => this.receive(event));
  }

  async connect(tab: Connectable): Promise<void> {
    const current = this._states().get(tab.key);
    if (current && current.phase !== 'closed') return;
    const socketId = `${tab.key}#${++this.connections}`;
    this.set(tab.key, {
      socketId,
      phase: 'connecting',
      protocol: null,
      notice: null,
      log: current?.log ?? [],
    });
    try {
      await this.repository.connectWebsocket(socketId, tab.draft.parts, tab.place);
    } catch (error) {
      if (!(error instanceof IpcError)) console.error(error);
      this.update(socketId, (state) => ({
        ...state,
        phase: 'closed',
        notice: ipcNotice(error, { key: 'http.socket.failed' }),
      }));
    }
  }

  async send(key: string, text: string): Promise<void> {
    const state = this._states().get(key);
    if (state?.phase !== 'open') return;
    await this.repository.sendWebsocket(state.socketId, text).catch(() => false);
  }

  async close(key: string): Promise<void> {
    const state = this._states().get(key);
    if (state?.phase !== 'open' && state?.phase !== 'connecting') return;
    this.set(key, { ...state, phase: 'closing' });
    await this.repository.closeWebsocket(state.socketId).catch(() => false);
  }

  clearLog(key: string): void {
    const state = this._states().get(key);
    if (state) this.set(key, { ...state, log: [] });
  }

  rekey(from: string, to: string): void {
    const state = this._states().get(from);
    if (!state) return;
    const next = new Map(this._states());
    next.delete(from);
    next.set(to, state);
    this._states.set(next);
  }

  /** A tab closed: its socket with it. */
  forget(key: string): void {
    const state = this._states().get(key);
    if (!state) return;
    if (state.phase !== 'closed') void this.repository.closeWebsocket(state.socketId).catch(() => false);
    const next = new Map(this._states());
    next.delete(key);
    this._states.set(next);
  }

  private receive({ id, at, event }: WebsocketEvent): void {
    this.update(id, (state) => {
      const log = [...state.log, { number: ++this.entries, at: new Date(at), event }].slice(-LOG_KEPT);
      switch (event.kind) {
        case 'opened':
          return { ...state, phase: 'open', protocol: event.protocol, notice: null, log };
        case 'closed':
          return { ...state, phase: 'closed', log };
        case 'failed':
          return {
            ...state,
            phase: 'closed',
            log,
            notice: ipcNotice(
              new IpcError('websocket', { code: event.code, params: {}, detail: event.detail }),
              {
                key: 'http.socket.failed',
              },
            ),
          };
        case 'sent':
        case 'received':
          return { ...state, log };
      }
    });
  }

  private set(key: string, state: SocketState): void {
    this._states.set(new Map(this._states()).set(key, state));
  }

  /** Under whatever key the tab has now; an event for a socket replaced since is dropped. */
  private update(socketId: string, change: (state: SocketState) => SocketState): void {
    for (const [key, state] of this._states()) {
      if (state.socketId === socketId) {
        this.set(key, change(state));
        return;
      }
    }
  }
}
