import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { VaultRepository } from '@core/data/vault.repository';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { SEEDED_KEY } from '@core/services/samples/sample-notes.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { VaultStore } from './vault.store';

const REFUSED = new IpcError('unlock_vault', {
  code: 'wrongPassphrase',
  params: {},
  detail: 'Wrong passphrase',
});

describe('VaultStore', () => {
  let store: VaultStore;
  let repository: FakeVaultRepository;
  let notifier: ErrorNotifier;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting()] });
    repository = TestBed.inject(VaultRepository) as unknown as FakeVaultRepository;
    store = TestBed.inject(VaultStore);
    notifier = TestBed.inject(ErrorNotifier);
  });

  /**
   * `null` is not "locked": the shell renders neither the canvas nor the gate until
   * Rust has answered, rather than flashing one and replacing it with the other.
   */
  it('knows nothing before the first answer', () => {
    expect(store.state()).toBeNull();
    expect(store.isUnlocked()).toBe(false);
    expect(store.needsCreating()).toBe(false);
  });

  it('reads the state from Rust, which is where it lives', async () => {
    repository.answer = 'locked';
    await store.load();

    expect(store.state()).toBe('locked');
    expect(store.isUnlocked()).toBe(false);
    expect(store.needsCreating()).toBe(false);
  });

  it('asks for a passphrase twice on a library that has never had one', async () => {
    repository.answer = 'absent';
    await store.load();

    expect(store.needsCreating()).toBe(true);
  });

  /** A page reload must not ask again for a library this process already has open. */
  it('stays unlocked for a front end that rebooted', async () => {
    repository.answer = 'unlocked';
    await store.load();

    expect(store.isUnlocked()).toBe(true);
  });

  it('reports a state it could not read rather than guessing', async () => {
    repository.failNext = new IpcError('vault_state', {
      code: 'storage',
      params: {},
      detail: 'no bridge',
    });

    await store.load();

    expect(store.state()).toBeNull();
    expect(notifier.notice()?.ref.key).toBe('errors.vaultStateFailed');
  });

  describe('opening the library', () => {
    it('unlocks with the phrase it was given', async () => {
      expect(await store.unlock('an end-to-end passphrase')).toBe(true);

      expect(repository.passphrases).toEqual(['an end-to-end passphrase']);
      expect(store.isUnlocked()).toBe(true);
      expect(TestBed.inject(StatusNotifier).status()).toBeNull();
    });

    /** Refusing a phrase chosen under the older floor would lock someone out of their notes. */
    it('opens under a phrase below the floor, and says so', async () => {
      expect(await store.unlock('eight ch')).toBe(true);

      expect(store.isUnlocked()).toBe(true);
      expect(TestBed.inject(StatusNotifier).status()).toEqual({
        key: 'vault.belowMinimum',
        params: { length: 12 },
      });
    });

    /** Six keys are twelve UTF-16 units and six characters, which is what Rust counts. */
    it('counts characters as Rust does, not UTF-16 units', async () => {
      await store.unlock('🔑'.repeat(6));

      expect(TestBed.inject(StatusNotifier).status()?.key).toBe('vault.belowMinimum');
    });

    it('creates on a first launch, and the library is open straight after', async () => {
      repository.answer = 'absent';
      await store.load();

      expect(await store.create('a first passphrase')).toBe(true);
      expect(store.isUnlocked()).toBe(true);
    });

    /**
     * A refused passphrase is the ordinary answer to a typo: it belongs beside the
     * field, never in the error banner, which is for things that went wrong.
     */
    it('refuses beside the field, not in the banner', async () => {
      repository.failNext = REFUSED;

      expect(await store.unlock('not it')).toBe(false);
      expect(store.refused()).toBe(true);
      expect(notifier.notice()).toBeNull();
      expect(store.isUnlocked()).toBe(false);
    });

    it('sends anything else to the banner, where failures go', async () => {
      repository.failNext = new IpcError('unlock_vault', {
        code: 'storage',
        params: {},
        detail: 'disk gone',
      });

      expect(await store.unlock('a passphrase')).toBe(false);
      expect(store.refused()).toBe(false);
      expect(notifier.notice()?.ref.key).toBe('errors.unlockFailed');
    });

    it('withdraws the refusal when the field is touched again', async () => {
      repository.failNext = REFUSED;
      await store.unlock('not it');

      store.clearRefusal();

      expect(store.refused()).toBe(false);
    });

    it('clears a standing refusal before trying again', async () => {
      repository.failNext = REFUSED;
      await store.unlock('not it');
      expect(store.refused()).toBe(true);

      expect(await store.unlock('the right one')).toBe(true);
      expect(store.refused()).toBe(false);
    });

    it('says it is working while the key is being derived', async () => {
      expect(store.isWorking()).toBe(false);

      const running = store.unlock('a passphrase');
      expect(store.isWorking()).toBe(true);

      await running;
      expect(store.isWorking()).toBe(false);
    });

    it('stops working even when the attempt failed', async () => {
      repository.failNext = REFUSED;

      await store.unlock('not it');

      expect(store.isWorking()).toBe(false);
    });
  });

  describe('a library that will not open', () => {
    const DAMAGED = new IpcError('unlock_vault', {
      code: 'libraryDamaged',
      params: {},
      detail: 'the library is damaged',
    });

    /**
     * Neither a refusal nor a failure: retyping the passphrase cannot help, so the
     * screen has to stop offering the field and offer a way out instead.
     */
    it('is its own state, not a refused passphrase', async () => {
      repository.failNext = DAMAGED;

      expect(await store.unlock('a passphrase')).toBe(false);
      expect(store.damaged()).toBe(true);
      expect(store.refused()).toBe(false);
      expect(notifier.notice()).toBeNull();
    });

    it('moves it aside and says where it went', async () => {
      repository.failNext = DAMAGED;
      await store.unlock('a passphrase');

      expect(await store.setAsideDamagedLibrary()).toBe(true);

      expect(repository.setAside).toHaveLength(1);
      expect(store.damaged()).toBe(false);
      expect(TestBed.inject(StatusNotifier).status()?.key).toBe('vault.setAside');
      expect(TestBed.inject(StatusNotifier).status()?.params?.['path']).toBe(repository.setAside[0]);
    });

    /**
     * Without this the fresh library opens on a canvas with no space — and a note
     * cannot be created without one, so the application comes back working and unusable.
     */
    it('forgets the samples marker, so the fresh library seeds like a first launch', async () => {
      const preferences = TestBed.inject(LibraryPreferencesService);
      preferences.write(SEEDED_KEY, 'true');

      await store.setAsideDamagedLibrary();

      expect(preferences.read(SEEDED_KEY)).toBeNull();
    });

    it('reports a failure and leaves the library where it was', async () => {
      repository.failNext = DAMAGED;
      await store.unlock('a passphrase');
      repository.failNext = new IpcError('set_aside_damaged_library', {
        code: 'fileAccess',
        params: {},
        detail: 'in use',
      });

      expect(await store.setAsideDamagedLibrary()).toBe(false);

      // A cause the back end named wins over the action; 'errors.setAsideFailed' is
      // what a failure with nothing to say falls back to.
      expect(notifier.notice()?.ref.key).toBe('errors.fileAccess');
      expect(store.damaged()).toBe(true);
    });
  });

  describe('changing the passphrase', () => {
    it('hands both phrases over and leaves the session open', async () => {
      repository.answer = 'unlocked';
      await store.load();

      expect(await store.changePassphrase('the old one', 'a longer phrase')).toBe(true);

      expect(repository.changes).toEqual([{ current: 'the old one', next: 'a longer phrase' }]);
      expect(store.isUnlocked()).toBe(true);
    });

    /** Rewrapping the live key file alone revokes nothing: every copy kept its own. */
    it('says the retired phrase no longer opens the copies kept beside the library', async () => {
      repository.rewrapped = { backupsRewrapped: 3, backupsLeft: 0 };

      await store.changePassphrase('the old one', 'a longer phrase');

      expect(TestBed.inject(StatusNotifier).status()?.key).toBe('settings.security.changed');
    });

    /** A copy it could not reach is the one thing the user has to be told about. */
    it('names the copies it could not rewrap rather than claiming a clean revocation', async () => {
      repository.rewrapped = { backupsRewrapped: 1, backupsLeft: 2 };

      await store.changePassphrase('the old one', 'a longer phrase');

      const said = TestBed.inject(StatusNotifier).status();
      expect(said?.key).toBe('settings.security.changedSomeLeft');
      expect(said?.params?.['count']).toBe(2);
    });

    /** French keeps the singular where a count would read wrong. */
    it('has a sentence of its own for a single copy left behind', async () => {
      repository.rewrapped = { backupsRewrapped: 0, backupsLeft: 1 };

      await store.changePassphrase('the old one', 'a longer phrase');

      expect(TestBed.inject(StatusNotifier).status()?.key).toBe('settings.security.changedOneLeft');
    });

    it('treats a refused current phrase as a refusal, not a failure', async () => {
      repository.failNext = REFUSED;

      expect(await store.changePassphrase('not the old one', 'a longer phrase')).toBe(false);
      expect(store.refused()).toBe(true);
      expect(notifier.notice()).toBeNull();
    });

    /** Its own message: "unlock failed" would be a lie about what was attempted. */
    it('reports anything else under its own name', async () => {
      repository.failNext = new IpcError('change_passphrase', {
        code: 'storage',
        params: {},
        detail: 'disk gone',
      });

      expect(await store.changePassphrase('the old one', 'a longer phrase')).toBe(false);
      expect(notifier.notice()?.ref.key).toBe('errors.passphraseChangeFailed');
    });
  });
});
