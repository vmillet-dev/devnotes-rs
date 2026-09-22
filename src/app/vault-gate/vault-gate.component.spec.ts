import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { VaultRepository } from '@core/data/vault.repository';
import { LibrariesStore } from '@core/state/libraries.store';
import { VaultStore } from '@core/state/vault.store';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeLibrariesRepository } from '@testing/fake-libraries-repository';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { VaultGateComponent } from './vault-gate.component';

describe('VaultGateComponent', () => {
  let fixture: ComponentFixture<VaultGateComponent>;
  let repository: FakeVaultRepository;
  let store: VaultStore;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [VaultGateComponent],
      providers: [provideAppTesting()],
    });
    repository = TestBed.inject(VaultRepository) as unknown as FakeVaultRepository;
    store = TestBed.inject(VaultStore);
    fixture = TestBed.createComponent(VaultGateComponent);
    fixture.autoDetectChanges();
  });

  function field(hook: string): HTMLInputElement | null {
    return fixture.debugElement.query(By.css(`[data-testid="${hook}"]`))?.nativeElement ?? null;
  }

  function submitButton(): HTMLButtonElement {
    return fixture.debugElement.query(By.css('[data-testid="vault-submit"]')).nativeElement;
  }

  function problem(): string {
    return (
      fixture.debugElement.query(By.css('[data-testid="vault-problem"]')).nativeElement.textContent?.trim() ??
      ''
    );
  }

  async function type(hook: string, value: string): Promise<void> {
    const input = field(hook);
    if (input === null) throw new Error(`no field "${hook}"`);
    input.value = value;
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  async function open(state: 'absent' | 'locked'): Promise<void> {
    repository.answer = state;
    await store.load();
    await fixture.whenStable();
  }

  describe('a library that will not open', () => {
    beforeEach(async () => {
      await open('locked');
      repository.failNext = new IpcError('unlock_vault', {
        code: 'libraryDamaged',
        params: {},
        detail: 'damaged',
      });
      await type('vault-passphrase', 'an end-to-end passphrase');
      submitButton().click();
      await fixture.whenStable();
    });

    /** ⚠️ The form would be an invitation to do the one thing that cannot work. */
    it('replaces the passphrase form with a way out', () => {
      expect(field('vault-passphrase')).toBeNull();
      expect(fixture.debugElement.query(By.css('[data-testid="vault-damaged"]'))).not.toBeNull();
    });

    it('says what will happen to the damaged library before it happens', () => {
      const panel = fixture.debugElement.query(By.css('[data-testid="vault-damaged"]'))
        .nativeElement as HTMLElement;

      expect(panel.textContent).toContain('pièces jointes');
      expect(panel.textContent).toContain('phrase de passe ne change pas');
    });

    it('sets the library aside and comes back to the passphrase', async () => {
      fixture.debugElement.query(By.css('[data-testid="vault-set-aside"]')).nativeElement.click();
      await fixture.whenStable();

      expect(repository.setAside).toHaveLength(1);
      expect(field('vault-passphrase')).not.toBeNull();
    });
  });

  describe('unlocking an existing library', () => {
    beforeEach(async () => {
      await open('locked');
    });

    it('asks for one passphrase and no confirmation', () => {
      expect(field('vault-passphrase')).not.toBeNull();
      expect(field('vault-confirmation')).toBeNull();
    });

    it('will not submit an empty field', () => {
      expect(submitButton().disabled).toBe(true);
    });

    /**
     * ⚠️ Said before the round trip: deriving takes 224 ms, and answering "too short"
     * after it reads as the application thinking about it.
     */
    it('says a passphrase is too short without asking the back end', async () => {
      await type('vault-passphrase', 'short');

      expect(problem()).toContain('8');
      expect(submitButton().disabled).toBe(true);
      expect(repository.passphrases).toEqual([]);
    });

    it('hands the passphrase over once it is long enough', async () => {
      await type('vault-passphrase', 'correct horse');
      submitButton().click();
      await fixture.whenStable();

      expect(repository.passphrases).toEqual(['correct horse']);
    });

    /**
     * ⚠️ Whatever happens, success included: a passphrase left in a DOM node is a
     * passphrase in a memory dump.
     */
    it('clears the field as soon as it has been sent', async () => {
      await type('vault-passphrase', 'correct horse');
      submitButton().click();
      await fixture.whenStable();

      expect(field('vault-passphrase')?.value).toBe('');
    });

    /** A typo belongs beside the field that caused it, not in the global error banner. */
    it('says a refused passphrase in place rather than through the banner', async () => {
      repository.failNext = new IpcError('unlock_vault', {
        code: 'wrongPassphrase',
        params: {},
        detail: 'Wrong passphrase',
      });

      await type('vault-passphrase', 'battery staple');
      submitButton().click();
      await fixture.whenStable();

      expect(store.refused()).toBe(true);
      expect(problem()).not.toBe('');
    });

    it('withdraws the refusal as soon as the field is touched again', async () => {
      repository.failNext = new IpcError('unlock_vault', {
        code: 'wrongPassphrase',
        params: {},
        detail: 'Wrong passphrase',
      });
      await type('vault-passphrase', 'battery staple');
      submitButton().click();
      await fixture.whenStable();

      await type('vault-passphrase', 'b');

      expect(store.refused()).toBe(false);
    });
  });

  describe('protecting a library that has never been encrypted', () => {
    beforeEach(async () => {
      await open('absent');
    });

    it('asks for the passphrase twice', () => {
      expect(field('vault-passphrase')).not.toBeNull();
      expect(field('vault-confirmation')).not.toBeNull();
    });

    it('refuses to submit while the two entries differ', async () => {
      await type('vault-passphrase', 'correct horse');
      await type('vault-confirmation', 'correct hors');

      expect(submitButton().disabled).toBe(true);
      expect(problem()).not.toBe('');
    });

    it('creates once both entries agree', async () => {
      await type('vault-passphrase', 'correct horse');
      await type('vault-confirmation', 'correct horse');
      submitButton().click();
      await fixture.whenStable();

      expect(repository.passphrases).toEqual(['correct horse']);
      expect(store.isUnlocked()).toBe(true);
    });

    /** It cannot be recovered, and this is the only moment that can still be acted on. */
    it('says plainly that a lost passphrase is a lost library', () => {
      const warning = fixture.debugElement.query(By.css('.warning'));

      expect(warning).not.toBeNull();
      expect(warning.nativeElement.textContent.trim()).not.toBe('');
    });
  });
  /**
   * ⚠️ A forgotten passphrase is final by design, and the gate used to offer nothing but
   * the field: the only way past was finding the profile directory and moving files by
   * hand, which is not something an application should require anyone to know.
   */
  describe('a passphrase nobody remembers', () => {
    const click = async (hook: string): Promise<void> => {
      (
        fixture.debugElement.query(By.css(`[data-testid="${hook}"]`)).nativeElement as HTMLButtonElement
      ).click();
      await fixture.whenStable();
    };

    const panel = (): HTMLElement | null =>
      fixture.debugElement.query(By.css('[data-testid="vault-archive"]'))?.nativeElement ?? null;

    beforeEach(async () => {
      repository.answer = 'locked';
      await store.load();
      await fixture.whenStable();
    });

    /** ⚠️ A fresh library has no phrase to have forgotten. */
    it('offers the way out only on a library that already exists', async () => {
      expect(fixture.debugElement.query(By.css('[data-testid="vault-forgotten"]'))).not.toBeNull();

      repository.answer = 'absent';
      await store.load();
      await fixture.whenStable();

      expect(fixture.debugElement.query(By.css('[data-testid="vault-forgotten"]'))).toBeNull();
    });

    /** ⚠️ Instead of the form, never beside it: this reads as giving up, not a shortcut. */
    it('replaces the field with what would be lost, rather than acting', async () => {
      await click('vault-forgotten');

      expect(panel()).not.toBeNull();
      expect(field('vault-passphrase')).toBeNull();
      expect(repository.archived).toEqual([]);
    });

    it('names what leaves and what does not come back', async () => {
      await click('vault-forgotten');

      const text = panel()!.textContent ?? '';
      expect(text).toContain('pas récupérées');
      expect(text).toContain('scellées');
    });

    it('goes back to the field without touching anything', async () => {
      await click('vault-forgotten');

      await click('vault-keep-trying');

      expect(panel()).toBeNull();
      expect(field('vault-passphrase')).not.toBeNull();
      expect(repository.archived).toEqual([]);
    });

    /**
     * ⚠️ The key file travels, so what is left has no library at all — `absent`, not
     * `locked`, which is what turns the gate into the one that asks for a new phrase.
     */
    it('archives the library and comes back asking for a new phrase', async () => {
      await click('vault-forgotten');

      await click('vault-archive-confirm');

      expect(repository.archived).toHaveLength(1);
      expect(store.needsCreating()).toBe(true);
      expect(field('vault-confirmation')).not.toBeNull();
    });

    it('stays on the panel when the move was refused', async () => {
      await click('vault-forgotten');
      repository.failNext = new IpcError('archive_locked_library', new Error('locked file'));

      await click('vault-archive-confirm');

      expect(panel()).not.toBeNull();
      expect(store.needsCreating()).toBe(false);
    });
  });

  /**
   * ⚠️ The File menu, where the libraries live, does not exist until one is open: the gate
   * is the only place a user holding several can say which one they have the phrase for.
   */
  describe('which library it asks for', () => {
    async function withLibraries(names: readonly string[]): Promise<FakeLibrariesRepository> {
      const libraries = new FakeLibrariesRepository(names);
      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        imports: [VaultGateComponent],
        providers: [provideAppTesting({ librariesRepository: libraries, appWindow })],
      });
      repository = TestBed.inject(VaultRepository) as unknown as FakeVaultRepository;
      store = TestBed.inject(VaultStore);
      await TestBed.inject(LibrariesStore).load();
      fixture = TestBed.createComponent(VaultGateComponent);
      fixture.autoDetectChanges();
      await open('locked');

      return libraries;
    }

    let appWindow: FakeAppWindow;

    function line(): HTMLElement | null {
      return fixture.debugElement.query(By.css('[data-testid="vault-library"]'))?.nativeElement ?? null;
    }

    beforeEach(() => {
      appWindow = new FakeAppWindow();
    });

    it('names the one it opens', async () => {
      await withLibraries(['Boulot']);

      expect(line()?.textContent).toContain('Boulot');
      expect(field('choice-vault-library')).toBeNull();
    });

    /** "Library: Library" would be a line that says nothing. */
    it('says nothing about the one library that was never named', async () => {
      await withLibraries(['']);

      expect(line()).toBeNull();
    });

    it('offers the others when there are several, and opens the one chosen', async () => {
      const libraries = await withLibraries(['Notes', 'Boulot']);

      (field('choice-vault-library') as unknown as HTMLButtonElement).click();
      await fixture.whenStable();
      const other = fixture.debugElement.query(By.css('[data-option-id="lib-1"]'))
        .nativeElement as HTMLElement;
      other.click();
      await fixture.whenStable();

      expect((await libraries.list()).open).toBe('lib-1');
      expect(appWindow.reloaded).toBe(1);
    });

    /** ⚠️ Archiving "the library" without naming which is the worst place to be vague. */
    it('names it on the way out too', async () => {
      await withLibraries(['Boulot']);

      (field('vault-forgotten') as unknown as HTMLButtonElement).click();
      await fixture.whenStable();

      const archive = fixture.debugElement.query(By.css('[data-testid="vault-archive"]'))
        .nativeElement as HTMLElement;
      expect(archive.textContent).toContain('Boulot');
    });
  });
});
