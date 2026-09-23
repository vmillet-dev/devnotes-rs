import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SettingsPageComponent } from './settings-page.component';

describe('SettingsPageComponent', () => {
  let fixture: ComponentFixture<SettingsPageComponent>;
  let settings: SettingsStore;
  let draft: SettingsDraftStore;
  function toggle(id: string): HTMLInputElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SettingsPageComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    draft = TestBed.inject(SettingsDraftStore);
    draft.cancel();
    fixture = TestBed.createComponent(SettingsPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows the four groups left once the keys and the library have pages of their own', () => {
    const titles = [...fixture.nativeElement.querySelectorAll('.setting-group-title')].map(
      (title: HTMLElement) => title.textContent?.trim(),
    );

    expect(titles).toEqual(['Apparence', 'Comportement', 'Collage rapide', 'Notifications']);
  });

  /**
   * ⚠️ A menu where the list can grow and a segmented control where it is three. A native
   * `<select>` brought the operating system's border, arrow, focus ring and — on Windows —
   * its font into the middle of an application that draws all of its own surfaces.
   */
  async function openLocaleMenu(): Promise<HTMLElement[]> {
    fixture.nativeElement.querySelector('[data-testid="choice-setting-locale"]').click();
    await fixture.whenStable();
    return [
      ...fixture.nativeElement.querySelectorAll(
        '[data-testid="choice-panel-setting-locale"] [data-option-id]',
      ),
    ];
  }

  /** ⚠️ By `data-testid`, never by the translated `aria-label`. */
  function segments(kind: string): HTMLElement[] {
    const root = fixture.nativeElement as HTMLElement;
    return [
      ...root.querySelectorAll<HTMLElement>(`[data-testid="segmented-${kind}"] [data-testid="segment"]`),
    ];
  }

  it('offers the language alongside the titlebar buttons, system included', async () => {
    const options = await openLocaleMenu();

    expect(options.map((option) => option.getAttribute('data-option-id'))).toEqual(['system', 'fr', 'en']);
  });

  /** ⚠️ Into the draft, not the file: nothing is written before Appliquer or OK. */
  it('stages a chosen language rather than writing it', async () => {
    const options = await openLocaleMenu();

    options.find((option) => option.getAttribute('data-option-id') === 'en')!.click();
    await fixture.whenStable();

    expect(draft.value('locale')).toBe('en');
  });

  /** ⚠️ Exactly one is chosen at all times, which is what `aria-checked` has to say. */
  it('shows the active theme as the checked segment', () => {
    const checked = segments('setting-theme').filter(
      (segment) => segment.getAttribute('aria-checked') === 'true',
    );

    expect(checked).toHaveLength(1);
    expect(checked[0].getAttribute('data-segment-id')).toBe('system');
  });

  /** The language is changed from this very panel, which stays open when it applies. */
  it('redraws its choices in a language applied while it is open', async () => {
    const locale = (): string =>
      fixture.nativeElement
        .querySelector('[data-testid="choice-setting-locale"] .choice-name')
        .textContent.trim();
    const themes = (): string[] => segments('setting-theme').map((segment) => segment.textContent!.trim());
    expect(locale()).toBe('Système');
    expect(themes()).toEqual(['Système', 'Sombre', 'Clair']);

    TestBed.inject(TranslocoService).setActiveLang('en');
    await fixture.whenStable();

    expect(locale()).toBe('System');
    expect(themes()).toEqual(['System', 'Dark', 'Light']);
  });

  it('stages a chosen theme, with nothing to validate', async () => {
    segments('setting-theme')
      .find((segment) => segment.getAttribute('data-segment-id') === 'light')!
      .click();
    await fixture.whenStable();

    expect(draft.value('theme')).toBe('light');
  });

  it('stages a chosen density the same way', async () => {
    segments('setting-density')
      .find((segment) => segment.getAttribute('data-segment-id') === 'compact')!
      .click();
    await fixture.whenStable();

    expect(draft.value('density')).toBe('compact');
  });

  it('binds every toggle to its setting', async () => {
    toggle('setting-close-to-tray').click();
    toggle('setting-pinned-first').click();
    toggle('setting-copy-confirmation').click();
    await fixture.whenStable();

    expect(draft.value('closeToTray')).toBe(false);
    expect(draft.value('showPinnedFirst')).toBe(false);
    expect(draft.value('copyConfirmation')).toBe(false);
  });

  describe('the update entry', () => {
    const note = (): HTMLElement | null =>
      fixture.nativeElement.querySelector('[data-testid="setting-skipped-update"]');

    it('says nothing while nothing has been silenced', () => {
      expect(note()).toBeNull();
    });

    it('names the silenced version, so the row has a subject', async () => {
      settings.setSkippedUpdate('0.1.5');
      await fixture.whenStable();

      expect(note()?.textContent).toContain('0.1.5');
    });

    it('takes the skip back without waiting for the next release', async () => {
      settings.setSkippedUpdate('0.1.5');
      await fixture.whenStable();

      note()?.querySelector('button')?.click();
      await fixture.whenStable();

      expect(draft.value('skippedUpdate')).toBe('');
      expect(note()).toBeNull();
    });

    /** Asking to be told about updates is exactly what taking a skip back means. */
    it('forgets the skipped version when notifications are turned back on', async () => {
      settings.setSkippedUpdate('0.1.5');
      toggle('setting-update-notifications').click();
      await fixture.whenStable();
      expect(draft.value('updateNotifications')).toBe(false);

      toggle('setting-update-notifications').click();
      await fixture.whenStable();

      expect(draft.value('updateNotifications')).toBe(true);
      expect(draft.value('skippedUpdate')).toBe('');
    });
  });
});
