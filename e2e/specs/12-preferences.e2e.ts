import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { aboutMenu, fileMenu, settings, titlebar } from '../pageobjects/titlebar.page.js';
import { cursorOf, eventually, press, reopenSession, testid } from '../support/app.js';

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

  describe('the keys', () => {
    beforeEach(async () => {
      await fileMenu.openPreferences();
      await settings.open('shortcuts');
    });

    afterEach(settings.close);

    it('captures a global shortcut from the keyboard, and refuses one with no modifier', async () => {
      const field = settings.shortcut('palette');
      const before = await field.getValue();

      // ⚠️ A global accelerator without a modifier would swallow that key in every
      // application on the machine — which is also what leaves Tab and Escape working here.
      await field.click();
      await press('p');
      expect(await field.getValue()).toBe(before);
      expect(await settings.shortcutError('palette').isExisting()).toBe(true);

      // `KeyboardEvent.code`, so a combination set on AZERTY stays put on QWERTY.
      await press('j', ['Control', 'Alt']);
      expect(await field.getValue()).toBe('Ctrl+Alt+J');

      await settings.resetShortcut('palette');
      expect(await field.getValue()).toBe(before);
    });

    /**
     * ⚠️ The whole of #254: three keys out of twenty could be moved. A canvas key takes a
     * bare letter, where a global one may not — it answers only while the canvas has the
     * keyboard, so nothing outside the window is taken.
     */
    it('takes a bare letter on a canvas key, and the canvas then answers to it', async () => {
      const field = settings.shortcut('canvas.check');
      await field.click();
      await press('j');
      expect(await field.getValue()).toBe('J');
      await settings.close();

      // Walked to a known end rather than started from wherever: these files share one
      // session, so what holds the focus here is whatever ran before.
      const titles = await canvas.titles();
      for (const _ of titles) {
        await press('ArrowLeft');
      }
      const focused = await canvas.focusedCardTitle();
      expect(focused).not.toBeNull();

      await press('j');
      expect(
        await eventually(
          () => canvas.isChecked(focused!),
          (checked) => checked,
          'the moved key to tick the focused card',
        ),
      ).toBe(true);

      // Put back, so the files after this one meet the keys they were written against.
      await press('j');
      await fileMenu.openPreferences();
      await settings.open('shortcuts');
      await settings.resetShortcut('canvas.check');
      expect(await settings.shortcut('canvas.check').getValue()).toBe('X');
    });

    /** ⚠️ The second action would be unreachable, and nothing would say which. */
    it('refuses a keystroke another action already answers to', async () => {
      const field = settings.shortcut('canvas.pin');
      await field.click();
      await press('c');

      expect(await field.getValue()).toBe('P');
      expect(await settings.shortcutError('canvas.pin').getText()).toContain('C');
    });

    // Readonly: a text cursor would promise the wrong interaction.
    it('leaves a capture field a cursor that does not invite typing', async () => {
      expect(await cursorOf('[data-testid="shortcut-field"][data-action="palette"]')).not.toBe('text');
    });
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

  /**
   * ⚠️ The same signal the panel writes, so the two cannot disagree — and the panel keeps
   * its row: a setting that only exists in a corner of the titlebar is a setting nobody
   * finds twice. The theme was three gestures away where the language was one.
   */
  describe('the theme, from the titlebar', () => {
    /** ⚠️ One button that cycles, in `THEME_CHOICES` order: system, dark, light. */
    const cycle = () => $(testid('theme-cycle'));

    after(async () => {
      await titlebar.setTheme('system');
    });

    it('repaints the window without opening anything', async () => {
      await titlebar.setTheme('light');

      expect(
        await eventually(
          () => browser.execute(() => document.documentElement.getAttribute('data-theme')),
          (theme) => theme === 'light',
          'the titlebar control to repaint the window',
        ),
      ).toBe('light');
    });

    it('shows the theme in force on the one control there is', async () => {
      expect(await cycle().getAttribute('data-theme-choice')).toBe('light');
    });

    it('is the same choice the panel shows', async () => {
      await fileMenu.openPreferences();

      expect(await settings.theme()).toBe('light');
      await settings.close();
    });
  });

  /**
   * ⚠️ Ten chapters of prose in one scrolling panel was roughly 3 400 characters, all of it
   * true and none of it looked at — the worst return a help surface can have.
   */
  describe('the written guide', () => {
    after(async () => {
      await aboutMenu.close();
    });

    it('opens on one chapter, with a schematic of the screen it is about', async () => {
      await aboutMenu.openGettingStarted();

      expect(await aboutMenu.chapter()).toBe('notes');
      expect(await $$(testid('guide-chapter')).length).toBe(1);
      expect(await $('app-guide-figure svg').isExisting()).toBe(true);
    });

    it('walks to the next one', async () => {
      await aboutMenu.next();

      expect(await aboutMenu.chapter()).toBe('spaces');
    });

    /** A reader who knows which chapter they want must still land on it. */
    it('jumps straight to a chapter from its dot', async () => {
      await aboutMenu.jumpTo('transfer');

      expect(await aboutMenu.chapter()).toBe('transfer');
    });

    /** ⚠️ A walk you can only leave by finishing it is a wall with extra steps. */
    it('closes from any chapter', async () => {
      await aboutMenu.jumpTo('fields');
      await aboutMenu.close();

      expect(await aboutMenu.gettingStarted().isExisting()).toBe(false);
      await aboutMenu.openGettingStarted();
    });
  });
});
