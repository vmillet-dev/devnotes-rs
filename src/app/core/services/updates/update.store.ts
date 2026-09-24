import { Injectable, Signal, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AvailableUpdate, DownloadProgress, UpdaterService } from './updater.service';

export type UpdateStatus = 'idle' | 'available' | 'installing' | 'installed';

/** Distinct from `UpdateStatus`: the prompt must not depend on a check that led nowhere. */
export type CheckState = 'idle' | 'checking' | 'upToDate' | 'failed';

/**
 * Nothing installs without an explicit gesture: a silent update would restart the app
 * mid-keystroke, and the editor's drafts only commit on blur.
 *
 * A failed check stays silent — offline, or on a dev build with a placeholder public
 * key, it fails on every launch. A failed install follows an explicit action.
 */
@Injectable({ providedIn: 'root' })
export class UpdateStore {
  private readonly updater = inject(UpdaterService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly settings = inject(SettingsStore);

  private readonly _update = signal<AvailableUpdate | null>(null);
  private readonly _status = signal<UpdateStatus>('idle');
  private readonly _progress = signal<DownloadProgress>(null);
  private readonly _checkState = signal<CheckState>('idle');

  readonly update: Signal<AvailableUpdate | null> = this._update.asReadonly();
  readonly status: Signal<UpdateStatus> = this._status.asReadonly();
  readonly checkState: Signal<CheckState> = this._checkState.asReadonly();

  /** `_update` survives `dismiss()`, so the offer is the status and not a version. */
  readonly offered = computed<AvailableUpdate | null>(() =>
    this._status() === 'idle' ? null : this._update(),
  );

  /**
   * ⚠️ A third thing next to `UpdateStatus` and `CheckState`: "an update exists and is
   * not installed" is not "the prompt is open". `dismiss()` moves the status to `idle`,
   * so a dot reading that would vanish with the dialog it exists to outlive.
   */
  readonly hasPendingUpdate = computed(() => this._update() !== null && this._status() !== 'installed');

  readonly progressPercent = computed<number | null>(() => {
    const progress = this._progress();
    return progress === null ? null : Math.round(progress * 100);
  });

  /**
   * The startup check: silent, and the only one the settings can turn off. An update it
   * finds is still remembered — it simply does not open the prompt.
   */
  async check(): Promise<void> {
    await this.runCheck({ silent: true });
  }

  /** Unlike the startup one it speaks, "there is nothing" included. */
  async checkNow(): Promise<void> {
    await this.runCheck({ silent: false });
  }

  private async runCheck({ silent }: { silent: boolean }): Promise<void> {
    // An install in progress is not overtaken, and an offer on screen needs no repeat.
    if (this._status() !== 'idle') return;

    this._checkState.set('checking');
    try {
      const found = await this.updater.check();
      if (found) {
        this._update.set(found);
        this._status.set(silent && this.isSilenced(found) ? 'idle' : 'available');
      }
      this._checkState.set(found ? 'idle' : 'upToDate');
    } catch (error) {
      this._checkState.set('failed');
      console.warn('Could not check for updates.', error);
      if (!silent) {
        this.notifier.notify({
          ref: { key: 'errors.updateCheckFailed' },
          detail: error instanceof Error ? error.message : String(error),
        });
      }
    }
  }

  /** On Windows the installer stops the app before `relaunch()`; it is still needed elsewhere. */
  async accept(): Promise<void> {
    if (this._status() !== 'available') return;

    this._status.set('installing');
    this._progress.set(null);

    try {
      await this.updater.install((progress) => this._progress.set(progress));
      this._status.set('installed');
      await this.updater.relaunch();
    } catch (error) {
      console.error(error);
      this.notifier.notify({
        ref: { key: 'errors.updateFailed' },
        detail: error instanceof Error ? error.message : String(error),
      });
      // Back to the offered state rather than `idle`: the user can retry from the prompt.
      this._status.set('available');
    }
  }

  /** "Later". A version is what gets written down, never a boolean. */
  async dismiss(skipThisVersion = false): Promise<void> {
    if (this._status() === 'installing') return;

    const version = this._update()?.version;
    if (skipThisVersion && version !== undefined) {
      this.settings.setSkippedUpdate(version);
    }

    // `_update` is left standing: the dot is what it feeds. Only the status moves.
    this._status.set('idle');
    await this.updater.discard();
  }

  private isSilenced(update: AvailableUpdate): boolean {
    return !this.settings.updateNotifications() || this.settings.skippedUpdate() === update.version;
  }
}
