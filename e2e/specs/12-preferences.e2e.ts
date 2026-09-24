import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { rail } from '../pageobjects/sidebar.page.js';
import { aboutMenu, fileMenu, settings, titlebar } from '../pageobjects/titlebar.page.js';
import { cursorOf, eventually, press, reopenSession, testid } from '../support/app.js';

/**
 * A preference applies on a button; only the theme and the density show while chosen.
 *
 * What this file does not prove is that any of it reached the disk: `reopenSession()`
 * opens a new session against the same living process, so the store plugin's in-memory
 * map is still the one answering. `15-preferences-on-disk` reads the file itself.
 */
describe('Preferences', () => {
  // Compact is measured on the rail too, and `05-spaces` leaves it put away.
  let railWasShowing = false;

  before(async () => {
    await canvas.open();
    railWasShowing = await rail.isShowing();
    await rail.show();
  });

  after(async () => {
    if (!railWasShowing) await rail.hide();
  });

  it('opens from the File menu', async () => {
    await fileMenu.openPreferences();
    expect(await settings.page('general').isExisting()).toBe(true);
  });

  it('applies the theme as it is chosen, before anything is written', async () => {
    await settings.setTheme('light');

    // A preview, not a write: nobody picks a theme without seeing it. Dark is the base
    // because the preference lives in a file nothing can read before Angular boots.
    expect(await browser.$('html').getAttribute('data-theme')).toBe('light');
  });

  it('previews the density the same way', async () => {
    await settings.setDensity('compact');
    expect(await browser.$('html').getAttribute('data-density')).toBe('compact');
  });

  /** Compact tightens the whole window, not the canvas alone: the topbar and the rail too. */
  it('tightens the topbar and the library rail, not only the cards', async () => {
    const paddings = () =>
      browser.execute(() => {
        const top = (selector: string) => {
          const element = document.querySelector(selector);
          return element ? getComputedStyle(element).paddingTop : null;
        };
        return { bar: top('.topbar'), row: top('.node-name') };
      });

    await settings.setDensity('comfortable');
    await eventually(
      paddings,
      ({ bar, row }) => bar === '12px' && row === '5px',
      'the comfortable topbar and rail',
    );
    await settings.setDensity('compact');
    await eventually(
      paddings,
      ({ bar, row }) => bar === '8px' && row === '4px',
      'the compact topbar and rail',
    );
  });

  /** A preview Annuler undoes is not the same thing as a write. */
  it('puts the previous appearance back on Annuler', async () => {
    // Applied first, so what Annuler falls back to is a theme this file chose rather than
    // whatever the machine resolves "system" to.
    await settings.setTheme('light');
    await settings.apply();

    await settings.setTheme('dark');
    expect(await browser.$('html').getAttribute('data-theme')).toBe('dark');

    await settings.cancel();

    expect(
      await eventually(
        () => browser.execute(() => document.documentElement.getAttribute('data-theme')),
        (theme) => theme === 'light',
        'Annuler to put the previous theme back',
      ),
    ).toBe('light');
  });

  /**
   * Everything but the appearance waits for the button. A half-captured global
   * shortcut live across the whole machine is the argument this panel was changed on,
   * and the language is in the same half.
   */
  it('holds the language until it is applied', async () => {
    await fileMenu.openPreferences();

    // Established, never assumed: the locale ships as "system", which resolves to the
    // machine's own, and the runner is not in French.
    await settings.setLocale('fr');
    await settings.apply();
    await eventually(
      () => titlebar.activeLocale(),
      (locale) => locale === 'fr',
      'the panel to settle on the locale this scenario starts from',
    );

    await settings.setLocale('en');
    expect(await titlebar.activeLocale()).toBe('fr');

    await settings.apply();

    expect(
      await eventually(
        () => titlebar.activeLocale(),
        (locale) => locale === 'en',
        'Appliquer to switch the interface language',
      ),
    ).toBe('en');
  });

  /**
   * Escape and the backdrop produce no click, so without this guard either one is a
   * silent Annuler — the one outcome nobody would have chosen on purpose.
   */
  describe('closing with work in hand', () => {
    beforeEach(async () => {
      await fileMenu.openPreferences();
      await settings.setDensity('comfortable');
    });

    it('says what is waiting instead of closing on Escape', async () => {
      await press('Escape');

      expect(await settings.unapplied().isExisting()).toBe(true);
      expect(await settings.page('general').isExisting()).toBe(true);
    });

    it('drops the draft when told to close without applying', async () => {
      await press('Escape');
      await settings.discard();

      expect(
        await eventually(
          () => browser.execute(() => document.documentElement.getAttribute('data-density')),
          (density) => density === 'compact',
          'the discarded draft to leave the stored density alone',
        ),
      ).toBe('compact');
    });
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

      // A global accelerator without a modifier would swallow that key in every
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
     * A canvas key takes a bare letter, where a global one may not: it answers only while the
     * canvas has the keyboard.
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

    /** The second action would be unreachable, and nothing would say which. */
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

  /**
   * The same signal the panel writes, so the two cannot disagree — and the panel keeps
   * its row: a setting that only exists in a corner of the titlebar is a setting nobody
   * finds twice. The theme was three gestures away where the language was one.
   */
  describe('the theme, from the titlebar', () => {
    /** "System" is not the titlebar's to set any more: back through the panel. */
    after(async () => {
      await fileMenu.openPreferences();
      await settings.setTheme('system');
      await settings.close();
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
      expect(await titlebar.shownTheme()).toBe('light');
    });

    it('is the same choice the panel shows', async () => {
      await fileMenu.openPreferences();

      expect(await settings.theme()).toBe('light');
      await settings.close();
    });

    /**
     * From "system", one press lands on the explicit opposite of what is on screen, never on
     * "system" again. What "system" resolves to is the runner's, so it is read, not assumed.
     */
    it('toggles between light and dark, never through system', async () => {
      await fileMenu.openPreferences();
      await settings.setTheme('system');
      await settings.close();
      const resolved = await titlebar.shownTheme();
      const opposite = resolved === 'dark' ? 'light' : 'dark';

      await $(testid('theme-toggle')).click();
      expect(
        await eventually(
          () => titlebar.shownTheme(),
          (shown) => shown === opposite,
          'the opposite theme',
        ),
      ).toBe(opposite);

      await $(testid('theme-toggle')).click();
      expect(
        await eventually(
          () => titlebar.shownTheme(),
          (shown) => shown === resolved,
          'back to where it was',
        ),
      ).toBe(resolved);

      await fileMenu.openPreferences();
      expect(await settings.theme()).toBe(resolved);
      await settings.close();
    });
  });

  /**
   * Ten chapters of prose in one scrolling panel was roughly 3 400 characters, all of it
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

    /** A walk you can only leave by finishing it is a wall with extra steps. */
    it('closes from any chapter', async () => {
      await aboutMenu.jumpTo('fields');
      await aboutMenu.close();

      expect(await aboutMenu.gettingStarted().isExisting()).toBe(false);
      await aboutMenu.openGettingStarted();
    });
  });
});
