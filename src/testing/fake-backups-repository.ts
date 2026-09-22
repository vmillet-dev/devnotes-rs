import { BackupsRepository } from '@core/data/backups.repository';
import { Backup } from '@core/model/backup.model';
import { IpcError } from '@core/ipc/ipc.error';

/** The real one reaches for the Tauri bridge, absent under jsdom. */
export class FakeBackupsRepository implements Pick<BackupsRepository, keyof BackupsRepository> {
  /** When set, the next call rejects with it. */
  failNext: IpcError | null = null;

  /** Which copies were asked to be restored, in order. */
  restored: string[] = [];

  constructor(private copies: readonly Backup[] = []) {}

  async list(): Promise<readonly Backup[]> {
    this.check();

    return this.copies;
  }

  async restore(id: string): Promise<string> {
    this.check();
    this.restored.push(id);
    // ⚠️ The real one leaves the library closed, and the copies are gone with it: what a
    // spec asserts after a restore is the gate, never this list.
    this.copies = [];

    return `/data/replaced/${id}`;
  }

  private check(): void {
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;
  }
}
