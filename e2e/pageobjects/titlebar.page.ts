import { $, $$, browser } from '@wdio/globals';

import { blur, clickToAddRow, readEach, setNativeValue, testid } from '../support/app.js';
import { bridge } from '../support/bridge.js';

/** The controls' `id`s, in one place: `select` needs the selector, the getters the element. */
const CONTROL = {
  theme: '#setting-theme',
  density: '#setting-density',
  locale: '#setting-locale',
} as const;

export const titlebar = {
  title: () => $(testid('titlebar-title')).getText(),

  /** One of the few untranslated labels, so a scenario can pin the language it asserts in. */
  setLocale: (locale: 'fr' | 'en') => $(`${testid('locale-option')}[data-locale="${locale}"]`).click(),

  /** One call: two reads leave a window in which the pressed option can change. */
  activeLocale: async (): Promise<string> =>
    (
      await browser.execute(
        (selector: string) =>
          [...document.querySelectorAll(selector)]
            .filter((option) => option.getAttribute('aria-pressed') === 'true')
            .map((option) => option.getAttribute('data-locale') ?? ''),
        testid('locale-option'),
      )
    )[0] ?? '',
};

export const fileMenu = {
  open: () => $(testid('file-menu')).click(),
  entry: (id: string) => $(`${testid('file-option')}[data-entry="${id}"]`),

  /** ⚠️ Asserted on, never clicked: it would take the application down mid-run. */
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
  close: () => $(testid('settings-close')).click(),

  /**
   * Addressed by their `id`, which is the `for` target of their own `<label>` and cannot
   * be renamed without breaking the association.
   */
  control: CONTROL,

  locale: () => $(CONTROL.locale),
  shortcut: () => $('#setting-shortcut'),
  resetShortcut: () => $('.setting-shortcut-reset').click(),

  /** Takes the selector and not the element: `setNativeValue` assigns and dispatches. */
  async select(selector: string, value: string): Promise<void> {
    await setNativeValue(selector, value);
    await browser.pause(200);
  },
};

export const variables = {
  async open(): Promise<void> {
    await fileMenu.openPreferences();
    await settings.page('notes.variables').click();
  },

  rows: () => $$(testid('variable-row')),

  names: (): Promise<string[]> => readEach(testid('variable-row'), 'value', testid('variable-name')),

  /**
   * ⚠️ Waits for the value to reach the back end, not for a plausible number of
   * milliseconds: the panel commits on blur and the write crosses the bridge.
   */
  async add(name: string, value: string): Promise<void> {
    const last = await clickToAddRow(testid('variable-add'), testid('variable-row'));
    await last.$(testid('variable-name')).setValue(name);
    await last.$(testid('variable-value')).setValue(value);
    await blur();

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
};

export const banners = {
  status: () => $(testid('status-toast')),
  error: () => $(testid('error-banner')),
};
