import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { FakeLibrariesRepository } from '@testing/fake-libraries-repository';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { LibrariesStore } from './libraries.store';
import { VaultStore } from './vault.store';

describe('LibrariesStore', () => {
  let store: LibrariesStore;
  let repository: FakeLibrariesRepository;
  let vault: FakeVaultRepository;

  function configure(names: readonly string[]): void {
    TestBed.resetTestingModule();
    repository = new FakeLibrariesRepository(names);
    vault = new FakeVaultRepository();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ librariesRepository: repository, vaultRepository: vault })],
    });
    store = TestBed.inject(LibrariesStore);
  }

  beforeEach(() => {
    configure(['Notes', 'Perso']);
  });

  it('reads the registry and says which one is open', async () => {
    await store.load();

    expect(store.libraries()).toHaveLength(2);
    expect(store.open()?.name).toBe('Notes');
    expect(store.hasSeveral()).toBe(true);
  });

  /**
   * ⚠️ Both, always together: the preference file's path comes from the entry, so a
   * registry read without the hydrate leaves the samples marker and the per-space views
   * pointing at whichever library was open last.
   */
  it('opens the library own preference file along with the registry', async () => {
    const preferences = TestBed.inject(LibraryPreferencesService);
    preferences.write('devnotes.notes.samplesSeeded', 'true');

    await store.load();

    // Re-hydrating cleared the cache the previous library had filled.
    expect(preferences.read('devnotes.notes.samplesSeeded')).toBeNull();
  });

  /**
   * ⚠️ One gesture rather than two: you have just named it, so you want to be in it. The
   * gate then asks for a phrase, which is what a first launch does too.
   */
  it('creates and opens in one gesture', async () => {
    await store.load();

    const created = await store.create('Boulot');

    expect(created?.name).toBe('Boulot');
    expect(store.open()?.name).toBe('Boulot');
  });

  /**
   * ⚠️ The connection closes, every command answers `Locked`, and the shell goes back to
   * the gate: the other library has its own passphrase, and asking for it is the only
   * proof the right one is open.
   */
  it('sends the shell back to the gate on a switch', async () => {
    const vaultStore = TestBed.inject(VaultStore);
    await vaultStore.load();
    expect(vaultStore.isUnlocked()).toBe(true);
    await store.load();
    vault.answer = 'locked';

    await store.openLibrary('lib-1');

    expect(store.open()?.name).toBe('Perso');
    expect(vaultStore.isUnlocked()).toBe(false);
  });

  it('does nothing when asked for the library already open', async () => {
    await store.load();

    await store.openLibrary('lib-0');

    expect(store.open()?.name).toBe('Notes');
  });

  it('renames one without touching which is open', async () => {
    await store.load();

    await store.rename('lib-1', 'Archives');

    expect(store.libraries()[1].name).toBe('Archives');
    expect(store.open()?.id).toBe('lib-0');
  });

  describe('deleting one', () => {
    /** ⚠️ Named before it runs, like emptying the trash: it takes everything at once. */
    it('proposes rather than erasing', async () => {
      await store.load();

      store.askToDelete(store.libraries()[1]);

      expect(store.pendingDeletion()?.entry.id).toBe('lib-1');
      expect(store.libraries()).toHaveLength(2);
    });

    /** ⚠️ Deleting files under a live connection takes the process down with them. */
    it('refuses the one that is open', async () => {
      await store.load();

      store.askToDelete(store.libraries()[0]);

      expect(store.pendingDeletion()).toBeNull();
    });

    /** ⚠️ The gate would have nothing to offer, and the registry would adopt an empty
     *  profile as a library nobody asked for. */
    it('refuses the last one', async () => {
      configure(['Notes']);
      await store.load();

      store.askToDelete(store.libraries()[0]);

      expect(store.pendingDeletion()).toBeNull();
    });

    it('erases only on the confirm, and only what was proposed', async () => {
      await store.load();
      store.askToDelete(store.libraries()[1]);

      await store.confirmDeletion();

      expect(store.libraries()).toHaveLength(1);
      expect(store.libraries()[0].id).toBe('lib-0');
      expect(store.pendingDeletion()).toBeNull();
    });

    it('gives up on the proposal without erasing anything', async () => {
      await store.load();
      store.askToDelete(store.libraries()[1]);

      store.dismissDeletion();
      await store.confirmDeletion();

      expect(store.libraries()).toHaveLength(2);
    });
  });
});
