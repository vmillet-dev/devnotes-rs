import { Injectable, Signal, WritableSignal, signal } from '@angular/core';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { IpcError, IpcErrorCode } from '@core/ipc/ipc.error';

export interface AppNotice {
  readonly ref: TranslationRef;
  /** Raw technical detail, shown in the background. */
  readonly detail?: string;
}

export function errorDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * ⚠️ `Record` and not `Partial`: a variant added in Rust breaks the build here until its
 * key is decided. `null` means the caller's own action message says it better.
 */
const CODE_KEYS: Record<IpcErrorCode, string | null> = {
  noteNotFound: 'errors.noteGone',
  spaceNotFound: 'errors.spaceGone',
  duplicateSpaceName: 'errors.spaceNameTaken',
  folderNotFound: 'errors.folderGone',
  duplicateFolderName: 'errors.folderNameTaken',
  attachmentNotFound: 'errors.attachmentGone',
  revisionNotFound: 'errors.revisionGone',
  libraryNotFound: 'errors.libraryGone',
  libraryOpen: 'errors.libraryOpen',
  lastLibrary: 'errors.lastLibrary',
  nothingToSetAside: 'errors.nothingToSetAside',
  backupNotFound: 'errors.backupGone',
  backupUnopenable: 'errors.backupUnopenable',
  fileAccess: 'errors.fileAccess',
  importFormat: 'errors.importFormat',
  invalidInput: 'errors.invalidInput',
  storageUnavailable: 'errors.storageUnavailable',
  wrongPassphrase: 'errors.wrongPassphrase',
  locked: 'errors.locked',
  passphraseRequired: 'errors.passphraseRequired',
  libraryDamaged: 'errors.libraryDamaged',
  storage: null,
};

/** `fallback` carries the action attempted, used when the cause adds nothing. */
export function ipcNotice(
  error: unknown,
  fallback: TranslationRef,
  params: Record<string, string> = {},
): AppNotice {
  const detail = errorDetail(error);

  if (error instanceof IpcError && error.code !== null) {
    const key = CODE_KEYS[error.code];
    if (key) {
      return { ref: { key, params: { ...params, ...error.params } }, detail };
    }
  }

  return { ref: fallback, detail };
}

/** One error is kept at a time, so banners do not stack. */
@Injectable({ providedIn: 'root' })
export class ErrorNotifier {
  private readonly _notice = signal<AppNotice | null>(null);

  readonly notice: Signal<AppNotice | null> = this._notice.asReadonly();

  notify(notice: AppNotice): void {
    this._notice.set(notice);
  }

  dismiss(): void {
    this._notice.set(null);
  }

  /** A cause the back end named wins over `key`. */
  reportFailure(key: string, error: unknown, params?: Record<string, string>): void {
    console.error(error);
    this.notify(ipcNotice(error, { key }, params));
  }

  /**
   * Runs `action` and reports its failure under `key`; `null` means it failed. A thunk
   * rather than a promise, so a synchronous throw is caught too.
   */
  attempt<T>(key: string, action: () => Promise<T>, params?: Record<string, string>): Promise<T | null> {
    const report = (error: unknown): null => {
      this.reportFailure(key, error, params);
      return null;
    };

    // ⚠️ `.catch` rather than `async`/`await`: wrapping adds two microtask hops between
    // the call and its answer, enough to change when a rendered view settles. The `try`
    // covers the synchronous throw alone.
    let pending: Promise<T>;
    try {
      pending = action(); // NOSONAR S4822: the rejection is `.catch`'s, below
    } catch (error) {
      return Promise.resolve(report(error));
    }

    return pending.catch(report);
  }

  /** `attempt`, with `flag` raised for exactly the length of the call, failure included. */
  async attemptWhile<T>(
    flag: WritableSignal<boolean>,
    key: string,
    action: () => Promise<T>,
    params?: Record<string, string>,
  ): Promise<T | null> {
    flag.set(true);
    try {
      return await this.attempt(key, action, params);
    } finally {
      flag.set(false);
    }
  }
}
