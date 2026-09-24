import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import type { Backup as WireBackup } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { Backup } from '@core/model/backup.model';

/**
 * Two wire shapes to put back. JSON has no date type, so `takenAt` crosses as an ISO
 * string; and specta types a `f64` as nullable because JSON cannot carry `NaN` — a size
 * that came back missing is reported as nothing rather than as a hole in the row.
 */
function toBackup(wire: WireBackup): Backup {
  return { ...wire, takenAt: new Date(wire.takenAt), bytes: wire.bytes ?? 0 };
}

@Injectable({ providedIn: 'root' })
export class BackupsRepository {
  async list(): Promise<readonly Backup[]> {
    return unwrap('list_backups', await commands.listBackups()).map(toBackup);
  }

  /**
   * Answers where the library it replaced was moved to, which the interface has to say:
   * "set aside" is only true if the user can be told where.
   *
   * The library is **closed** by the time this returns. Every command answers `Locked`
   * afterwards, so the caller's next move is to send the shell back to the gate.
   */
  async restore(id: string): Promise<string> {
    return unwrap('restore_backup', await commands.restoreBackup(id));
  }
}
