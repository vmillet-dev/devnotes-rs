import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { fileMenu, settings, titlebar } from '../pageobjects/titlebar.page.js';
import { cursorOf, eventually, press, reopenSession } from '../support/app.js';

/**
 * A preference applies as it is typed, one key at a time.
 *
 * ⚠️ What this file does not prove is that any of it reached the disk: `reopenSession()`
 * opens a new session against the same living process, so the store plugin's in-memory
 * map is still the one answering. `15-preferences-on-disk` reads the file itself.
 */
describe('Preferences', () => {
  before(canvas.open);

  it('opens from the File menu', async () => {
    await fileMenu.openPreferences();
    expect(await settings.page('general').isExisting()).toBe(true);
  });

  it('applies the theme as it is chosen, with no confirmation step', async () => {
    await settings.setTheme('light');

    // Dark is the base because the preference lives in a file nothing can read before
    // Angular boots.
    expect(await browser.$('html').getAttribute('data-theme')).toBe('light');
  });

  it('applies the density the same way', async () => {
    await settings.setDensity('compact');
    expect(await browser.$('html').getAttribute('data-density')).toBe('compact');
  });

  it('switches the interface language from the panel', async () => {
    await settings.setLocale('en');
    expect(await titlebar.activeLocale()).toBe('en');
  });

  it('rebuilds all three from the store rather than from a signal it was holding', async () => {
    await settings.close();
    await reopenSession();

    // These came back because `SettingsStore` restored them, not because a signal survived.
    expect(await browser.$('html').getAttribute('data-theme')).toBe('light');
    expect(await browser.$('html').getAttribute('data-density')).toBe('compact');
    expect(await titlebar.activeLocale()).toBe('en');
  });

  it('captures a shortcut from the keyboard, and refuses one with no modifier', async () => {
    await fileMenu.openPreferences();
    const field = settings.shortcut();
    const before = await field.getValue();

    // ⚠️ A global accelerator without a modifier would swallow that key in every
    // application on the machine — which is also what leaves Tab and Escape working here.
    await field.click();
    await press('p');
    expect(await field.getValue()).toBe(before);

    // `KeyboardEvent.code`, so a combination set on AZERTY stays put on QWERTY.
    await press('j', ['Control', 'Alt']);
    expect(await field.getValue()).toBe('Ctrl+Alt+J');

    await settings.resetShortcut();
    expect(await field.getValue()).toBe(before);
    await settings.close();
  });

  // Readonly: a text cursor would promise the wrong interaction.
  it('leaves the shortcut field a cursor that does not invite typing', async () => {
    await fileMenu.openPreferences();
    expect(await cursorOf('#setting-shortcut')).not.toBe('text');
    await settings.close();
  });

  it('keeps the titlebar switch and the panel in agreement', async () => {
    await titlebar.setLocale('fr');
    await eventually(
      () => titlebar.activeLocale(),
      (locale) => locale === 'fr',
      'the titlebar to settle on the locale it was given',
    );
    await fileMenu.openPreferences();

    expect(await settings.locale()).toContain('Français');
    await settings.close();
  });
});
