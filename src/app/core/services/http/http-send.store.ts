import { Injectable, inject, signal } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { IpcError } from '@core/ipc/ipc.error';
import { RequestDraft, SentResponse } from '@core/model/http.model';
import { AppNotice, ErrorNotifier, ipcNotice } from '@core/services/errors/error-notifier.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';

/**
 * Where a tab's send stands. `sendId` is the name Rust keeps the body under, fixed at the send:
 * a draft saved meanwhile changes its tab's key, not that.
 */
export type SendState =
  | { readonly phase: 'sending'; readonly sendId: string }
  | { readonly phase: 'answered'; readonly sendId: string; readonly response: SentResponse }
  | { readonly phase: 'failed'; readonly sendId: string; readonly notice: AppNotice }
  | { readonly phase: 'cancelled'; readonly sendId: string };

/** What a send needs of its tab. */
export interface Sendable {
  readonly key: string;
  readonly place: { readonly collectionId: string; readonly folderId: string | null } | null;
  readonly draft: RequestDraft;
}

const EXTENSIONS: readonly (readonly [string, string])[] = [
  ['json', 'json'],
  ['html', 'html'],
  ['xml', 'xml'],
  ['csv', 'csv'],
  ['javascript', 'js'],
  ['image/png', 'png'],
  ['image/jpeg', 'jpg'],
  ['image/gif', 'gif'],
  ['image/svg', 'svg'],
  ['image/webp', 'webp'],
  ['pdf', 'pdf'],
  ['zip', 'zip'],
  ['text/', 'txt'],
];

export function contentTypeOf(response: SentResponse): string | null {
  return response.headers.find((header) => header.key?.toLowerCase() === 'content-type')?.value ?? null;
}

/** The name the save dialog proposes, its extension read off the type. */
export function responseFileName(response: SentResponse): string {
  const type = (contentTypeOf(response) ?? '').toLowerCase();
  const extension = EXTENSIONS.find(([word]) => type.includes(word))?.[1] ?? 'bin';
  return `response.${extension}`;
}

/** The sends, one a tab: Rust sends; this keeps what each tab shows of it. */
@Injectable({ providedIn: 'root' })
export class HttpSendStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly dialogs = inject(FileDialogService);
  private readonly status = inject(StatusNotifier);

  private readonly _states = signal<ReadonlyMap<string, SendState>>(new Map());
  readonly states = this._states.asReadonly();

  private sends = 0;

  isSending(key: string): boolean {
    return this._states().get(key)?.phase === 'sending';
  }

  async send(tab: Sendable): Promise<void> {
    if (this.isSending(tab.key)) return;
    const previous = this._states().get(tab.key);
    const sendId = `${tab.key}#${++this.sends}`;
    this.set(tab.key, { phase: 'sending', sendId });
    const sent = this.repository.send(sendId, tab.draft, tab.place);
    if (previous) void this.release(previous.sendId);
    try {
      const response = await sent;
      this.settle(sendId, { phase: 'answered', sendId, response });
    } catch (error) {
      if (error instanceof IpcError && error.code === 'httpCancelled') {
        this.settle(sendId, { phase: 'cancelled', sendId });
      } else {
        console.error(error);
        this.settle(sendId, {
          phase: 'failed',
          sendId,
          notice: ipcNotice(error, { key: 'http.send.failed' }),
        });
      }
    }
  }

  async cancel(key: string): Promise<void> {
    const state = this._states().get(key);
    if (state?.phase !== 'sending') return;
    await this.errors.attempt('http.send.failed', () => this.repository.cancel(state.sendId));
  }

  /** The whole body to a file the user names. */
  async saveBody(key: string): Promise<void> {
    const state = this._states().get(key);
    if (state?.phase !== 'answered') return;
    const path = await this.dialogs.chooseDestination(responseFileName(state.response));
    if (path === null) return;
    const saved = await this.errors.attempt('http.response.saveFailed', () =>
      this.repository.saveResponse(state.sendId, path),
    );
    if (saved) this.status.notify({ key: 'http.response.saved', params: { path } });
  }

  /** A draft saved for the first time: its tab is known by the request's id from then on. */
  rekey(from: string, to: string): void {
    const state = this._states().get(from);
    if (!state) return;
    const next = new Map(this._states());
    next.delete(from);
    next.set(to, state);
    this._states.set(next);
  }

  /** A tab closed: what it sent, and what Rust kept of the answer, go. */
  forget(key: string): void {
    const state = this._states().get(key);
    if (!state) return;
    if (state.phase === 'sending') void this.repository.cancel(state.sendId).catch(() => false);
    const next = new Map(this._states());
    next.delete(key);
    this._states.set(next);
    void this.release(state.sendId);
  }

  private set(key: string, state: SendState): void {
    this._states.set(new Map(this._states()).set(key, state));
  }

  /** Under whatever key the tab has now, and only if no newer send replaced it. */
  private settle(sendId: string, state: SendState): void {
    for (const [key, current] of this._states()) {
      if (current.sendId === sendId) {
        this.set(key, state);
        return;
      }
    }
    void this.release(sendId);
  }

  private async release(sendId: string): Promise<void> {
    await this.repository.forgetResponse(sendId).catch(() => undefined);
  }
}
