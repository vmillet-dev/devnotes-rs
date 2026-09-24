import { Injectable } from '@angular/core';
import { PassphraseChange, commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { VaultState } from '@core/model/vault.model';

/**
 * ⚠️ The passphrase crosses the bridge and is never held on this side: nothing here keeps
 * it, and the field that carried it is cleared as soon as it has been sent.
 */
@Injectable({ providedIn: 'root' })
export class VaultRepository {
  async state(): Promise<VaultState> {
    return unwrap('vault_state', await commands.vaultState());
  }

  async create(passphrase: string): Promise<void> {
    unwrap('create_vault', await commands.createVault(passphrase));
  }

  async unlock(passphrase: string): Promise<void> {
    unwrap('unlock_vault', await commands.unlockVault(passphrase));
  }

  /** Answers where everything was moved, which is what the interface has to say. */
  async setAsideDamagedLibrary(): Promise<string> {
    return unwrap('set_aside_damaged_library', await commands.setAsideDamagedLibrary());
  }

  /**
   * The same move for a library nobody can open any more. ⚠️ Nothing is recovered: the
   * notes leave sealed, and the key file goes with them so the day the phrase comes back
   * there is still something to try it on.
   */
  async archiveLockedLibrary(): Promise<string> {
    return unwrap('archive_locked_library', await commands.archiveLockedLibrary());
  }

  /** Answers what the change reached, which the interface has to say out loud. */
  async changePassphrase(current: string, next: string): Promise<PassphraseChange> {
    return unwrap('change_passphrase', await commands.changePassphrase(current, next));
  }
}
