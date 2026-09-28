import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorBannerComponent } from '@banners/error-banner/error-banner.component';
import { NotesPageComponent } from '@notes/notes-page.component';
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
      providers: [provideAppTesting()],
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

  it('shows the notes page once the library is unlocked', async () => {
    await withVault('unlocked');

    expect(fixture.debugElement.query(By.directive(NotesPageComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).toBeNull();
  });

  /**
   * The page is not merely hidden while the library is locked: it is not created.
   * The canvas queries notes the moment it mounts, and there would be nothing to answer.
   */
  it('puts the gate in front of the notes page while the library is locked', async () => {
    await withVault('locked');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(NotesPageComponent))).toBeNull();
  });

  /** A library that has never been encrypted asks for a passphrase to be chosen. */
  it('asks for a passphrase to be created on a library that has none', async () => {
    await withVault('absent');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
  });

  /** Before the answer lands, neither: the shell would flash a screen it is replacing. */
  it('renders neither until Rust has answered', () => {
    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).toBeNull();
    expect(fixture.debugElement.query(By.directive(NotesPageComponent))).toBeNull();
  });
});
