import { guard } from './fail-next';
import { TransferRepository } from '@core/data/transfer.repository';
import { IpcError } from '@core/ipc/ipc.error';
import { ExportReport, ExportScope, ImportReport } from '@core/model/note.model';

/** Records the arguments and hands back the report the spec asked for. */
export class FakeTransferRepository implements Pick<TransferRepository, keyof TransferRepository> {
  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  exportedTo: { path: string; spaceId: string | null; passphrase: string | null } | null = null;
  importedFrom: string | null = null;
  inspectedPath: string | null = null;
  importedWith: string | null = null;
  sharedIds: readonly string[] | null = null;
  exportedIds: readonly string[] | null = null;

  /** What `export_is_protected` answers, and what an import then has to be given. */
  fileIsProtected = false;

  /** When set, an import with anything else is refused the way the engine refuses it. */
  expectedPassphrase: string | null = null;

  exportReport: ExportReport = { notes: 3, spaces: 1, folders: 0, attachments: 0, protected: false };
  importReport: ImportReport = {
    foldersCreated: 0,
    spacesCreated: 1,
    notesImported: 2,
    notesSkipped: 0,
    notesDegraded: 0,
    attachmentsImported: 0,
    attachmentsMissing: 0,
  };
  markdown = '## Shared\n\n```txt\nbody\n```\n';

  export(path: string, scope: ExportScope, passphrase: string | null): Promise<ExportReport> {
    return guard(this, () => {
      this.exportedTo = { path, spaceId: scope.kind === 'space' ? scope.spaceId : null, passphrase };
      const protectedFile = passphrase !== null;
      if (scope.kind !== 'notes') return { ...this.exportReport, protected: protectedFile };

      this.exportedIds = scope.ids;
      return { ...this.exportReport, notes: scope.ids.length, protected: protectedFile };
    });
  }

  import(path: string, passphrase: string | null): Promise<ImportReport> {
    return guard(this, () => {
      this.importedFrom = path;
      this.importedWith = passphrase;
      if (this.expectedPassphrase !== null && passphrase !== this.expectedPassphrase) {
        throw new IpcError('import_notes', {
          code: 'wrongPassphrase',
          params: {},
          detail: 'Wrong passphrase',
        });
      }

      return this.importReport;
    });
  }

  isProtected(path: string): Promise<boolean> {
    return guard(this, () => {
      this.inspectedPath = path;
      return this.fileIsProtected;
    });
  }

  share(ids: readonly string[]): Promise<string> {
    return guard(this, () => {
      this.sharedIds = ids;
      return this.markdown;
    });
  }
}
