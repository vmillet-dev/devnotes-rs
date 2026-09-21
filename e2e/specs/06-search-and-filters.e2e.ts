import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import {
  activeTestId,
  blur,
  cursorOf,
  placeholderFitsAtItsFloor,
  press,
  reloadCanvas,
  testid,
  waitForCanvas,
} from '../support/app.js';
import { bridge, draft, homeSpaceId } from '../support/bridge.js';

/**
 * Filtering, grouping and facet aggregation all run in Rust. What crosses the bridge here
 * is the query itself — the debounce, the local-day offset, and an exhaustive section set.
 */
describe('Search, filters and facets', () => {
  before(async () => {
    await canvas.open();
    const spaceId = await homeSpaceId();
    await bridge.createNote(
      draft({
        spaceId,
        title: 'Étape de migration',
        content: 'ALTER TABLE notes',
        language: 'sql',
        tags: ['db'],
      }),
    );
    await bridge.createNote(
      draft({
        spaceId,
        title: 'Docker compose',
        content: 'docker compose up -d',
        language: 'sh',
        tags: ['ops'],
      }),
    );
    await bridge.createNote(draft({ spaceId, title: 'Pinned reference', pinned: true, tags: ['ops'] }));
    await reloadCanvas();
  });

  // ⚠️ `unstyled-control` is `all: unset` and `cursor` is inherited, so a field takes its
  // parent's arrow. The `<label>` is the visible box here.
  it('says it can be typed into, on the whole box', async () => {
    expect(await cursorOf(testid('search-input'))).toBe('text');
    expect(await cursorOf('.search-bar')).toBe('text');
  });

  /**
   * ⚠️ The field is borderless inside the <label> that draws the box, so the box is the
   * only thing on screen that can say the caret is here. It said nothing at all.
   */
  it('says it has the caret, on the box the field sits in', async () => {
    const resting = await canvas.searchBorderColour();

    await canvas.search('Docker');

    expect(await canvas.searchBorderColour()).not.toBe(resting);
    await canvas.clearSearch();
  });

  /** ⚠️ At the floor, which is the width a wrapping toolbar leaves it most of the time. */
  it('says what it searches without being cut off', async () => {
    expect(await placeholderFitsAtItsFloor(testid('search-input'), '.search-bar')).toBe(true);
  });

  it('matches on the title, past the debounce', async () => {
    await canvas.search('Docker');
    expect(await canvas.titles()).toEqual(['Docker compose']);
  });

  it('matches on the body too', async () => {
    await canvas.search('ALTER TABLE');
    expect(await canvas.titles()).toEqual(['Étape de migration']);
  });

  it('folds case the way Rust does, not the way SQLite would', async () => {
    // `LOWER()` without ICU only folds ASCII, which is why matching runs on the fetched
    // rows and not in the WHERE clause.
    await canvas.search('étape');
    expect(await canvas.titles()).toEqual(['Étape de migration']);
  });

  it('folds the accents too, on both sides of the comparison', async () => {
    // Nobody reaches for the accent key to search, and the corpus is written with them.
    await canvas.search('etape');
    expect(await canvas.titles()).toEqual(['Étape de migration']);

    // Symmetric, because the needle goes through the same fold as the haystack.
    await canvas.search('Dôcker');
    expect(await canvas.titles()).toEqual(['Docker compose']);
  });

  /**
   * ⚠️ Opening a note is what arms this: it sets the canvas cursor, and the cards follow
   * that cursor with the real focus. The first character typed switches the canvas to one
   * flat section, every card is rebuilt, and the rebuilt one used to grab the keyboard
   * back out of the field mid-word.
   */
  it('keeps the keyboard in the field once a note has been opened', async () => {
    await canvas.openNote('Docker compose');
    await editor.close();

    await canvas.search('Docker');

    expect(await activeTestId()).toBe('search-input');
    await canvas.clearSearch();
  });

  it('collapses to a single flat results section while searching', async () => {
    // Searched here rather than inherited: an `it` that depends on the previous one
    // cannot be run or reordered alone.
    await canvas.search('étape');
    expect(await canvas.sectionKeys()).toEqual(['results']);
  });

  it('says so when nothing matches', async () => {
    await canvas.search('nothing matches this');
    expect(await canvas.noResults().isExisting()).toBe(true);
  });

  /**
   * ⚠️ An empty state that only reports is a dead end: the one thing to do from here is the
   * thing that emptied it, and `clearFilters()` was already sitting there unoffered.
   */
  it('offers the way out of the state that emptied it', async () => {
    await canvas.search('nothing matches this');

    await $(testid('canvas-clear-filters')).click();
    await waitForCanvas();

    expect(await canvas.searchQuery()).toBe('');
    expect(await canvas.noResults().isExisting()).toBe(false);
  });

  /** How big is this result, and why is that card in it. */
  describe('what a search says about itself', () => {
    /**
     * ⚠️ Mocha runs a suite's own tests before its nested suites, so this block is the
     * last thing in the file whatever its position — and a search left in the field is a
     * search the next spec file inherits.
     */
    after(async () => {
      await canvas.clearSearch();
    });

    it('counts the results, and says zero rather than going quiet', async () => {
      await canvas.search('Docker');
      // Against what is on screen: the corpus is shared with every file that ran before.
      expect(await canvas.matchedCount()).toContain(String((await canvas.titles()).length));

      // ⚠️ Not on the words. The zero case is written out — French calls zero `one`, so
      // "0 résultat" would read as a singular — but **the suite runs in whatever language
      // the machine is set to**: French here, English on CI. Asserting "aucun" passed
      // locally and failed on both runners. What the scenario is about is that the badge
      // still says something, so that is what it asks.
      await canvas.search('nothing matches this');
      const atZero = (await canvas.matchedCount()).replace('✕', '').trim();
      expect(atZero.length).toBeGreaterThan(0);
      expect(await canvas.noResults().isExisting()).toBe(true);
    });

    it('hides the count again once nothing is being filtered', async () => {
      await canvas.clearSearch();
      expect(await $(testid('search-matched')).isExisting()).toBe(false);
    });

    /**
     * ⚠️ Both halves of #201, and both came from the hairline #196 gave the badge. On a card
     * the band is `overflow: hidden` and a `<span>` is inline, so the badge's border hung
     * outside the line box its host was sized to and was cut. In the rail the chip drew a
     * second outline two pixels further out, at a different radius — two rings around one
     * badge rather than a selection.
     */
    it('draws the badge whole, with one outline around the selected one', async () => {
      await canvas.toggleLanguage('sh');

      const boxes = await canvas.badgeBoxes();

      expect(boxes).not.toBeNull();
      expect(boxes?.cutByBand).toBe(0);
      expect(boxes?.outlines).toBe(1);

      await canvas.toggleLanguage('sh');
    });

    /**
     * ⚠️ #207: the rail cannot reach the badge. `.lang-tag` belongs to `app-language-badge`,
     * so a rule written in the rail's own stylesheet is rewritten with the rail's
     * `_ngcontent` attribute and never matches — the selection had no mark at all while
     * `outlines` above still counted one. The badge's own hairline was that one.
     */
    it('marks the selected format on the badge that carries the ring', async () => {
      const resting = await canvas.badgeRing('sh');
      await canvas.toggleLanguage('sh');
      const selected = await canvas.badgeRing('sh');
      await canvas.toggleLanguage('sh');

      expect(resting).not.toBeNull();
      expect(selected).not.toBe(resting);
    });
    /**
     * ⚠️ Its own button in the row, shown whenever any of the three is on. The way out
     * used to be the count inside the field, which looks like a count and reads like a
     * cross, and did neither.
     */
    it('drops the search, the tag and the language in one click', async () => {
      await canvas.search('Docker');
      await canvas.toggleTag('ops');
      await canvas.toggleLanguage('sh');

      await $(testid('clear-filters')).click();
      await canvas.open();

      expect(await $(testid('search-input')).getValue()).toBe('');
      expect(await $(testid('clear-filters')).isExisting()).toBe(false);
      expect(await canvas.tagPill('ops').getAttribute('aria-pressed')).toBe('false');
      expect(await canvas.languageChip('sh').getAttribute('aria-pressed')).toBe('false');
    });

    /**
     * ⚠️ The cross in the field empties the field. It used to drop the tags and the
     * languages with it, which is not what a cross in a search box means anywhere.
     */
    it('empties the field alone from the cross inside it', async () => {
      await canvas.search('Docker');
      await canvas.toggleTag('ops');

      await $(testid('search-clear')).click();
      await waitForCanvas();

      expect(await $(testid('search-input')).getValue()).toBe('');
      expect(await canvas.tagPill('ops').getAttribute('aria-pressed')).toBe('true');

      await canvas.toggleTag('ops');
      await waitForCanvas();
    });

    it('does the same on Escape, once there is no selection to clear', async () => {
      await canvas.search('Docker');
      // ⚠️ Focus has to leave the field: the canvas keyboard ignores a keystroke aimed at
      // an input, which is what leaves Ctrl+K and typing alone.
      await blur();
      await press('Escape');
      await canvas.open();

      expect(await $(testid('search-matched')).isExisting()).toBe(false);
    });

    it('quotes the line that matched rather than the head of the body', async () => {
      const spaceId = await homeSpaceId();
      await bridge.createNote(
        draft({
          spaceId,
          title: 'Deployment runbook',
          // The needle is on the last line: a preview of the first three explains nothing.
          content: ['# preamble', 'nothing to see', 'still nothing', '  helm upgrade gateway'].join(
            String.fromCharCode(10),
          ),
          language: 'sh',
        }),
      );
      await reloadCanvas();

      await canvas.search('helm');
      const card = await canvas.cardWithTitle('Deployment runbook');
      // Trimmed of its indentation: a card shows one line and it starts with the code.
      expect(await card.$('.card-snippet').getText()).toBe('helm upgrade gateway');
    });

    it('quotes the tag when that is what matched, since no preview ever showed it', async () => {
      await canvas.search('db');
      const card = await canvas.cardWithTitle('Étape de migration');
      expect(await card.$(testid('note-card-hit')).getText()).toContain('db');
    });

    it('quotes nothing when the title is what matched, the card showing it already', async () => {
      await canvas.search('Docker');
      const card = await canvas.cardWithTitle('Docker compose');
      expect(await card.$(testid('note-card-hit')).isExisting()).toBe(false);
      // Back to the head of the body, which is what the card shows outside a search.
      expect(await card.$('.card-snippet').getText()).toContain('docker compose up -d');
    });
  });

  it('goes back to the chronological sections when the search is cleared', async () => {
    await canvas.clearSearch();
    const keys = await canvas.sectionKeys();
    expect(keys).not.toContain('results');
    // `week` is always emitted: it hosts the create-ghost card.
    expect(keys).toContain('week');
  });

  it('shows a note its tags, on the card', async () => {
    await canvas.search('Docker');
    const card = await canvas.cardWithTitle('Docker compose');
    expect(await canvas.cardTags(card).getText()).toContain('ops');
    await canvas.clearSearch();
  });

  it('filters on a tag from the rail', async () => {
    await canvas.toggleTag('ops');
    expect((await canvas.titles()).sort()).toEqual(['Docker compose', 'Pinned reference']);
    await canvas.toggleTag('ops');
  });

  it('filters on a language from the rail', async () => {
    await canvas.toggleLanguage('sql');
    expect(await canvas.titles()).toEqual(['Étape de migration']);
    await canvas.toggleLanguage('sql');
  });

  it('keeps the quick filters chronological, unlike a facet', async () => {
    await canvas.applyFilter('pinned');

    const titles = await canvas.titles();
    expect(titles).toContain('Pinned reference');
    expect(titles).not.toContain('Docker compose');
    // A quick filter keeps the chronological shape; only a search or a facet flattens it.
    expect(await canvas.sectionKeys()).not.toContain('results');
    await canvas.applyFilter('all');
  });
});
