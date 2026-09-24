import { Injectable, inject, signal } from '@angular/core';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ExportReport, ExportScope, ImportReport } from '@core/model/note.model';
import { hasErrorCode } from '@core/ipc/ipc.error';
import { TransferRepository } from '@core/data/transfer.repository';
import { NotesRevision } from './notes-revision';
import { PassphraseAnswer, PassphrasePromptStore } from './passphrase-prompt.store';

function pad(value: number): string {
  return String(value).padStart(2, '0');
}

/**
 * Stamped to the minute, so two exports on the same day are two names — the second was
 * offered the first one's, and replacing it was one Enter away.
 *
 * Local time, not `toISOString`: the name is read by whoever wrote it, and an export
 * taken at 23:30 in Paris was dated the next day.
 */
function defaultFileName(now: Date): string {
  const day = [now.getFullYear(), pad(now.getMonth() + 1), pad(now.getDate())].join('-');
  return `devnotes-${day}-${pad(now.getHours())}${pad(now.getMinutes())}.devnotes`;
}

/**
 * Export then re-import at once adds nothing at all, and saying so explicitly stops it
 * looking like a breakdown. The rest is a ladder of what most deserves saying: an
 * attachment the archive named and did not carry leaves a thumbnail that will never
 * load, and only this says why.
 */
function importedKey(report: ImportReport): string {
  if (report.notesImported === 0) return 'file.importedNothing';
  if (report.attachmentsMissing > 0) return 'file.importedWithoutSomeAttachments';
  if (report.notesDegraded > 0) return 'file.importedFromNewerVersion';
  // A library received arranged is worth saying so: the folders are half of what came in.
  if (report.foldersCreated > 0) return 'file.importedIntoFolders';

  return report.attachmentsImported > 0 ? 'file.importedWithAttachments' : 'file.imported';
}

/** Which of the two files was written is part of the report, not a detail. */
function exportedKey(report: ExportReport): string {
  if (report.protected) {
    return report.attachments > 0 ? 'file.exportedProtectedWithAttachments' : 'file.exportedProtected';
  }

  return report.attachments > 0 ? 'file.exportedWithAttachments' : 'file.exported';
}

function fileNameOf(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

/**
 * Every operation reports, including when it changed nothing. Reports go under the
 * titlebar: the menu closes on the click, and a native dialog would cover it.
 */
@Injectable({ providedIn: 'root' })
export class TransferStore {
  private readonly repository = inject(TransferRepository);
  private readonly dialog = inject(FileDialogService);
  private readonly clipboard = inject(ClipboardService);
  private readonly status = inject(StatusNotifier);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);
  private readonly prompt = inject(PassphrasePromptStore);

  private readonly _isBusy = signal(false);

  readonly isBusy = this._isBusy.asReadonly();

  /** `true` when notes came in, which is what bumps the canvas revision. */
  async import(): Promise<boolean> {
    const path = await this.dialog.pickBundle();
    if (path === null) return false;

    return this.run(async () => {
      const report = await this.readWithPrompt(path);
      if (report === null) return false;

      const params = {
        notes: String(report.notesImported),
        skipped: String(report.notesSkipped),
        degraded: String(report.notesDegraded),
        attachments: String(report.attachmentsImported),
        missing: String(report.attachmentsMissing),
        folders: String(report.foldersCreated),
        path: fileNameOf(path),
      };

      this.status.notify({ key: importedKey(report), params });

      const changed = report.notesImported > 0 || report.spacesCreated > 0 || report.foldersCreated > 0;
      if (changed) this.revision.bump();

      return changed;
    }, 'errors.importFailed');
  }

  /** A `null` `spaceId` exports the whole corpus. */
  async export(spaceId: string | null, now: Date): Promise<void> {
    const scope: ExportScope = spaceId === null ? { kind: 'library' } : { kind: 'space', spaceId };
    await this.write((path, passphrase) => this.repository.export(path, scope, passphrase), now);
  }

  async exportSelection(ids: readonly string[], now: Date): Promise<void> {
    if (!this.requireSelection(ids)) return;

    const scope: ExportScope = { kind: 'notes', ids: [...ids] };
    await this.write((path, passphrase) => this.repository.export(path, scope, passphrase), now);
  }

  /** Sharing stops at the clipboard: nothing is sent anywhere. */
  async copyAsMarkdown(ids: readonly string[]): Promise<void> {
    if (!this.requireSelection(ids)) return;

    await this.run(async () => {
      const markdown = await this.repository.share(ids);
      if (!(await this.clipboard.copy(markdown))) {
        this.notifier.notify({ ref: { key: 'errors.copyFailed' } });
        return false;
      }

      this.status.notify({ key: 'file.copied', params: { notes: String(ids.length) } });
      return true;
    }, 'errors.shareFailed');
  }

  /** The prompt never outlives the operation it was opened for, failure included. */
  private async readWithPrompt(path: string): Promise<ImportReport | null> {
    const isProtected = await this.repository.isProtected(path);
    try {
      return await this.read(path, isProtected);
    } finally {
      this.prompt.close();
    }
  }

  /**
   * A refused phrase asks again rather than failing the import: it is the ordinary
   * answer to a typo, and a file nobody can reopen for one is a file lost. `null` when
   * the user gave up at the prompt, which is not a failure either.
   */
  private async read(path: string, isProtected: boolean): Promise<ImportReport | null> {
    let refused = false;

    for (;;) {
      let passphrase: string | null = null;
      if (isProtected) {
        const answer = await this.prompt.ask({ purpose: 'unlock', fileName: fileNameOf(path), refused });
        if (answer.kind !== 'phrase') return null;
        passphrase = answer.value;
      }

      try {
        return await this.repository.import(path, passphrase);
      } catch (error) {
        if (!isProtected || !hasErrorCode(error, 'wrongPassphrase')) throw error;
        refused = true;
      }
    }
  }

  private requireSelection(ids: readonly string[]): boolean {
    if (ids.length > 0) return true;

    this.notifier.notify({ ref: { key: 'file.needsSelection' } });
    return false;
  }

  /**
   * The phrase is asked for once the destination is known, and never kept: it goes
   * straight to the command, which derives a key of its own for that one file.
   */
  private async write(
    action: (path: string, passphrase: string | null) => Promise<ExportReport>,
    now: Date,
  ): Promise<void> {
    const path = await this.dialog.chooseBundleDestination(defaultFileName(now));
    if (path === null) return;

    const answer = await this.prompt.ask({ purpose: 'protect', fileName: fileNameOf(path), refused: false });
    if (answer.kind === 'cancelled') return;

    await this.run(async () => {
      try {
        return await this.writeWith(action, path, answer);
      } finally {
        this.prompt.close();
      }
    }, 'errors.exportFailed');
  }

  private async writeWith(
    action: (path: string, passphrase: string | null) => Promise<ExportReport>,
    path: string,
    answer: PassphraseAnswer,
  ): Promise<boolean> {
    const report = await action(path, answer.kind === 'phrase' ? answer.value : null);

    if (report.notes === 0) {
      this.status.notify({ key: 'file.emptyLibrary' });
      return false;
    }

    // The file name is part of the report: an export whose landing place is unknown
    // is no use.
    this.status.notify({
      key: exportedKey(report),
      params: {
        notes: String(report.notes),
        attachments: String(report.attachments),
        path: fileNameOf(path),
      },
    });

    return true;
  }

  private async run(action: () => Promise<boolean>, failureKey: string): Promise<boolean> {
    return (await this.notifier.attemptWhile(this._isBusy, failureKey, action)) ?? false;
  }
}
