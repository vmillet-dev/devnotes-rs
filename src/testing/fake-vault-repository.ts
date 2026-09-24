import { VaultRepository } from '@core/data/vault.repository';
import { IpcError } from '@core/ipc/ipc.error';
import { PassphraseChange, Unlocked } from '@core/ipc/bindings';
import { VaultState } from '@core/model/vault.model';

/** The real one reaches for the Tauri bridge, absent under jsdom. */
export class FakeVaultRepository implements Pick<VaultRepository, keyof VaultRepository> {
  /** What `state()` answers. Specs set it before asking the store to load. */
  answer: VaultState = 'unlocked';

  /** When set, the next call rejects with it — a wrong passphrase, or worse. */
  failNext: IpcError | null = null;

  passphrases: string[] = [];

  /** Where a set-aside library was moved, and how many times it was asked for. */
  setAside: string[] = [];
  changes: { current: string; next: string }[] = [];

  /** Where an archived library was moved, and how many times it was asked for. */
  archived: string[] = [];

  /** What the next change answers, so a spec can drive the report it produces. */
  rewrapped: PassphraseChange = { backupsRewrapped: 0, backupsLeft: 0 };

  /** What the next unlock answers, so a spec can drive the warning it produces. */
  unlocked: Unlocked = { belowMinimum: false };

  /** ⚠️ Through the guard like the rest: reading the state is a command too, and it is
   *  the one that fails when there is no bridge at all. */
  async state(): Promise<VaultState> {
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;

    return this.answer;
  }

  async create(passphrase: string): Promise<void> {
    return this.attempt(passphrase);
  }

  async unlock(passphrase: string): Promise<Unlocked> {
    await this.attempt(passphrase);

    return this.unlocked;
  }

  /** Records the pair; `failNext` is how a spec makes the current phrase wrong. */
  async setAsideDamagedLibrary(): Promise<string> {
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;

    const target = `/data/damaged/${this.setAside.length + 1}`;
    this.setAside.push(target);
    // `vault.json` stays where it was: a library that had one comes back locked, one that had
    // lost it comes back with none.
    this.answer = this.answer === 'keyMissing' ? 'absent' : 'locked';

    return target;
  }

  /** ⚠️ The key file travels, so what is left has no library at all: `absent`, not
   *  `locked`, which is what makes the gate ask for a new phrase. */
  async archiveLockedLibrary(): Promise<string> {
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;

    const target = `/data/archived/${this.archived.length + 1}`;
    this.archived.push(target);
    this.answer = 'absent';

    return target;
  }

  async changePassphrase(current: string, next: string): Promise<PassphraseChange> {
    this.changes.push({ current, next });
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;

    return this.rewrapped;
  }

  private async attempt(passphrase: string): Promise<void> {
    this.passphrases.push(passphrase);
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;

    this.answer = 'unlocked';
  }
}
