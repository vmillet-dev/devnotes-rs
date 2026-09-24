import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Backup } from '@core/model/backup.model';
import { IpcError } from '@core/ipc/ipc.error';
import { FakeBackupsRepository } from '@testing/fake-backups-repository';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { BackupsStore } from './backups.store';
import { VaultStore } from './vault.store';

function backup(overrides: Partial<Backup> = {}): Backup {
  return {
    id: '2026-07-25_09-00-00',
    takenAt: new Date('2026-07-25T09:00:00.000Z'),
    bytes: 2_500_000,
    openable: true,
    ...overrides,
  };
}

describe('BackupsStore', () => {
  let store: BackupsStore;
  let repository: FakeBackupsRepository;
  let vault: FakeVaultRepository;

  function configure(copies: readonly Backup[]): void {
    TestBed.resetTestingModule();
    repository = new FakeBackupsRepository(copies);
    vault = new FakeVaultRepository();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ backupsRepository: repository, vaultRepository: vault })],
    });
    store = TestBed.inject(BackupsStore);
  }

  beforeEach(() => {
    configure([backup(), backup({ id: '2026-07-24_09-00-00' })]);
  });

  it('lists the copies that exist', async () => {
    await store.load();

    expect(store.backups()).toHaveLength(2);
    expect(store.isLoading()).toBe(false);
  });

  it('says nothing was read rather than pretending there are none', async () => {
    repository.failNext = new IpcError('list_backups', new Error('no bridge'));

    await store.load();

    expect(store.backups()).toEqual([]);
  });

  /**
   * The shape the tag manager and the trash already use: the trigger only proposes,
   * and something else confirms. This is the one gesture that replaces a whole corpus.
   */
  it('proposes a restore rather than running one', async () => {
    await store.load();

    store.ask(store.backups()[0]);

    expect(store.pending()?.id).toBe('2026-07-25_09-00-00');
    expect(repository.restored).toEqual([]);
  });

  /** It opens for nobody, so offering it would be offering to lose the library. */
  it('refuses to propose a copy with no key file', async () => {
    configure([backup({ openable: false })]);
    await store.load();

    store.ask(store.backups()[0]);

    expect(store.pending()).toBeNull();
  });

  it('gives up on the proposal without running it', async () => {
    await store.load();
    store.ask(store.backups()[0]);

    store.dismiss();

    expect(store.pending()).toBeNull();
    expect(repository.restored).toEqual([]);
  });

  it('restores only what was proposed, once', async () => {
    await store.load();
    store.ask(store.backups()[1]);

    await store.confirm();

    expect(repository.restored).toEqual(['2026-07-24_09-00-00']);
    expect(store.pending()).toBeNull();
  });

  it('does nothing when nothing was proposed', async () => {
    await store.confirm();

    expect(repository.restored).toEqual([]);
  });

  /**
   * The library is closed by the time the command returns, so the shell has to be sent
   * back to the gate: the restored copy needs a passphrase, and asking for it is the only
   * proof the right file is in place.
   */
  it('sends the shell back to the gate once the copy is in place', async () => {
    const vaultStore = TestBed.inject(VaultStore);
    await vaultStore.load();
    expect(vaultStore.isUnlocked()).toBe(true);
    vault.answer = 'locked';

    await store.load();
    store.ask(store.backups()[0]);
    await store.confirm();

    expect(vaultStore.isUnlocked()).toBe(false);
  });

  /** The library stays where it is on a failure, and the offer stays on screen. */
  it('keeps the proposal standing when the restore was refused', async () => {
    const vaultStore = TestBed.inject(VaultStore);
    await vaultStore.load();
    await store.load();
    store.ask(store.backups()[0]);
    repository.failNext = new IpcError('restore_backup', new Error('locked file'));

    await store.confirm();

    expect(store.pending()?.id).toBe('2026-07-25_09-00-00');
    expect(vaultStore.isUnlocked()).toBe(true);
    expect(store.isRestoring()).toBe(false);
  });
});
