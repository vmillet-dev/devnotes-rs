import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Backup } from '@core/model/backup.model';
import { IpcError } from '@core/ipc/ipc.error';
import { HttpLeaveStore } from '@core/services/http/http-leave.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { FakeBackupsRepository } from '@testing/fake-backups-repository';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeAppWindow } from '@testing/fake-app-window';
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
  let appWindow: FakeAppWindow;

  function configure(copies: readonly Backup[]): void {
    TestBed.resetTestingModule();
    repository = new FakeBackupsRepository(copies);
    vault = new FakeVaultRepository();
    appWindow = new FakeAppWindow();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ backupsRepository: repository, vaultRepository: vault, appWindow })],
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

  /** The restore sends the shell back to the gate: what was typed in a tab and not saved goes. */
  it('asks before a restore that would lose unsaved requests, and restores nothing when told to stay', async () => {
    await store.load();
    store.ask(store.backups()[0]);
    TestBed.inject(HttpTabsStore).newRequest();
    const leave = TestBed.inject(HttpLeaveStore);

    const staying = store.confirm();
    expect(leave.asking()).toBe('restore');
    leave.cancel();
    await staying;
    expect(repository.restored).toEqual([]);
    expect(store.pending()).not.toBeNull();

    const going = store.confirm();
    leave.discard();
    await going;
    expect(repository.restored).toEqual(['2026-07-25_09-00-00']);
  });

  it('does nothing when nothing was proposed', async () => {
    await store.confirm();

    expect(repository.restored).toEqual([]);
  });

  /**
   * SEC-09: sent back to the gate over the same stores, the canvas listed the replaced
   * library's notes once unlocked, and Preferences opened again. The page reloads, as
   * opening another library does, and says where the replaced files went once reloaded.
   */
  it('reloads the page to the gate once the copy is in place, and says so after', async () => {
    await store.load();
    store.ask(store.backups()[0]);
    await store.confirm();

    expect(appWindow.reloaded).toBe(1);
    const status = TestBed.inject(StatusNotifier);
    expect(status.status()).toBeNull();

    await store.sayRestored();
    expect(status.status()).toEqual({
      key: 'backups.restored',
      params: { path: '/data/replaced/2026-07-25_09-00-00' },
    });
    status.dismiss();
    await store.sayRestored();
    expect(status.status()).toBeNull();
  });

  it('says nothing after a reload that no restore caused, nor when it cannot ask', async () => {
    const status = TestBed.inject(StatusNotifier);
    await store.sayRestored();
    expect(status.status()).toBeNull();

    repository.aside = '/data/replaced/x';
    repository.failNext = new IpcError('take_restored_aside', new Error('gone'));
    await store.sayRestored();
    expect(status.status()).toBeNull();
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
    expect(appWindow.reloaded).toBe(0);
  });
});
