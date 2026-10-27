import { Injectable, inject, signal } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { IpcError, IpcErrorCode } from '@core/ipc/ipc.error';
import { RequestDraft, SentResponse } from '@core/model/http.model';
import { AppNotice, ErrorNotifier, ipcNotice } from '@core/services/errors/error-notifier.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpEnvironmentsStore } from './http-environments.store';

/**
 * Where a tab's send stands. `sendId` is the name Rust keeps the body under, fixed at the send:
 * a draft saved meanwhile changes its tab's key, not that. `startedAt` is here and not in the
 * pane that counts from it: leaving the tab and coming back must not restart the count.
 */
export type SendState =
  | { readonly phase: 'sending'; readonly sendId: string; readonly startedAt: number }
  | { readonly phase: 'answered'; readonly sendId: string; readonly response: SentResponse }
  | {
      readonly phase: 'failed';
      readonly sendId: string;
      readonly notice: AppNotice;
      /** What the pane answers with an action: a script that stopped, a protected environment. */
      readonly code: IpcErrorCode | null;
      readonly params: Readonly<Record<string, string>>;
    }
  | { readonly phase: 'cancelled'; readonly sendId: string };

const SCRIPT_CODES: ReadonlySet<IpcErrorCode | null> = new Set<IpcErrorCode>([
  'httpScript',
  'httpScriptSkipped',
  'httpScriptMethod',
  'httpScriptProtected',
]);

/** What a send needs of its tab. */
export interface Sendable {
  readonly key: string;
  readonly place: { readonly collectionId: string; readonly folderId: string | null } | null;
  readonly draft: RequestDraft;
  /** For the history: the request it came from, once saved. */
  readonly requestId: string | null;
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
];

export function contentTypeOf(response: SentResponse): string | null {
  return response.headers.find((header) => header.key?.toLowerCase() === 'content-type')?.value ?? null;
}

/**
 * The name the save dialog proposes, its extension read off the type; a type that says no more
 * than text or bytes leaves it to the address, as a file served raw is (`large-file.json`).
 */
export function responseFileName(response: SentResponse): string {
  const type = (contentTypeOf(response) ?? '').toLowerCase();
  const extension =
    EXTENSIONS.find(([word]) => type.includes(word))?.[1] ??
    extensionOfAddress(response.url) ??
    (type.startsWith('text/') ? 'txt' : 'bin');
  return `response.${extension}`;
}

function extensionOfAddress(url: string): string | null {
  try {
    const name = new URL(url).pathname.split('/').pop() ?? '';
    return /\.([a-z0-9]{1,5})$/i.exec(name)?.[1]?.toLowerCase() ?? null;
  } catch {
    return null;
  }
}

/** The sends, one a tab: Rust sends; this keeps what each tab shows of it. */
@Injectable({ providedIn: 'root' })
export class HttpSendStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly dialogs = inject(FileDialogService);
  private readonly status = inject(StatusNotifier);
  private readonly environments = inject(HttpEnvironmentsStore);
  private readonly collections = inject(HttpCollectionsStore);

  private readonly _states = signal<ReadonlyMap<string, SendState>>(new Map());
  readonly states = this._states.asReadonly();
  /** Moves each time a send that left ends: the history has an entry more. */
  private readonly _sent = signal(0);
  readonly sent = this._sent.asReadonly();

  private sends = 0;

  /** The tab a send belongs to now, saved meanwhile or not; `null` once it was replaced. */
  keyOf(sendId: string): string | null {
    for (const [key, state] of this._states()) if (state.sendId === sendId) return key;
    return null;
  }

  isSending(key: string): boolean {
    return this._states().get(key)?.phase === 'sending';
  }

  /** `confirmed`: the yes a protected environment asked for, which its scripts are sent with. */
  async send(tab: Sendable, confirmed = false): Promise<void> {
    if (this.isSending(tab.key)) return;
    if (!this.environments.mayLeave(tab.key, tab.draft.kind, tab.draft.method, confirmed)) return;
    const previous = this._states().get(tab.key);
    const sendId = `${tab.key}#${++this.sends}`;
    this.set(tab.key, { phase: 'sending', sendId, startedAt: Date.now() });
    // A table still being typed in is written first: the send resolves from what Rust holds.
    await this.environments.flush();
    const sent = this.repository.send(
      sendId,
      tab.draft,
      tab.place,
      tab.requestId,
      this.environments.active()?.id ?? null,
      confirmed,
    );
    if (previous) void this.release(previous.sendId);
    try {
      const response = await sent;
      this.settle(sendId, { phase: 'answered', sendId, response });
      this._sent.update((count) => count + 1);
      if (response.scripts?.wrote) void this.readWritten();
    } catch (error) {
      if (error instanceof IpcError && error.code === 'httpCancelled') {
        this.settle(sendId, { phase: 'cancelled', sendId });
      } else {
        console.error(error);
        const code = error instanceof IpcError ? error.code : null;
        this.settle(sendId, {
          phase: 'failed',
          sendId,
          notice: ipcNotice(error, { key: 'http.send.failed' }),
          code,
          params: error instanceof IpcError ? error.params : {},
        });
        // What a script wrote before it stopped is kept.
        if (SCRIPT_CODES.has(code)) void this.readWritten();
        this._sent.update((count) => count + 1);
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

  /** A tab closed, or its pane cleared: what it sent, and what Rust kept of the answer, go. */
  forget(key: string): void {
    const state = this._states().get(key);
    if (!state) return;
    if (state.phase === 'sending') void this.repository.cancel(state.sendId).catch(() => false);
    const next = new Map(this._states());
    next.delete(key);
    this._states.set(next);
    void this.release(state.sendId);
  }

  /** A script wrote variables: the sets and the settings are read again. */
  private async readWritten(): Promise<void> {
    await this.environments.load();
    this.collections.settingsSaved();
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
