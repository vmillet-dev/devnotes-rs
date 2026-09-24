import { Injectable, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { hasErrorCode } from '@core/ipc/ipc.error';
import { PassphraseChange } from '@core/ipc/bindings';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { VaultRepository } from '@core/data/vault.repository';
import { VaultState } from '@core/model/vault.model';
import { SEEDED_KEY } from '@core/services/samples/sample-notes.service';

/** The state lives in Rust: a page reload must not ask again for a library the process has open. */
@Injectable({ providedIn: 'root' })
export class VaultStore {
  private readonly repository = inject(VaultRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly preferences = inject(LibraryPreferencesService);

  private readonly _state = signal<VaultState | null>(null);
  private readonly _isWorking = signal(false);
  private readonly _refused = signal(false);
  private readonly _damaged = signal(false);

  /** `null` until the first answer: the shell renders nothing rather than guessing. */
  readonly state = this._state.asReadonly();
  readonly isWorking = this._isWorking.asReadonly();

  /** The last attempt was refused. Cleared as soon as the field is touched again. */
  readonly refused = this._refused.asReadonly();

  /** SQLite says the file is damaged: retyping will not help, so the screen offers something else. */
  readonly damaged = this._damaged.asReadonly();

  readonly isUnlocked = computed(() => this._state() === 'unlocked');
  readonly needsCreating = computed(() => this._state() === 'absent');

  async load(): Promise<void> {
    const state = await this.notifier.attempt('errors.vaultStateFailed', () => this.repository.state());
    if (state !== null) this._state.set(state);
  }

  /** The first launch of a library that has never been encrypted. */
  async create(passphrase: string): Promise<boolean> {
    return this.attempt(() => this.repository.create(passphrase));
  }

  async unlock(passphrase: string): Promise<boolean> {
    return this.attempt(() => this.repository.unlock(passphrase));
  }

  /**
   * A new phrase over the same library; nothing is re-encrypted. Reported from here rather than
   * from the dialog, which closes on the click: a backup left unwrapped still opens with the
   * retired phrase, and saying so is the point.
   */
  async changePassphrase(current: string, next: string): Promise<boolean> {
    return this.attempt(async () => {
      const change = await this.repository.changePassphrase(current, next);
      this.status.notify(revocation(change));
    }, 'errors.passphraseChangeFailed');
  }

  /** Typing again withdraws the refusal. */
  clearRefusal(): void {
    this._refused.set(false);
  }

  /** Moves the damaged library aside so the next unlock starts on a fresh one, and says where. */
  async setAsideDamagedLibrary(): Promise<boolean> {
    return this.moveAside(() => this.repository.setAsideDamagedLibrary(), 'vault.setAside');
  }

  /**
   * Archives a library whose phrase was forgotten, and starts over. It recovers nothing — the
   * notes leave sealed — and the report says where the copy went.
   */
  async archiveLockedLibrary(): Promise<boolean> {
    return this.moveAside(() => this.repository.archiveLockedLibrary(), 'vault.archived');
  }

  /**
   * ⚠️ The samples marker goes with the library, or the fresh one opens with no space, where no
   * note can be created. The state is re-read: a damaged library keeps its key file and comes
   * back `locked`, an archived one takes it and comes back `absent`.
   */
  private async moveAside(move: () => Promise<string>, reportKey: string): Promise<boolean> {
    const moved = await this.notifier.attemptWhile(this._isWorking, 'errors.setAsideFailed', async () => {
      const target = await move();
      this.preferences.forget(SEEDED_KEY);
      this._damaged.set(false);
      this._refused.set(false);
      this.status.notify({ key: reportKey, params: { path: target } });
      await this.load();
    });

    return moved !== null;
  }

  /**
   * A refused passphrase goes beside the field, not into the banner: it is the ordinary answer
   * to a typo. Anything else is a failure.
   */
  private async attempt(action: () => Promise<void>, failureKey = 'errors.unlockFailed'): Promise<boolean> {
    this._isWorking.set(true);
    this._refused.set(false);
    try {
      await action();
      this._state.set('unlocked');
      return true;
    } catch (error) {
      if (hasErrorCode(error, 'wrongPassphrase')) {
        this._refused.set(true);
        return false;
      }

      if (hasErrorCode(error, 'libraryDamaged')) {
        this._damaged.set(true);
        return false;
      }

      this.notifier.reportFailure(failureKey, error);
      return false;
    } finally {
      this._isWorking.set(false);
    }
  }
}

/** Three strings, not one with a count: French keeps the singular where English does not. */
function revocation(change: PassphraseChange): TranslationRef {
  if (change.backupsLeft === 0) {
    return { key: 'settings.security.changed' };
  }

  return change.backupsLeft === 1
    ? { key: 'settings.security.changedOneLeft' }
    : { key: 'settings.security.changedSomeLeft', params: { count: change.backupsLeft } };
}
