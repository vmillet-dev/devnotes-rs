import { $, $$, browser } from '@wdio/globals';

import {
  blur,
  checkedSegment,
  choiceLabel,
  clickToAddRow,
  eventually,
  pickChoice,
  pickSegment,
  readEach,
  testid,
} from '../support/app.js';
import { bridge } from '../support/bridge.js';

/**
 * Not `<select>`s any more: the language is a menu and the other two are segmented
 * controls, so there is no value to assign. Addressed by `data-testid` and never by
 * their translated label — the suite switches the interface language partway through.
 */
const SEGMENTED = { theme: 'setting-theme', density: 'setting-density' } as const;

export const titlebar = {
  title: () => $(testid('titlebar-title')).getText(),

  /** The theme on screen, which on "system" is whatever the machine resolved it to. */
  shownTheme: () => $(testid('theme-toggle')).getAttribute('data-theme-shown'),

  /**
   * Light or dark only: the button toggles between the two, and "system" is the
   * preferences panel's to choose. One press at most.
   */
  async setTheme(theme: 'light' | 'dark'): Promise<void> {
    if ((await titlebar.shownTheme()) === theme) return;

    await $(testid('theme-toggle')).click();
    await eventually(
      () => titlebar.shownTheme(),
      (shown) => shown === theme,
      `the ${theme} theme on screen`,
    );
  },

  /** Read off `<html lang>`, which `LocaleService` writes. */
  activeLocale: (): Promise<string> => browser.execute(() => document.documentElement.lang),
};

export const fileMenu = {
  open: () => $(testid('file-menu')).click(),
  entry: (id: string) => $(`${testid('file-option')}[data-entry="${id}"]`),

  /** Asserted on, never clicked: it would take the application down mid-run. */
  quit: () => $(testid('file-quit')),

  async isDisabled(id: string): Promise<boolean> {
    return (await fileMenu.entry(id).getAttribute('aria-disabled')) === 'true';
  },

  async openPreferences(): Promise<void> {
    await $(testid('file-menu')).click();
    await $(testid('file-preferences')).click();
    await $(testid('settings-page')).waitForExist({ timeout: 10_000 });
  },
};

export const settings = {
  page: (id: string) => $(`${testid('settings-page')}[data-page="${id}"]`),
  open: (id: string) => settings.page(id).click(),

  /** The button marked OK: it writes the draft **and** closes. */
  close: () => $(testid('settings-close')).click(),
  apply: () => $(testid('settings-apply')).click(),
  cancel: () => $(testid('settings-cancel')).click(),
  /** The strip that replaces the buttons once a close was attempted with work in hand. */
  unapplied: () => $(testid('settings-unapplied')),
  discard: () => $(testid('settings-discard')).click(),

  /** Addressed by its action, never by position: the page holds eleven of them. */
  shortcut: (action: string) => $(`${testid('shortcut-field')}[data-action="${action}"]`),
  resetShortcut: (action: string) => $(`${testid('shortcut-reset')}[data-action="${action}"]`).click(),
  /** An id holding a dot, so an attribute selector rather than `#`. */
  shortcutError: (action: string) => $(`[id="shortcut-error-${action}"]`),

  /** What the language menu names, which is the only one of the three that has a trigger. */
  locale: () => choiceLabel('setting-locale'),

  async setLocale(locale: string): Promise<void> {
    await pickChoice('setting-locale', locale);
    await browser.pause(200);
  },

  /** The label addresses the group: a segmented control has no `id` to assign to. */
  async setTheme(theme: string): Promise<void> {
    await pickSegment(SEGMENTED.theme, theme);
    await browser.pause(200);
  },

  async setDensity(density: string): Promise<void> {
    await pickSegment(SEGMENTED.density, density);
    await browser.pause(200);
  },

  theme: () => checkedSegment(SEGMENTED.theme),
  density: () => checkedSegment(SEGMENTED.density),
};

export const variables = {
  async open(): Promise<void> {
    await fileMenu.openPreferences();
    await settings.page('notes.variables').click();
  },

  rows: () => $$(testid('variable-row')),

  names: (): Promise<string[]> => readEach(testid('variable-row'), 'value', testid('variable-name')),

  /**
   * Waits for the value to reach the back end, not for a plausible number of
   * milliseconds: the panel commits on Appliquer and the write crosses the bridge.
   */
  async add(name: string, value: string): Promise<void> {
    const last = await clickToAddRow(testid('variable-add'), testid('variable-row'));
    await last.$(testid('variable-name')).setValue(name);
    await last.$(testid('variable-value')).setValue(value);
    await blur();
    await settings.apply();

    await browser.waitUntil(async () => (await bridge.listGlobalPlaceholders())[name] === value, {
      timeout: 10_000,
      timeoutMsg: `the variable "${name}" never reached the back end as ${JSON.stringify(value)}`,
    });
  },

  async remove(name: string): Promise<void> {
    // The row has to be there before it can be found: reading once and giving up reports
    // a row missing that the error message then prints.
    await browser.waitUntil(async () => (await variables.names()).includes(name), {
      timeout: 10_000,
      timeoutMsg: `no variable row named "${name}" ever appeared`,
    });

    const index = (await variables.names()).indexOf(name);
    if (index >= 0) {
      const rows = await $$(testid('variable-row')).getElements();
      const row = rows[index];
      if (row) {
        await row.$(testid('variable-remove')).click();
        await blur();
        await settings.apply();
        await browser.waitUntil(async () => !(await variables.names()).includes(name), {
          timeout: 10_000,
          timeoutMsg: `the variable "${name}" is still listed`,
        });
        // Gone from the panel is not gone from the database — same bridge, same wait.
        await browser.waitUntil(async () => !(name in (await bridge.listGlobalPlaceholders())), {
          timeout: 10_000,
          timeoutMsg: `the variable "${name}" is still stored`,
        });
        return;
      }
    }

    throw new Error(`no variable row named "${name}" — found ${JSON.stringify(await variables.names())}`);
  },
};

/** The "À propos" menu in the titlebar, and the help panel it opens. */
export const aboutMenu = {
  async openGettingStarted(): Promise<void> {
    await $(testid('about-open')).click();
    await $(testid('about-getting-started')).click();
    await $(testid('getting-started')).waitForExist({ timeout: 10_000 });
  },

  gettingStarted: () => $(testid('getting-started')),

  /** The guide shows one chapter at a time: this is the one on screen. */
  chapter: () => $(testid('guide-chapter')).getAttribute('data-chapter'),

  next: () => $(testid('guide-next')).click(),

  jumpTo: (chapter: string) => $(`${testid('guide-dot')}[data-chapter="${chapter}"]`).click(),

  close: () => $(testid('guide-close')).click(),
};

export const banners = {
  status: () => $(testid('status-toast')),
  error: () => $(testid('error-banner')),
};
