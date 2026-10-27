import { Injectable, inject, signal } from '@angular/core';
import { BackupsRepository } from '@core/data/backups.repository';
import { Backup } from '@core/model/backup.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { HttpLeaveStore } from '@core/services/http/http-leave.store';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { AppWindowService } from '@core/services/window/app-window.service';

/**
 * The copies of the library, and the one gesture that puts one back.
 *
 * Restoring is the most destructive thing the application can do — it replaces the
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
  private readonly window = inject(AppWindowService);
  private readonly leave = inject(HttpLeaveStore);

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

  /** Proposes only. A copy with no key file opens for nobody and is never offered. */
  ask(backup: Backup): void {
    if (!backup.openable) return;

    this._pending.set(backup);
  }

  dismiss(): void {
    this._pending.set(null);
  }

  /**
   * The library is closed by the time this returns, so the page reloads to the gate: the
   * restored copy needs a passphrase, and asking for it is the only proof the right file is
   * in place. ⚠️ A reload, as opening another library is, and not a `vault.load()`: every
   * store is `providedIn: 'root'` and would carry the replaced library's notes, undo record
   * and open dialogs past the gate.
   */
  async confirm(): Promise<void> {
    const backup = this._pending();
    if (backup === null || this._isRestoring()) return;
    if (!(await this.leave.confirm('restore'))) return;

    await this.notifier.attemptWhile(this._isRestoring, 'errors.backupRestoreFailed', async () => {
      await this.repository.restore(backup.id);
      this._pending.set(null);
      this.window.reload();
    });
  }

  /** Once the page a restore reloads knows the vault's state: said over the gate. */
  async sayRestored(): Promise<void> {
    try {
      const aside = await this.repository.takeRestoredAside();
      if (aside) this.status.notify({ key: 'backups.restored', params: { path: aside } });
    } catch {
      // Nothing to say rather than a startup that fails.
    }
  }
}
