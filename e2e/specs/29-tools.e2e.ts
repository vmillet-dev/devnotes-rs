import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { activeTestId, eventually, press, setField, testid } from '../support/app.js';

describe('The tools', () => {
  before(async () => {
    await canvas.open();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('tools-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  describe('their home', () => {
    it('opens on its search from the notes, on Ctrl+Shift+T', async () => {
      await press('T', ['Control', 'Shift']);

      await $(testid('tools-page')).waitForDisplayed({ timeout: 10_000 });
      await eventually(activeTestId, (id) => id === 'tools-search', 'the search field to take the focus');
    });

    it('says what it holds reaches no network', async () => {
      expect(await $(testid('tools-promise')).isDisplayed()).toBe(true);
    });

    it('says so when no tool answers a search', async () => {
      await setField(testid('tools-search'), 'zzz-no-such-tool');

      await $(testid('tools-no-match')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('tools-search'), '');
    });
  });
});
