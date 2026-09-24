import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { VaultRepository } from '@core/data/vault.repository';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { ChangePassphraseDialogComponent } from './change-passphrase-dialog.component';

const REFUSED = new IpcError('change_passphrase', {
  code: 'wrongPassphrase',
  params: {},
  detail: 'Wrong passphrase',
});

describe('ChangePassphraseDialogComponent', () => {
  let fixture: ComponentFixture<ChangePassphraseDialogComponent>;
  let repository: FakeVaultRepository;
  let status: StatusNotifier;
  let closed: number;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ChangePassphraseDialogComponent],
      providers: [provideAppTesting()],
    });
    repository = TestBed.inject(VaultRepository) as unknown as FakeVaultRepository;
    status = TestBed.inject(StatusNotifier);
    fixture = TestBed.createComponent(ChangePassphraseDialogComponent);
    closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));
    fixture.autoDetectChanges();
  });

  function element(hook: string): HTMLElement {
    return fixture.debugElement.query(By.css(`[data-testid="${hook}"]`)).nativeElement;
  }

  function submitButton(): HTMLButtonElement {
    return element('change-passphrase-submit') as HTMLButtonElement;
  }

  async function type(hook: string, value: string): Promise<void> {
    const input = element(hook) as HTMLInputElement;
    input.value = value;
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  async function fill(current: string, next: string, confirmation = next): Promise<void> {
    await type('change-passphrase-current', current);
    await type('change-passphrase-next', next);
    await type('change-passphrase-confirmation', confirmation);
  }

  it('will not submit without the current phrase', async () => {
    await fill('', 'a longer phrase');

    expect(submitButton().disabled).toBe(true);
  });

  /** The same floor the gate holds, said before the round trip rather than after it. */
  it('refuses a new phrase too short to be worth deriving', async () => {
    await fill('the old one', 'short');

    expect(submitButton().disabled).toBe(true);
    expect(element('change-passphrase-problem').textContent?.trim()).not.toBe('');
  });

  it('refuses two entries that differ', async () => {
    await fill('the old one', 'a longer phrase', 'a longer phrasr');

    expect(submitButton().disabled).toBe(true);
  });

  it('hands both phrases over, says so, and closes', async () => {
    await fill('the old one', 'a longer phrase');
    submitButton().click();
    await fixture.whenStable();

    expect(repository.changes).toEqual([{ current: 'the old one', next: 'a longer phrase' }]);
    expect(status.status()?.key).toBe('settings.security.changed');
    expect(closed).toBe(1);
  });

  /** A phrase left in a DOM node outlives the dialog that carried it. */
  it('clears the three fields whatever the answer', async () => {
    repository.failNext = REFUSED;
    await fill('not the old one', 'a longer phrase');
    submitButton().click();
    await fixture.whenStable();

    expect((element('change-passphrase-current') as HTMLInputElement).value).toBe('');
    expect((element('change-passphrase-next') as HTMLInputElement).value).toBe('');
    expect((element('change-passphrase-confirmation') as HTMLInputElement).value).toBe('');
  });

  /** A refused current phrase belongs beside the field, and the dialog stays. */
  it('says the current phrase was refused rather than closing on it', async () => {
    repository.failNext = REFUSED;
    await fill('not the old one', 'a longer phrase');
    submitButton().click();
    await fixture.whenStable();

    expect(element('change-passphrase-problem').textContent?.trim()).not.toBe('');
    expect(status.status()).toBeNull();
    expect(closed).toBe(0);
  });

  it('withdraws the refusal as soon as the field is touched again', async () => {
    repository.failNext = REFUSED;
    await fill('not the old one', 'a longer phrase');
    submitButton().click();
    await fixture.whenStable();

    await type('change-passphrase-current', 'another try');

    expect(element('change-passphrase-problem').textContent?.trim()).toBe('');
  });

  it('closes without changing anything when cancelled', async () => {
    await fill('the old one', 'a longer phrase');
    (element('change-passphrase-cancel') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(repository.changes).toEqual([]);
    expect(closed).toBe(1);
  });
});
