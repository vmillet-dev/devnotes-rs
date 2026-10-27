import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { selectionBar } from '../pageobjects/header.page.js';
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
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The header's controls as drawn: each one's box, which row it sits on, and whether any runs
 * past the header. Rows are told apart by their middles, as the controls differ in height.
 */
async function headerLayout() {
  return browser.execute(() => {
    const topbar = document.querySelector('.topbar') as HTMLElement | null;
    if (!topbar) return null;
    const controls = [
      ...topbar.querySelectorAll<HTMLElement>(':scope > [data-group] > *, :scope > app-new-note-button'),
    ];
    const bar = topbar.getBoundingClientRect();
    const boxes = controls.map((control) => {
      const box = control.getBoundingClientRect();
      return {
        tag: control.tagName.toLowerCase(),
        middle: Math.round(box.top + box.height / 2),
        height: Math.round(box.height),
        inside: box.left >= bar.left - 1 && box.right <= bar.right + 1,
      };
    });
    const middles = boxes.map((box) => box.middle).sort((a, b) => a - b);
    const starts = middles.filter((middle, at) => at === 0 || middle - middles[at - 1]! > 12);
    const rowOf = (tag: string) =>
      starts.filter((start) => start <= boxes.find((box) => box.tag === tag)!.middle + 12).length;
    return {
      railShowing: document.querySelector('app-library-tree') !== null,
      rows: starts.length,
      tallest: Math.max(...boxes.map((box) => box.height)),
      overflowing:
        Math.round(topbar.scrollWidth) > Math.round(topbar.clientWidth) || boxes.some((box) => !box.inside),
      newNoteRow: rowOf('app-new-note-button'),
      railToggleRow: rowOf('app-rail-toggle'),
      searchRow: rowOf('app-search-box'),
    };
  });
}

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
    await bridge.createNote(
      draft({ spaceId, title: 'Compte rendu de migration', content: '## Décisions', kind: 'note' }),
    );
    await bridge.createNote(draft({ spaceId, title: 'Rotation des secrets', priority: 'urgent' }));
    await reloadCanvas();
  });

  // `unstyled-control` is `all: unset` and `cursor` is inherited, so a field takes its
  // parent's arrow. The `<label>` is the visible box here.
  it('says it can be typed into, on the whole box', async () => {
    expect(await cursorOf(testid('search-input'))).toBe('text');
    expect(await cursorOf('.search-bar')).toBe('text');
  });

  /**
   * The field is borderless inside the <label> that draws the box, so the box is the
   * only thing on screen that can say the caret is here. It said nothing at all.
   */
  it('says it has the caret, on the box the field sits in', async () => {
    const resting = await canvas.searchBorderColour();

    await canvas.search('Docker');

    expect(await canvas.searchBorderColour()).not.toBe(resting);
    await canvas.clearSearch();
  });

  /** At the floor, which is the width a wrapping toolbar leaves it most of the time. */
  it('says what it searches without being cut off', async () => {
    expect(await placeholderFitsAtItsFloor(testid('search-input'), 'app-search-box')).toBe(true);
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
   * Opening a note sets the canvas cursor, which the cards follow with the real focus. The first
   * character typed rebuilds every card as one flat section: none of them may take the keyboard
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

  /** A Note is prose: the line that found it is quoted in the card's text, not as code. */
  it('quotes a Note found by its text as prose', async () => {
    const { id } = await bridge.createNote(
      draft({
        spaceId: await homeSpaceId(),
        title: 'Rollout notes',
        kind: 'note',
        content: '## Before\n\nAsk the on-call before the zebra migration.',
      }),
    );
    await reloadCanvas();
    await canvas.search('zebra');

    const card = await canvas.cardWithTitle('Rollout notes');
    expect(await card.$(testid('note-card-hit')).isExisting()).toBe(true);
    expect(await card.$('.card-snippet .line-content').isExisting()).toBe(false);

    await canvas.clearSearch();
    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  /** COD-21, REC-03: a context was searched nowhere, and shown on a pinned card alone. */
  it('finds a snippet by its context, quotes it, and shows it on its card', async () => {
    const title = 'Connexion psql e2e';
    const { id } = await bridge.createNote(
      draft({
        spaceId: await homeSpaceId(),
        title,
        content: 'psql -h db.internal',
        language: 'sh',
        // A context of its own: the samples' « psql connection » carries PostgreSQL too.
        source: 'Zqbase / production',
      }),
    );
    await reloadCanvas();
    const card = await canvas.cardWithTitle(title);
    expect((await card.$(testid('note-card-when')).getText()).trim()).toBe('Zqbase');

    await canvas.search('zqbase');
    expect(await canvas.titles()).toEqual([title]);
    const found = await canvas.cardWithTitle(title);
    expect((await found.$(testid('note-card-hit')).getText()).trim()).toBe('Zqbase / production');

    await canvas.clearSearch();
    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  it('says so when nothing matches', async () => {
    await canvas.search('nothing matches this');
    expect(await canvas.noResults().isExisting()).toBe(true);
  });

  /**
   * An empty state that only reports is a dead end: the one thing to do from here is the
   * thing that emptied it, and `clearFilters()` was already sitting there unoffered.
   */
  it('offers the way out of the state that emptied it', async () => {
    await canvas.search('nothing matches this');

    // « Effacer la recherche »: no filter is set, only the search.
    expect(await $(testid('canvas-clear-filters')).getAttribute('data-clears')).toBe('search');
    await $(testid('canvas-clear-filters')).click();
    await waitForCanvas();

    expect(await canvas.searchQuery()).toBe('');
    expect(await canvas.noResults().isExisting()).toBe(false);
  });

  /** How big is this result, and why is that card in it. */
  describe('what a search says about itself', () => {
    /**
     * Mocha runs a suite's own tests before its nested suites, so this block is the
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

      // Not on the words. The zero case is written out — French calls zero `one`, so
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
     * On a card, an inline badge's border hangs outside its line box and an `overflow: hidden`
     * band cuts it; in the rail, a second outline at another radius reads as two rings.
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
     * The rail cannot reach the badge: `.lang-tag` belongs to `app-language-badge`, so a rule in
     * the rail's stylesheet carries the rail's `_ngcontent` and matches nothing.
     */
    it('marks the selected format on the badge that carries the ring', async () => {
      const resting = await canvas.badgeRing('sh');
      await canvas.toggleLanguage('sh');
      const selected = await canvas.badgeRing('sh');
      await canvas.toggleLanguage('sh');

      expect(resting).not.toBeNull();
      expect(selected).not.toBe(resting);
    });
    /** Its own button in the row, shown whenever any of the three is on. */
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

    /** The cross in the field empties the field, and nothing else. */
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
      // Focus has to leave the field: the canvas keyboard ignores a keystroke aimed at
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

  /** Counted over the space, so the count does not move with the selection. */
  it('filters on a kind from the rail, and gives every kind back from "All"', async () => {
    await canvas.openFacets();
    const counted = Number(await canvas.kindChip('note').$('.kind-count').getText());
    expect(counted).toBeGreaterThan(0);

    await canvas.toggleKind('note');
    const titles = await canvas.titles();
    expect(titles).toContain('Compte rendu de migration');
    expect(titles).not.toContain('Docker compose');
    expect(await canvas.sectionKeys()).toEqual(['results']);
    expect(await canvas.kindChip('all').getAttribute('aria-pressed')).toBe('false');
    expect(Number(await canvas.kindChip('note').$('.kind-count').getText())).toBe(counted);

    await canvas.toggleKind('all');
    expect(await canvas.titles()).toContain('Docker compose');
    expect(await canvas.kindChip('note').getAttribute('aria-pressed')).toBe('false');
  });

  /**
   * REC-05: Type and Priority counted their notes, Format and Tags did not, and the results
   * read « Résultats · 45 · à trier bientôt ».
   */
  it('counts the notes of a format and a tag as the filter finds them, and heads the results with the count alone', async () => {
    const { id } = await bridge.createNote(
      draft({
        spaceId: await homeSpaceId(),
        title: 'Renouveler le certificat',
        content: 'certbot renew',
        language: 'sh',
        tags: ['ops'],
        lifecycle: { kind: 'expires', at: new Date(Date.now() + 86_400_000).toISOString() },
      }),
    );
    await reloadCanvas();
    const resultsCount = () =>
      $(`${testid('note-section')}[data-section="results"] ${testid('note-section-count')}`).getText();

    await canvas.openFacets();
    const tagged = Number(await canvas.tagPill('ops').$(testid('tag-count')).getText());
    expect(tagged).toBeGreaterThan(1);
    await canvas.toggleTag('ops');
    expect((await resultsCount()).trim()).toBe(`· ${tagged}`);
    await canvas.toggleTag('ops');

    const shells = Number(await canvas.languageChip('sh').$(testid('language-count')).getText());
    expect(shells).toBeGreaterThan(0);
    await canvas.toggleLanguage('sh');
    expect((await resultsCount()).trim()).toBe(`· ${shells}`);
    await canvas.toggleLanguage('sh');

    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  /** Ordered in Rust: titles folded, ties on the last edit; the card names the date sorted by. */
  it('orders the notes as asked, and dates the cards by creation when sorted so', async () => {
    const spaceId = await homeSpaceId();
    const byTitle = await bridge.queryNotes(
      query({ spaceId, search: 'migration', order: { key: 'title', direction: 'ascending' } }),
    );
    const byCreation = await bridge.queryNotes(
      query({ spaceId, search: 'migration', order: { key: 'created', direction: 'descending' } }),
    );

    expect(byTitle.sections[0]?.notes.map((note) => note.title)).toEqual([
      'Compte rendu de migration',
      'Étape de migration',
    ]);
    expect(byCreation.sections[0]?.notes.every((note) => note.footer.kind === 'created')).toBe(true);
  });

  /**
   * The arrangement is the library's, and every later spec file reads the same preferences:
   * the scenario puts the default back before it ends.
   */
  it('sorts by title in one section from the topbar menu', async () => {
    const fold = (title: string): string => title.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

    await canvas.arrange('sort-grouping', 'none');
    await canvas.arrange('sort-key', 'title');
    await canvas.setPinnedFirst(false);

    const titles = (await canvas.titles()).map(fold);
    const keys = await canvas.sectionKeys();

    await canvas.setPinnedFirst(true);
    await canvas.arrange('sort-key', 'modified');
    await canvas.arrange('sort-grouping', 'date');
    await press('Escape');

    expect(keys).toEqual(['all']);
    expect(titles.every((title, at) => at === 0 || titles[at - 1]! <= title)).toBe(true);
    expect(await canvas.sectionKeys()).toContain('week');
  });

  /** From the most pressing level down, "none" always there to create in. */
  it('gathers the notes by priority when asked', async () => {
    const spaceId = await homeSpaceId();
    const view = await bridge.queryNotes(query({ spaceId, grouping: 'priority' }));

    const ids = view.sections.map((section) => section.id);
    expect(ids[0]).toBe('priority-urgent');
    expect(ids.at(-1)).toBe('priority-none');
    expect(view.sections[0]?.notes.map((note) => note.title)).toContain('Rotation des secrets');
  });

  /** Several levels at once, like tags; "All" gives every note back. */
  it('filters on a priority from the rail', async () => {
    await canvas.togglePriority('urgent');
    const titles = await canvas.titles();
    expect(titles).toContain('Rotation des secrets');
    expect(titles).not.toContain('Docker compose');
    expect(await canvas.sectionKeys()).toEqual(['results']);

    await canvas.togglePriority('all');
    expect(await canvas.titles()).toContain('Docker compose');
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

  /**
   * The header at the size the application ships with: one row, the primary action on it, and
   * the search no narrower than its placeholder.
   */
  describe('the header, at the window the application ships with', () => {
    let restore = { width: 1100, height: 720 };

    before(async () => {
      restore = await browser.getWindowSize();
      await browser.setWindowSize(1100, 720);
      await reloadCanvas();
      await waitForCanvas();
    });

    after(async () => {
      await browser.setWindowSize(restore.width, restore.height);
      await reloadCanvas();
      await waitForCanvas();
    });

    it('fits on one row, with the library rail open', async () => {
      const header = await headerLayout();

      expect(header?.railShowing).toBe(true);
      expect(header?.rows).toBe(1);
      expect(header?.overflowing).toBe(false);
    });

    it('puts the first card in the top half of the window', async () => {
      const top = await browser.execute(() => {
        const card = document.querySelector('[data-testid="note-card"]');
        return card ? Math.round(card.getBoundingClientRect().top) : null;
      });

      expect(top).not.toBeNull();
      expect(top!).toBeLessThan(360);
    });

    /** The bar floats over the canvas: inserted, it pushed everything down 85px. */
    it('moves nothing when a card is ticked', async () => {
      const before = await canvas.firstCardTop();

      await canvas.check(await canvas.firstCardTitle());

      const after = await canvas.firstCardTop();
      expect(Math.abs(after - before)).toBeLessThanOrEqual(2);
      await selectionBar.clear();
    });
  });

  /**
   * The rail hidden hands the header the space and folder switchers. Squeezed, they broke
   * their labels and the quick filter stood as a column of three: now the search gives way,
   * then the switchers' names, then the header takes two rows on purpose.
   */
  describe('the header, the library rail hidden', () => {
    let restore = { width: 1100, height: 720 };

    const at = async (width: number) => {
      await browser.setWindowSize(width, 720);
      await reloadCanvas();
      await waitForCanvas();
      if (await $('app-library-tree').isExisting()) await $(testid('library-rail-toggle')).click();
      await $('app-library-tree').waitForExist({ reverse: true, timeout: 10_000 });
      await $(testid('space-switcher')).waitForDisplayed({ timeout: 10_000 });
      return headerLayout();
    };

    before(async () => {
      restore = await browser.getWindowSize();
    });

    after(async () => {
      await browser.setWindowSize(restore.width, restore.height);
      await reloadCanvas();
      await waitForCanvas();
      if (!(await $('app-library-tree').isExisting())) await $(testid('library-rail-toggle')).click();
      await $('app-library-tree').waitForExist({ timeout: 10_000 });
    });

    it('holds one row in a wide window, no control broken onto two lines', async () => {
      const header = await at(1330);

      expect(header?.rows).toBe(1);
      expect(header?.tallest).toBeLessThan(40);
      expect(header?.overflowing).toBe(false);
    });

    it('overflows nothing at the window the application ships with', async () => {
      const header = await at(1100);

      expect(header?.tallest).toBeLessThan(40);
      expect(header?.overflowing).toBe(false);
    });

    it('takes two rows at the narrowest window, « Nouvelle note » ending the first', async () => {
      const header = await at(640);

      expect(header?.rows).toBe(2);
      expect(header?.newNoteRow).toBe(header?.railToggleRow);
      expect(header?.searchRow).toBe(2);
      expect(header?.tallest).toBeLessThan(40);
      expect(header?.overflowing).toBe(false);
      // Cut, the names are whole in their titles.
      expect(await $(testid('space-switcher')).getAttribute('title')).not.toBe('');
    });

    it('overflows nothing at the narrowest window with the rail open either', async () => {
      await at(640);
      await $(testid('library-rail-toggle')).click();
      await $('app-library-tree').waitForExist({ timeout: 10_000 });
      const header = await headerLayout();

      expect(header?.rows).toBe(2);
      expect(header?.tallest).toBeLessThan(40);
      expect(header?.overflowing).toBe(false);
      expect(await placeholderFitsAtItsFloor(testid('search-input'), 'app-search-box')).toBe(true);
    });
  });
});
