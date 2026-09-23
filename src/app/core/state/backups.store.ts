import { Injectable, inject, signal } from '@angular/core';
import { BackupsRepository } from '@core/data/backups.repository';
import { Backup } from '@core/model/backup.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { VaultStore } from './vault.store';

/**
 * The copies of the library, and the one gesture that puts one back.
 *
 * ⚠️ Restoring is the most destructive thing the application can do — it replaces the
 * whole corpus — so it follows the shape the tag manager and the trash already use: the
 * trigger only **proposes**, `pending()` says which copy and what it would cost, and
 * `confirm()` is what writes. A second click on the button that fired it is the guard a
 * double click defeats.
 */
@Injectable({ providedIn: 'root' })
export class BackupsStore {
  private readonly repository = inject(BackupsRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly vault = inject(VaultStore);

  private readonly _backups = signal<readonly Backup[]>([]);
  private readonly _isLoading = signal(false);
  private readonly _pending = signal<Backup | null>(null);
  private readonly _isRestoring = signal(false);

  readonly backups = this._backups.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly pending = this._pending.asReadonly();
  readonly isRestoring = this._isRestoring.asReadonly();

  async load(): Promise<void> {
    const backups = await this.notifier.attemptWhile(this._isLoading, 'errors.backupsListFailed', () =>
      this.repository.list(),
    );
    if (backups !== null) this._backups.set(backups);
  }

  /** ⚠️ Proposes only. A copy with no key file opens for nobody and is never offered. */
  ask(backup: Backup): void {
    if (!backup.openable) return;

    this._pending.set(backup);
  }

  dismiss(): void {
    this._pending.set(null);
  }

  /**
   * ⚠️ The library is closed by the time this returns, so the shell is sent back to the
   * gate: the restored copy needs a passphrase, and asking for it is the only proof the
   * right file is in place. Reloading the vault state is what destroys the outlet — and
   * with it this panel, which is rendered from a File menu that only exists unlocked.
   */
  async confirm(): Promise<void> {
    const backup = this._pending();
    if (backup === null || this._isRestoring()) return;

    await this.notifier.attemptWhile(this._isRestoring, 'errors.backupRestoreFailed', async () => {
      const aside = await this.repository.restore(backup.id);
      this._pending.set(null);
      this.status.notify({ key: 'backups.restored', params: { path: aside } });
      await this.vault.load();
    });
  }
}
