import { $, $$, browser } from '@wdio/globals';
import type { ChainablePromiseElement } from 'webdriverio';

import { readEach, setField, testid, toggleAndWait, waitForCanvas } from '../support/app.js';

/** Every selector the scenarios use lives here, so a renamed `data-testid` is one edit. */
export const canvas = {
  open: waitForCanvas,

  cards: () => $$(testid('note-card')),

  /**
   * Read in one call: a round trip per card leaves a window in which the canvas
   * re-renders, and the list that comes back mixes two states.
   */
  async titles(): Promise<string[]> {
    return browser.execute(
      (selector: string, titleSelector: string) =>
        [...document.querySelectorAll(selector)].map((card) =>
          (card.querySelector(titleSelector)?.textContent ?? '').trim(),
        ),
      testid('note-card'),
      testid('note-card-title'),
    );
  },

  /**
   * The note id behind a title, matched in one call for the same reason `titles()` is.
   * The card is then addressed by that id, so what comes back is resolved against the
   * canvas as it is now rather than against a position in a list that has moved.
   */
  async noteIdWithTitle(title: string): Promise<string> {
    await canvas.waitForCard(title);

    const id = await browser.execute(
      (cardSelector: string, titleSelector: string, wanted: string) =>
        [...document.querySelectorAll(cardSelector)]
          .find((card) => (card.querySelector(titleSelector)?.textContent ?? '').trim() === wanted)
          ?.getAttribute('data-note-id') ?? null,
      testid('note-card'),
      testid('note-card-title'),
      title,
    );

    if (id === null) {
      throw new Error(`no card titled "${title}" — found ${JSON.stringify(await canvas.titles())}`);
    }

    return id;
  },

  /** A selector and not a resolved element: it re-resolves on every command. */
  async cardWithTitle(title: string) {
    const id = await canvas.noteIdWithTitle(title);
    return $(`${testid('note-card')}[data-note-id="${id}"]`);
  },

  /**
   * What a card's head has to work with, measured in the page. The buttons are drawn
   * at `opacity: 0` until the pointer arrives, and opacity changes nothing about layout —
   * so their boxes are readable without hovering, which is the only way this is not flaky.
   */
  cardHeadLayout(title: string): Promise<{
    actionsBelowBottom: boolean;
    titleRowOffset: number;
    marksRightAligned: boolean;
    titleTop: number;
    snippetTop: number;
  } | null> {
    return browser.execute(
      (cardSelector: string, titleSelector: string, wanted: string) => {
        const shell = [...document.querySelectorAll(cardSelector)].find(
          (each) => (each.querySelector(titleSelector)?.textContent ?? '').trim() === wanted,
        );
        const card = shell?.querySelector('.card')?.getBoundingClientRect();
        const row = shell?.querySelector('.card-title-row')?.getBoundingClientRect();
        const titleBox = shell?.querySelector('.card-title')?.getBoundingClientRect();
        const marks = shell?.querySelector('.card-marks')?.getBoundingClientRect();
        const snippet = shell?.querySelector('.card-snippet')?.getBoundingClientRect();
        const actions = shell?.querySelector('.card-actions')?.getBoundingClientRect();
        if (!card || !row || !titleBox || !marks || !snippet || !actions) return null;

        return {
          // The pill hangs over the card's bottom edge instead of being laid out inside
          // it, which is what stops it costing the card a band of its own — and it is the
          // bottom, because the top right belongs to the marks now.
          actionsBelowBottom: actions.bottom > card.bottom && actions.top > titleBox.bottom,
          // The row starts at the card's own edge, exactly like the snippet under it.
          titleRowOffset: Math.round(row.left - snippet.left),
          // The marks end where the row ends, and the title is what gives way to them.
          marksRightAligned: Math.round(marks.right) <= Math.round(row.right) && marks.left >= titleBox.right,
          // And the body starts right after that one row: there is no band above it.
          titleTop: Math.round(titleBox.top - card.top),
          snippetTop: Math.round(snippet.top - row.bottom),
        };
      },
      testid('note-card'),
      testid('note-card-title'),
      title,
    );
  },

  /**
   * How visible the hover pill is on one card. Read from the computed style: the pill
   * hangs under the card's bottom edge, right where the arming band is asking its question,
   * and it stands down rather than crowding it.
   */
  actionsOpacity(title: string): Promise<string | null> {
    return browser.execute(
      (cardSelector: string, titleSelector: string, wanted: string) => {
        const shell = [...document.querySelectorAll(cardSelector)].find(
          (each) => (each.querySelector(titleSelector)?.textContent ?? '').trim() === wanted,
        );
        const actions = shell?.querySelector('.card-actions');
        return actions ? getComputedStyle(actions).opacity : null;
      },
      testid('note-card'),
      testid('note-card-title'),
      title,
    );
  },

  /**
   * The smallest box each control actually gets, measured in the page. Read from the
   * rendered boxes and not from the stylesheet: padding, line-height and the mixin compose,
   * and it is the composition that has to clear 24px.
   */
  async controlSizes(): Promise<Record<string, { width: number; height: number }[]>> {
    // The tag pill is one of the controls measured here, and the tag rail sits behind the
    // facets disclosure: closed, it is not in the DOM.
    await canvas.openFacets();

    return browser.execute(
      (itemSelector: string, pillSelector: string) => {
        const boxes = (selector: string) =>
          [...document.querySelectorAll(selector)].map((element) => {
            const box = element.getBoundingClientRect();
            return { width: Math.round(box.width), height: Math.round(box.height) };
          });

        return {
          item: boxes(itemSelector),
          action: boxes('.card-action'),
          menu: boxes('.card-menu-trigger'),
          copy: boxes('.copy-btn'),
          tag: boxes(pillSelector),
        };
      },
      testid('note-card-item'),
      testid('tag-pill'),
    );
  },

  /**
   * The card that really has the keyboard, read from `document.activeElement` rather than
   * from a class. The ring a card draws is `:focus-visible` on its own click surface, so
   * asserting on the store's idea of focus would not prove the keyboard went with it.
   */
  focusedCardTitle(): Promise<string | null> {
    return browser.execute(
      (cardSelector: string, titleSelector: string) => {
        const owner = document.activeElement?.closest(cardSelector);
        return owner ? ((owner.querySelector(titleSelector)?.textContent ?? '').trim() ?? null) : null;
      },
      testid('note-card'),
      testid('note-card-title'),
    );
  },

  /**
   * What the language badge is drawn outside its band, and how many boxes the selected chip
   * in the rail draws around it.
   *
   * Both come from the badge having a border of its own. The band is `overflow: hidden`,
   * so a badge taller than its line box is cut; and a chip that draws its own outline around
   * a badge that already has one reads as two rings rather than as a selection.
   */
  badgeBoxes(): Promise<{ cutByBand: number; outlines: number } | null> {
    return browser.execute(() => {
      const band = document.querySelector('.card-marks');
      const onCard = band?.querySelector('.lang-tag');
      const chip = document.querySelector('.language-chip.on');
      const inChip = chip?.querySelector('.lang-tag');
      if (!band || !onCard || !chip || !inChip) return null;

      // A border nobody can see is not an outline, whatever its width says. `transparent`
      // computes to `rgba(…, 0)`, so the alpha is what decides — read by splitting rather
      // than by matching, which is one escaping mistake fewer in a string sent to the page.
      const drawn = (element: Element) => {
        const style = getComputedStyle(element);
        const channels = style.borderTopColor
          .slice(style.borderTopColor.indexOf('(') + 1, style.borderTopColor.lastIndexOf(')'))
          .split(',');
        const alpha = channels.length > 3 ? Number(channels[3]) : 1;

        return alpha > 0 && Number.parseFloat(style.borderTopWidth) > 0 ? 1 : 0;
      };

      const bandBox = band.getBoundingClientRect();
      const badgeBox = onCard.getBoundingClientRect();

      return {
        cutByBand: Math.max(
          0,
          Math.round(bandBox.top - badgeBox.top),
          Math.round(badgeBox.bottom - bandBox.bottom),
        ),
        outlines: drawn(chip) + drawn(inChip),
      };
    });
  },

  /**
   * How far each todo row is drawn **outside** the box that holds it, in pixels. Counting
   * the rows in the DOM is not the same question: `.card-items` is `overflow: hidden` and
   * anchored to the bottom, so a row that no longer fits is still there and simply gets cut
   * off the top.
   */
  rowOverflow(title: string): Promise<number[] | null> {
    return browser.execute(
      (cardSelector: string, titleSelector: string, rowSelector: string, wanted: string) => {
        const shell = [...document.querySelectorAll(cardSelector)].find(
          (each) => (each.querySelector(titleSelector)?.textContent ?? '').trim() === wanted,
        );
        const list = shell?.querySelector('.card-items')?.getBoundingClientRect();
        if (!list) return null;

        return [...(shell?.querySelectorAll(rowSelector) ?? [])].map((row) => {
          const box = row.getBoundingClientRect();
          return Math.max(0, Math.round(list.top - box.top), Math.round(box.bottom - list.bottom));
        });
      },
      testid('note-card'),
      testid('note-card-title'),
      testid('note-card-item'),
      title,
    );
  },

  /**
   * The colour of the hairline around one format's badge in the rail. Read on the badge, not
   * the chip: the chip is the click surface, the badge carries the ring, and they are two
   * components.
   */
  badgeRing(language: string): Promise<string | null> {
    return browser.execute(
      (chipSelector: string, wanted: string) => {
        const chip = document.querySelector(`${chipSelector}[data-language="${wanted}"]`);
        const badge = chip?.querySelector('.lang-tag');
        return badge ? getComputedStyle(badge).borderTopColor : null;
      },
      testid('language-chip'),
      language,
    );
  },
  /** The colour the search box draws around itself, which says whether it has the caret. */
  searchBorderColour(): Promise<string> {
    return browser.execute(
      (sel: string) => getComputedStyle(document.querySelector(sel) as HTMLElement).borderTopColor,
      '.search-bar',
    );
  },

  async waitForCard(title: string): Promise<void> {
    await browser.waitUntil(async () => (await canvas.titles()).includes(title), {
      timeout: 15_000,
      timeoutMsg: `no card titled "${title}" appeared`,
    });
  },

  async waitForNoCard(title: string): Promise<void> {
    await browser.waitUntil(async () => !(await canvas.titles()).includes(title), {
      timeout: 15_000,
      timeoutMsg: `the card titled "${title}" is still there`,
    });
  },

  /**
   * Clicks the card's own click surface, which is a layer **under** what it shows: the
   * card's content is transparent to the pointer, so a click on the title reaches nothing
   * at all. It is also what keeps a checklist's tickable items out of the way.
   */
  async openNote(title: string): Promise<void> {
    const card = await canvas.cardWithTitle(title);
    await card.$(testid('note-card-open')).click();
    await $(testid('editor-title')).waitForExist({ timeout: 10_000 });
  },

  /** `ChainablePromiseElement` and not `Element`: a resolved element would go stale. */
  cardButton: (card: ChainablePromiseElement) => card.$(testid('note-card-open')),

  cardTags: (card: ChainablePromiseElement) => card.$(testid('note-card-tags')),

  async createSnippet(): Promise<void> {
    await $(testid('new-note')).click();
    await $(testid('editor-title')).waitForExist({ timeout: 10_000 });
  },

  /** Through the kind menu rather than the split button's default half. */
  async createSnippetFromMenu(): Promise<void> {
    await $(testid('new-note-kind')).click();
    await $(testid('new-note-snippet')).click();
    await $(testid('editor-title')).waitForExist({ timeout: 10_000 });
  },

  async createChecklist(): Promise<void> {
    await $(testid('new-note-kind')).click();
    await $(testid('new-note-checklist')).click();
    await $(testid('editor-title')).waitForExist({ timeout: 10_000 });
  },

  /** The empty card at the end of the `week` section, which is why it is always emitted. */
  async createFromGhost(): Promise<void> {
    await $(testid('create-ghost')).click();
    await $(testid('editor-title')).waitForExist({ timeout: 10_000 });
  },

  /**
   * The search crosses the bridge behind a 150 ms debounce, so a spec asserting
   * straight after typing reads the previous view.
   */
  async search(text: string): Promise<void> {
    await setField(testid('search-input'), text);
    // The condition rather than a guess at the debounce plus the round trip.
    await waitForCanvas();
  },

  searchQuery: (): Promise<string> => $(testid('search-input')).getValue(),

  async clearSearch(): Promise<void> {
    // `setValue('')` rather than select-all-then-Backspace: it goes through the element
    // endpoint, which the embedded driver implements, where key actions are dropped.
    await setField(testid('search-input'), '');
    await waitForCanvas();
  },

  /** One segmented control now, where four chips read as four combinable filters. */
  filter: (key: 'all' | 'pinned' | 'untriaged') =>
    $(`${testid('segmented-quick-filter')} [data-segment-id="${key}"]`),
  tagPill: (tag: string) => $(`${testid('tag-pill')}[data-tag="${tag}"]`),
  languageChip: (language: string) => $(`${testid('language-chip')}[data-language="${language}"]`),

  /** The three controls that re-run the query, each waited on rather than slept past. */
  async applyFilter(key: 'all' | 'pinned' | 'untriaged'): Promise<void> {
    await canvas.filter(key).click();
    await waitForCanvas();
  },

  /**
   * Opens the disclosure the two facet rails live behind. Idempotent: it refuses to fold while
   * a facet is selected.
   */
  async openFacets(): Promise<void> {
    if (await $(testid('facets-panel')).isExisting()) return;

    await $(testid('facets-toggle')).click();
    await $(testid('facets-panel')).waitForExist({ timeout: 5_000 });
  },

  async toggleTag(tag: string): Promise<void> {
    await canvas.openFacets();
    await canvas.tagPill(tag).click();
    await waitForCanvas();
  },

  async toggleLanguage(language: string): Promise<void> {
    await canvas.openFacets();
    await canvas.languageChip(language).click();
    await waitForCanvas();
  },
  sections: () => $$(testid('note-section')),

  sectionKeys: (): Promise<string[]> => readEach(testid('note-section'), '@data-section'),

  noResults: () => $(testid('canvas-no-results')),

  /** Where the canvas actually starts, under the header. */
  async firstCardTop(): Promise<number> {
    const top = await browser.execute(() => {
      const card = document.querySelector('[data-testid="note-card"]');
      return card ? Math.round(card.getBoundingClientRect().top) : null;
    });
    if (top === null) throw new Error('no card on the canvas to measure');
    return top;
  },

  async firstCardTitle(): Promise<string> {
    const [first] = await canvas.titles();
    if (first === undefined) throw new Error('no card on the canvas');
    return first;
  },

  /** What the search field says instead of its shortcut hint while filtering. */
  matchedCount: () => $(testid('search-matched')).getText(),

  /** "Gérer" sits at the end of the tag rail, which is behind the disclosure now. */
  async openTagManager(): Promise<void> {
    await canvas.openFacets();
    await $(testid('tag-manage')).click();
    await $(testid('tag-manager-close')).waitForExist({ timeout: 10_000 });
  },

  // Multiple selection
  async check(title: string): Promise<void> {
    const id = await canvas.noteIdWithTitle(title);
    await toggleAndWait(`${testid('note-card')}[data-note-id="${id}"] ${testid('note-card-check')}`);
  },

  async isChecked(title: string): Promise<boolean> {
    const card = await canvas.cardWithTitle(title);
    return (await card.$(testid('note-card-check')).getAttribute('aria-pressed')) === 'true';
  },

  /** Opening an already-open menu closes it, so this asks rather than toggles. */
  async openCardMenu(title: string) {
    const card = await canvas.cardWithTitle(title);
    if (!(await card.$(testid('note-card-menu-panel')).isExisting())) {
      await card.$(testid('note-card-menu')).click();
      await card.$(testid('note-card-menu-panel')).waitForExist({ timeout: 5_000 });
    }
    return card;
  },

  /** Both destructive menus confirm on a second click; one click alone deletes nothing. */
  async deleteNote(title: string): Promise<void> {
    const card = await canvas.openCardMenu(title);
    const remove = card.$(testid('note-card-delete'));
    await remove.click();
    await remove.click();
  },

  async moveNote(title: string, spaceId: string): Promise<void> {
    const card = await canvas.openCardMenu(title);
    await card.$(`${testid('note-card-move')}[data-space-id="${spaceId}"]`).click();
  },

  /** A batch of one, through the command the selection bar already takes. */
  async fileNote(title: string, folderId: string | null): Promise<void> {
    const card = await canvas.openCardMenu(title);
    const entry =
      folderId === null
        ? card.$(testid('note-card-unfile'))
        : card.$(`${testid('note-card-file')}[data-folder-id="${folderId}"]`);
    await entry.click();
  },

  async pinFromCardMenu(title: string): Promise<void> {
    const card = await canvas.openCardMenu(title);
    await card.$(testid('note-card-pin')).click();
  },

  /**
   * Which entries the menu offers, by `data-testid` rather than by label.
   *
   * Not the text: the suite switches the interface language partway through, so an
   * assertion on "Épingler" passes or fails depending on which spec file ran before this
   * one. Read in one call — a round trip per entry leaves a window in which the menu
   * can close.
   */
  async cardMenuEntries(title: string): Promise<string[]> {
    await canvas.openCardMenu(title);
    return browser.execute(
      (panelSelector: string) =>
        [...document.querySelectorAll(`${panelSelector} button`)].map(
          (item) => item.getAttribute('data-testid') ?? '',
        ),
      testid('note-card-menu-panel'),
    );
  },
};
