import type { AppError, ErrorCode } from './bindings';

/**
 * Codes, never text: a sentence written in Rust would impose its language on the whole
 * interface. A plain alias, so a variant added there breaks the build here.
 */
export type IpcErrorCode = ErrorCode;

/** Redeclared rather than imported: the generator writes it inline without naming it. */
export type IpcResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError };

/**
 * ⚠️ A runtime guard despite the typing: `bindings.ts` declares an `AppError` where
 * Tauri may have rejected with something else (see `IpcError`). A `Record` over the
 * generated union, so it cannot fall behind a variant added in Rust.
 */
const IPC_ERROR_CODES: Record<IpcErrorCode, true> = {
  noteNotFound: true,
  spaceNotFound: true,
  duplicateSpaceName: true,
  folderNotFound: true,
  duplicateFolderName: true,
  attachmentNotFound: true,
  revisionNotFound: true,
  libraryNotFound: true,
  libraryOpen: true,
  lastLibrary: true,
  nothingToSetAside: true,
  backupNotFound: true,
  backupUnopenable: true,
  fileAccess: true,
  importFormat: true,
  invalidInput: true,
  storageUnavailable: true,
  wrongPassphrase: true,
  locked: true,
  passphraseRequired: true,
  libraryDamaged: true,
  storage: true,
};

function isAppError(cause: unknown): cause is AppError {
  if (typeof cause !== 'object' || cause === null) return false;
  const candidate = cause as Partial<AppError>;
  return (
    typeof candidate.code === 'string' &&
    candidate.code in IPC_ERROR_CODES &&
    typeof candidate.detail === 'string'
  );
}

function describeCause(cause: unknown): string {
  if (typeof cause === 'string') return cause;
  if (cause instanceof Error) return cause.message;
  return JSON.stringify(cause);
}

/**
 * ⚠️ `code` is `null` when the rejection does not come from our commands: Tauri rejects
 * with a plain string for an unknown command or a bad argument, and `bindings.ts`
 * files that in the `error` branch typed as an `AppError` it is not.
 */
export class IpcError extends Error {
  readonly code: IpcErrorCode | null;
  readonly params: Record<string, string>;

  constructor(
    readonly command: string,
    override readonly cause: unknown,
  ) {
    const structured = isAppError(cause) ? cause : null;
    super(`Tauri command "${command}" failed: ${structured?.detail ?? describeCause(cause)}`);
    this.name = 'IpcError';
    this.code = structured?.code ?? null;
    this.params = structured?.params ?? {};
  }
}

/**
 * The one code a caller acts on rather than reports: a refused passphrase is the ordinary
 * answer to a typo and belongs beside the field, not in the error banner.
 */
export function hasErrorCode(error: unknown, code: IpcErrorCode): boolean {
  return error instanceof IpcError && error.code === code;
}

/** Throws rather than propagating the `status`, which every caller would have to branch on. */
export function unwrap<T>(command: string, result: IpcResult<T>): T {
  if (result.status === 'error') {
    throw new IpcError(command, result.error);
  }
  return result.data;
}
