import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SettingsPageComponent } from './settings-page.component';

describe('SettingsPageComponent', () => {
  let fixture: ComponentFixture<SettingsPageComponent>;
  let settings: SettingsStore;
  function toggle(id: string): HTMLInputElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  function shortcutField(): HTMLInputElement {
    return fixture.nativeElement.querySelector('#setting-shortcut');
  }

  function press(init: KeyboardEventInit): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { ...init, bubbles: true, cancelable: true });
    shortcutField().dispatchEvent(event);
    return event;
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SettingsPageComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    fixture = TestBed.createComponent(SettingsPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows the five groups the panel is made of', () => {
    const titles = [...fixture.nativeElement.querySelectorAll('.setting-group-title')].map(
      (title: HTMLElement) => title.textContent?.trim(),
    );

    expect(titles).toEqual(['Apparence', 'Comportement', 'Collage rapide', 'Sécurité', 'Notifications']);
  });

  /** The dialog is opened from here and nowhere else; what it does is its own spec. */
  it('opens the passphrase dialog from the security group, and only on demand', async () => {
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

  it('writes a chosen language straight through', async () => {
    const options = await openLocaleMenu();

    options.find((option) => option.getAttribute('data-option-id') === 'en')!.click();
    await fixture.whenStable();

    expect(settings.locale()).toBe('en');
  });

  /** ⚠️ Exactly one is chosen at all times, which is what `aria-checked` has to say. */
  it('shows the active theme as the checked segment', () => {
    const checked = segments('setting-theme').filter(
      (segment) => segment.getAttribute('aria-checked') === 'true',
    );

    expect(checked).toHaveLength(1);
    expect(checked[0].getAttribute('data-segment-id')).toBe('system');
  });

  it('writes a chosen theme straight through, with nothing to validate', async () => {
    segments('setting-theme')
      .find((segment) => segment.getAttribute('data-segment-id') === 'light')!
      .click();
    await fixture.whenStable();

    expect(settings.theme()).toBe('light');
  });

  it('writes a chosen density the same way', async () => {
    segments('setting-density')
      .find((segment) => segment.getAttribute('data-segment-id') === 'compact')!
      .click();
    await fixture.whenStable();

    expect(settings.density()).toBe('compact');
  });

  it('binds every toggle to its setting', async () => {
    toggle('setting-close-to-tray').click();
    toggle('setting-pinned-first').click();
    toggle('setting-copy-confirmation').click();
    toggle('setting-automatic-backups').click();
    await fixture.whenStable();

    expect(settings.closeToTray()).toBe(false);
    expect(settings.showPinnedFirst()).toBe(false);
    expect(settings.copyConfirmation()).toBe(false);
    expect(settings.automaticBackups()).toBe(false);
  });

  /** ⚠️ Rust reads this at launch, before the front end exists — turning it off has to
   *  reach the preferences file, which is the only thing the back end sees. */
  it('shows the copies as on, and writes the choice through', async () => {
    expect(toggle('setting-automatic-backups').checked).toBe(true);

    toggle('setting-automatic-backups').click();
    await fixture.whenStable();

    expect(settings.automaticBackups()).toBe(false);
    expect(toggle('setting-automatic-backups').checked).toBe(false);
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

      expect(settings.skippedUpdate()).toBe('');
      expect(note()).toBeNull();
    });

    /** Asking to be told about updates is exactly what taking a skip back means. */
    it('forgets the skipped version when notifications are turned back on', async () => {
      settings.setSkippedUpdate('0.1.5');
      toggle('setting-update-notifications').click();
      await fixture.whenStable();
      expect(settings.updateNotifications()).toBe(false);

      toggle('setting-update-notifications').click();
      await fixture.whenStable();

      expect(settings.updateNotifications()).toBe(true);
      expect(settings.skippedUpdate()).toBe('');
    });
  });

  it('records a shortcut from the keystroke rather than from typed text', async () => {
    press({ code: 'KeyK', ctrlKey: true, shiftKey: true });
    await fixture.whenStable();

    expect(settings.paletteShortcut()).toBe('Ctrl+Shift+K');
    expect(shortcutField().value).toBe('Ctrl+Shift+K');
  });

  it('lets a bare keystroke through, which is what keeps Tab and Escape working', () => {
    const event = press({ code: 'Tab' });

    expect(event.defaultPrevented).toBe(false);
    expect(settings.paletteShortcut()).toBe('Ctrl+Alt+P');
  });

  it('offers to restore the original combination, and only once it changed', async () => {
    const reset = (): HTMLButtonElement => fixture.nativeElement.querySelector('.setting-shortcut-reset');
    expect(reset().disabled).toBe(true);

    press({ code: 'KeyK', ctrlKey: true, shiftKey: true });
    await fixture.whenStable();
    reset().click();
    await fixture.whenStable();

    expect(settings.paletteShortcut()).toBe('Ctrl+Alt+P');
  });
});
