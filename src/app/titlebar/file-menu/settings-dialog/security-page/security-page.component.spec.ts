import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SecurityPageComponent } from './security-page.component';

describe('SecurityPageComponent', () => {
  let fixture: ComponentFixture<SecurityPageComponent>;
  let settings: SettingsStore;
  let draft: SettingsDraftStore;

  const backups = (): HTMLInputElement =>
    fixture.nativeElement.querySelector('[data-testid="setting-automatic-backups"]');

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SecurityPageComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    draft = TestBed.inject(SettingsDraftStore);
    draft.cancel();
    fixture = TestBed.createComponent(SecurityPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** The dialog is opened from here and nowhere else; what it does is its own spec. */
  it('opens the passphrase dialog, and only on demand', async () => {
    const opener = (): HTMLButtonElement =>
      fixture.nativeElement.querySelector('[data-testid="setting-change-passphrase"]');
    const dialog = (): HTMLElement | null =>
      fixture.nativeElement.querySelector('[data-testid="change-passphrase"]');

    expect(dialog()).toBeNull();

    opener().click();
    await fixture.whenStable();

    expect(dialog()).not.toBeNull();
  });

  /**
   * ⚠️ Rust reads this at launch, before the front end exists — turning it off has to
   * reach the preferences file, which is the only thing the back end sees. Which is
   * exactly why it waits for Appliquer: until then the switch is a draft.
   */
  it('shows the copies as on, and stages the choice until it is applied', async () => {
    expect(backups().checked).toBe(true);

    backups().click();
    await fixture.whenStable();

    expect(backups().checked).toBe(false);
    expect(draft.value('automaticBackups')).toBe(false);
    expect(settings.automaticBackups()).toBe(true);

    draft.apply();

    expect(settings.automaticBackups()).toBe(false);
  });

  /**
   * ⚠️ The two together, which is the whole of this page: the phrase unwraps the key and
   * the copies are wrapped under it too, so a page carrying only one of them would let
   * someone change the phrase without meeting what the change reaches.
   */
  it('names the copies beside the phrase, not somewhere else', () => {
    const titles = [...fixture.nativeElement.querySelectorAll('.setting-group-title')].map(
      (title: HTMLElement) => title.textContent?.trim(),
    );

    expect(titles).toEqual(['Accès', 'Copies de sauvegarde']);
  });
});
