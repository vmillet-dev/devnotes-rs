import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { RouterOutlet, provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorBannerComponent } from '@banners/error-banner/error-banner.component';
import { TitlebarComponent } from '@titlebar/titlebar.component';
import { UpdatePromptComponent } from '@banners/update-prompt/update-prompt.component';
import { provideAppTesting } from '@testing/testing.providers';
import { VaultGateComponent } from './vault-gate/vault-gate.component';
import { VaultStore } from '@core/state/vault.store';
import { VaultRepository } from '@core/data/vault.repository';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { AppComponent } from './app.component';

describe('AppComponent', () => {
  let fixture: ComponentFixture<AppComponent>;

  beforeEach(() => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [provideAppTesting(), provideRouter([])],
    });
    fixture = TestBed.createComponent(AppComponent);
    fixture.autoDetectChanges();
  });

  /** The shell asks Rust before the first render; a spec says what the answer was. */
  async function withVault(state: 'absent' | 'locked' | 'unlocked'): Promise<void> {
    (TestBed.inject(VaultRepository) as unknown as FakeVaultRepository).answer = state;
    await TestBed.inject(VaultStore).load();
    await fixture.whenStable();
  }

  it('renders the persistent chrome: titlebar, global error banner and update prompt', () => {
    expect(fixture.debugElement.query(By.directive(TitlebarComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(ErrorBannerComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(UpdatePromptComponent))).not.toBeNull();
  });

  it('hosts features through the router outlet rather than importing them directly', async () => {
    await withVault('unlocked');

    expect(fixture.debugElement.query(By.directive(RouterOutlet))).not.toBeNull();
  });

  /**
   * The outlet is not merely hidden while the library is locked: it is not created.
   * The canvas queries notes the moment it mounts, and there would be nothing to answer.
   */
  it('puts the gate in front of the outlet while the library is locked', async () => {
    await withVault('locked');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(RouterOutlet))).toBeNull();
  });

  /** A library that has never been encrypted asks for a passphrase to be chosen. */
  it('asks for a passphrase to be created on a library that has none', async () => {
    await withVault('absent');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
  });

  /** Before the answer lands, neither: the shell would flash a screen it is replacing. */
  it('renders neither until Rust has answered', () => {
    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).toBeNull();
    expect(fixture.debugElement.query(By.directive(RouterOutlet))).toBeNull();
  });
});
