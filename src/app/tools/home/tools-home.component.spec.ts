import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ReferenceRow, ReferenceSource, referenceQueryKey } from '@core/services/tools/reference.model';
import { MAX_RECENT_TOOLS, TOOL_CATALOGUE, ToolDefinition } from '@core/services/tools/tool.model';
import { ToolSessions } from '@core/services/tools/tool-sessions';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FAKE_TOOLS, FakeToolComponent } from '@testing/tool-catalogue.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ToolsHomeComponent } from './tools-home.component';

describe('ToolsHomeComponent', () => {
  let fixture: ComponentFixture<ToolsHomeComponent>;
  let store: ToolsStore;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [ToolsHomeComponent],
      providers: [provideAppTesting(), { provide: TOOL_CATALOGUE, useValue: FAKE_TOOLS }],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsHomeComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const panels = () => all('[data-testid="tools-panel"]').map((panel) => panel.dataset['category']);
  const entries = () => all('[data-testid="tools-entry"]').map((entry) => entry.dataset['tool']);

  /** The subtitle says it: a band of its own above the panels only took their room. */
  it('counts the tools, and says in their title that none of them reaches the network', () => {
    const count: HTMLElement = fixture.nativeElement.querySelector('[data-testid="tools-count"]');

    expect(count.textContent!.trim()).toBe('3 utilitaires, tous hors ligne');
    expect(count.title).toContain('réseau');
    expect(fixture.nativeElement.querySelector('[role="note"]')).toBeNull();
  });

  it('lays the tools out in their category panels, in the categories order', () => {
    expect(panels()).toEqual(['text', 'crypto']);
    expect(entries()).toEqual(['case', 'line-breaks', 'hash']);
  });

  it('shows one category when the rail chose it', async () => {
    store.home('crypto');
    await fixture.whenStable();

    expect(panels()).toEqual(['crypto']);
  });

  /**
   * The rail is what says which category is showing. Hidden, the choice is kept — and said
   * in the top bar with the way back, or the catalogue would look as if it had lost tools.
   */
  it('says the category it is held to while the rail is hidden, with the way back', async () => {
    const settings = TestBed.inject(SettingsStore);
    const shown = () => fixture.nativeElement.querySelector('[data-testid="tools-category-shown"]');
    store.home('crypto');
    await fixture.whenStable();
    expect(shown()).toBeNull();

    (fixture.nativeElement.querySelector('[data-testid="tools-rail-toggle"]') as HTMLElement).click();
    await fixture.whenStable();
    expect(settings.showLibraryRail()).toBe(false);
    expect(panels()).toEqual(['crypto']);
    expect(shown().textContent).toContain('Crypto');
    expect(fixture.nativeElement.querySelector('[data-testid="tools-count"]')).toBeNull();

    // A search looks through every category: nothing is held back, so nothing is said.
    store.setSearch('case');
    await fixture.whenStable();
    expect(shown()).toBeNull();
    store.setSearch('');
    await fixture.whenStable();

    (fixture.nativeElement.querySelector('[data-testid="tools-category-clear"]') as HTMLElement).click();
    await fixture.whenStable();
    expect(panels()).toEqual(['text', 'crypto']);
    expect(shown()).toBeNull();
  });

  it('searches the keywords too, across every category', async () => {
    store.home('crypto');
    store.setSearch('CAMEL');
    await fixture.whenStable();

    expect(entries()).toEqual(['case']);
  });

  it('says so when nothing matches', async () => {
    store.setSearch('nothing like it');
    await fixture.whenStable();

    expect(entries()).toEqual([]);
    expect(fixture.nativeElement.querySelector('[data-testid="tools-no-match"]').textContent).toContain(
      'nothing like it',
    );
  });

  it('writes the search as it is typed', async () => {
    const field: HTMLInputElement = fixture.nativeElement.querySelector('[data-testid="tools-search"]');
    field.value = 'sha';
    field.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(store.search()).toBe('sha');
    expect(entries()).toEqual(['hash']);
  });

  it('offers the recent tools, and when they were opened', async () => {
    store.open('hash');
    store.home();
    await fixture.whenStable();

    const recent = fixture.nativeElement.querySelector('[data-testid="tools-recent"]') as HTMLButtonElement;
    expect(recent.dataset['tool']).toBe('hash');
    expect(recent.textContent).toContain('à l’instant');
  });

  it('opens a tool from its panel', () => {
    fixture.nativeElement.querySelector('[data-tool="line-breaks"]').click();

    expect(store.openId()).toBe('line-breaks');
  });

  it('takes the search field when Ctrl+Shift+T asks for it', async () => {
    store.requestSearch();
    await fixture.whenStable();

    expect(document.activeElement?.getAttribute('data-testid')).toBe('tools-search');
    expect(store.searchWanted()).toBe(false);
  });
});

/**
 * « generateur » found nothing, though three tools sit under « Générateurs »; « teapot » and
 * « 418 » found nothing, though the status reference holds them; and with results on screen
 * the keyboard had no way to them but Tab.
 */
describe('ToolsHomeComponent, searching', () => {
  let fixture: ComponentFixture<ToolsHomeComponent>;
  let store: ToolsStore;
  let loads: number;

  const load = async () => FakeToolComponent;

  /** Seven rows that all answer « teapot »: more than the home lists of one reference. */
  const MANY: ReferenceSource<unknown, ReferenceRow> = {
    words: { fr: async () => ({ default: null }), en: async () => ({ default: null }) },
    rows: () =>
      Array.from({ length: 7 }, (_, index) => ({
        key: String(8000 + index),
        group: 'all',
        searchable: [`teapot ${index}`],
      })),
    title: (row) => row.key,
    line: () => 'a port',
  };

  const CATALOGUE: readonly ToolDefinition[] = [
    ...FAKE_TOOLS,
    { id: 'lorem', category: 'generate', keywords: [], load },
    { id: 'password', category: 'generate', keywords: [], load },
    {
      id: 'http-status',
      category: 'reference',
      keywords: [],
      load,
      reference: () => {
        loads += 1;
        return import('@tools/catalogue/http-status/http-status.reference').then(
          (m) => m.HTTP_STATUS_REFERENCE,
        );
      },
    },
    { id: 'ports', category: 'reference', keywords: [], load, reference: async () => MANY },
  ];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    localStorage.clear();
    loads = 0;
    TestBed.configureTestingModule({
      imports: [ToolsHomeComponent],
      providers: [provideAppTesting(), { provide: TOOL_CATALOGUE, useValue: CATALOGUE }],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsHomeComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const one = (testid: string): HTMLElement | null =>
    fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  const entries = () => all('[data-testid="tools-entry"]').map((entry) => entry.dataset['tool']);
  const hits = () => all('[data-testid="tools-reference-hit"]').map((hit) => hit.dataset['key']);
  const field = () => one('tools-search') as HTMLInputElement;
  const search = async (typed: string) => {
    store.setSearch(typed);
    await fixture.whenStable();
  };
  const press = async (key: string, on: Element | null = document.activeElement) => {
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
    on!.dispatchEvent(event);
    await fixture.whenStable();
    return event;
  };

  it('finds the tools of a category by its name, accents and case aside, in the language on screen', async () => {
    await search('GENERATEUR');
    expect(entries()).toEqual(['lorem', 'password']);

    await search('générateurs');
    expect(entries()).toEqual(['lorem', 'password']);

    TestBed.inject(TranslocoService).setActiveLang('en');
    await search('generators');
    expect(entries()).toEqual(['lorem', 'password']);
    await search('generateur');
    expect(entries()).toEqual([]);
  });

  it('loads no reference by opening, then finds an entry by its code or by a word of it', async () => {
    expect(loads).toBe(0);

    await search('418');
    expect(entries()).toEqual([]);
    expect(hits()).toEqual(['418']);
    const hit = one('tools-reference-hit')!;
    expect(hit.dataset['tool']).toBe('http-status');
    expect(hit.textContent).toContain("418 I'm a teapot");
    expect(hit.textContent).toContain('Codes de statut HTTP');
    expect(one('tools-no-match')).toBeNull();

    await search('teapot');
    expect(hits().slice(0, 1)).toEqual(['418']);
    expect(loads).toBe(1);
  });

  it('lists a few rows of a reference and counts the others, which open it searched for the same', async () => {
    await search('teapot');

    expect(hits()).toEqual(['418', '8000', '8001', '8002', '8003']);
    const more = one('tools-reference-more')!;
    expect(more.dataset['tool']).toBe('ports');
    expect(more.textContent).toContain('3 autres dans');

    more.click();
    expect(store.openId()).toBe('ports');
    expect(store.entry()).toBeNull();
    expect(TestBed.inject(ToolSessions).slot(referenceQueryKey('ports'), '')()).toBe('teapot');
  });

  it('opens a reference on the row that was found, its own search emptied', async () => {
    const own = TestBed.inject(ToolSessions).slot(referenceQueryKey('http-status'), '');
    own.set('left from before');
    await search('teapot');

    one('tools-reference-hit')!.click();

    expect(store.openId()).toBe('http-status');
    expect(store.entry()).toBe('418');
    expect(own()).toBe('');
  });

  it('marks the first result as the one Enter opens, a tool before a reference', async () => {
    expect(one('tools-first-result')).toBeNull();

    await search('lorem');
    expect(all('[data-first]').map((first) => first.dataset['tool'])).toEqual(['lorem']);
    expect(one('tools-first-result')!.textContent!.trim()).toBe('↵');
    expect((await press('Enter', field())).defaultPrevented).toBe(true);
    expect(store.openId()).toBe('lorem');

    store.home();
    await search('418');
    expect(all('[data-first]').map((first) => first.dataset['key'])).toEqual(['418']);
    await press('Enter', field());
    expect(store.openId()).toBe('http-status');
    expect(store.entry()).toBe('418');
  });

  it('does nothing on Enter while nothing is searched, or nothing is found', async () => {
    expect((await press('Enter', field())).defaultPrevented).toBe(false);
    expect(store.openId()).toBeNull();

    await search('nothing like it');
    await press('Enter', field());
    expect(store.openId()).toBeNull();
    expect(one('tools-no-match')!.textContent).toContain('nothing like it');
  });

  it('goes down from the field into the results and through them, and back up to the field', async () => {
    await search('teapot');
    field().focus();
    const focused = () =>
      (document.activeElement as HTMLElement).dataset['key'] ??
      (document.activeElement as HTMLElement).dataset['testid'];

    expect((await press('ArrowDown')).defaultPrevented).toBe(true);
    expect(focused()).toBe('418');
    await press('ArrowDown');
    expect(focused()).toBe('8000');
    for (let step = 0; step < 6; step++) await press('ArrowDown');
    // The last one: « n autres ». Another press stays on it.
    expect(focused()).toBe('tools-reference-more');

    for (let step = 0; step < 5; step++) await press('ArrowUp');
    expect(focused()).toBe('418');
    await press('ArrowUp');
    expect(focused()).toBe('tools-search');
    // Other keys are the browser's.
    expect((await press('a')).defaultPrevented).toBe(false);
  });

  it('walks the tools then the references, in the order they are read', async () => {
    await search('http');
    field().focus();
    await press('ArrowDown');

    expect((document.activeElement as HTMLElement).dataset['testid']).toBe('tools-entry');
    expect(all('[data-result]').map((result) => result.dataset['testid'])[0]).toBe('tools-entry');
    expect(all('[data-result]').at(-1)!.dataset['testid']).toMatch(/^tools-reference-/);
  });

  it('empties the search on Escape, and leaves the field on a second one', async () => {
    await search('teapot');
    field().focus();

    expect((await press('Escape')).defaultPrevented).toBe(true);
    expect(store.search()).toBe('');
    expect(document.activeElement).toBe(field());

    await press('Escape');
    expect(document.activeElement).not.toBe(field());
  });

  it('empties the search from a result too, back in the field', async () => {
    await search('teapot');
    field().focus();
    await press('ArrowDown');

    await press('Escape');

    expect(store.search()).toBe('');
    expect(document.activeElement).toBe(field());
    // With nothing searched, Escape on a tool is nobody's.
    const tool = one('tools-entry')!;
    tool.focus();
    expect((await press('Escape')).defaultPrevented).toBe(false);
    // And a key pressed on what is no result is left alone.
    expect((await press('ArrowDown', one('tools-count'))).defaultPrevented).toBe(false);
  });

  it('shows the five tools opened last, though more are kept for the rail', async () => {
    for (const id of ['case', 'line-breaks', 'hash', 'lorem', 'password', 'http-status', 'ports']) {
      store.open(id);
    }
    store.home();
    await fixture.whenStable();

    expect(store.recents()).toHaveLength(7);
    expect(all('[data-testid="tools-recent"]').map((recent) => recent.dataset['tool'])).toEqual([
      'ports',
      'http-status',
      'password',
      'lorem',
      'hash',
    ]);
    expect(all('[data-testid="tools-recent"]')).toHaveLength(MAX_RECENT_TOOLS);
  });
});

/**
 * Thirty-two tools, and the five used every day were found again each time through a category
 * or the search: « Récents » helps, but moves with every tool opened.
 */
describe('ToolsHomeComponent, the favourites', () => {
  let fixture: ComponentFixture<ToolsHomeComponent>;
  let store: ToolsStore;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [ToolsHomeComponent],
      providers: [provideAppTesting(), { provide: TOOL_CATALOGUE, useValue: FAKE_TOOLS }],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsHomeComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const tools = (testid: string) =>
    all(`[data-testid="${testid}"]`).map((element) => element.dataset['tool']);
  const star = (tool: string): HTMLButtonElement =>
    fixture.nativeElement.querySelector(`[data-testid="tools-entry-star"][data-tool="${tool}"]`);

  it('stars a tool from its row without opening it, the star beside the row and not inside it', async () => {
    const row = star('hash').closest('.tool-row')!;
    expect(star('hash').getAttribute('aria-pressed')).toBe('false');
    expect(star('hash').getAttribute('aria-label')).toBe('Favori : Hash / HMAC');
    // Two layers: the row opens the tool, the star is its sibling — and a button, so Tab reaches it.
    expect(star('hash').localName).toBe('button');
    expect(star('hash').tabIndex).toBe(0);
    expect(row.querySelector('[data-testid="tools-entry"]')!.contains(star('hash'))).toBe(false);
    expect(star('hash').hasAttribute('data-result')).toBe(false);

    star('hash').click();
    await fixture.whenStable();

    expect(store.favourites()).toEqual(['hash']);
    expect(store.openId()).toBeNull();
    expect(star('hash').getAttribute('aria-pressed')).toBe('true');
    expect(row.classList.contains('starred')).toBe(true);
    expect(star('case').getAttribute('aria-pressed')).toBe('false');
  });

  it('shows the favourites above the recents once there is one, in the order they were starred', async () => {
    expect(tools('tools-favourite')).toEqual([]);
    expect(fixture.nativeElement.querySelector('#tools-favourites-title')).toBeNull();

    star('hash').click();
    star('case').click();
    store.open('line-breaks');
    store.home();
    await fixture.whenStable();

    expect(tools('tools-favourite')).toEqual(['hash', 'case']);
    const titles = all('.section-title').map((title) => title.textContent!.trim());
    expect(titles).toEqual(['Favoris', 'Récents']);

    all('[data-testid="tools-favourite"]')[1]!.click();
    expect(store.openId()).toBe('case');

    // Gone with the last one.
    store.home();
    star('hash').click();
    star('case').click();
    await fixture.whenStable();
    expect(tools('tools-favourite')).toEqual([]);
    expect(all('.section-title').map((title) => title.textContent!.trim())).toEqual(['Récents']);
  });

  it('does not repeat a favourite among the recents', async () => {
    store.open('hash');
    store.open('case');
    store.home();
    await fixture.whenStable();
    expect(tools('tools-recent')).toEqual(['case', 'hash']);

    star('case').click();
    await fixture.whenStable();

    expect(tools('tools-favourite')).toEqual(['case']);
    expect(tools('tools-recent')).toEqual(['hash']);
  });

  it('drops a starred tool the catalogue no longer has', async () => {
    store.toggleFavourite('gone');
    store.toggleFavourite('hash');
    await fixture.whenStable();

    expect(tools('tools-favourite')).toEqual(['hash']);
  });

  it('narrows the catalogue to the favourites, as a category does, and says so with the rail hidden', async () => {
    star('hash').click();
    star('line-breaks').click();
    store.home('favourites');
    await fixture.whenStable();

    expect(tools('tools-entry')).toEqual(['line-breaks', 'hash']);
    expect(all('[data-testid="tools-panel"]').map((panel) => panel.dataset['category'])).toEqual([
      'text',
      'crypto',
    ]);

    TestBed.inject(SettingsStore).toggleLibraryRail();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[data-testid="tools-category-shown"]').textContent).toContain(
      'Favoris',
    );

    // A search still answers over the whole catalogue.
    store.setSearch('camel');
    await fixture.whenStable();
    expect(tools('tools-entry')).toEqual(['case']);
    store.setSearch('');

    // Narrowed to favourites the file alone still names, the whole catalogue shows.
    store.toggleFavourite('hash');
    store.toggleFavourite('line-breaks');
    store.toggleFavourite('gone');
    store.home('favourites');
    await fixture.whenStable();
    expect(tools('tools-entry')).toEqual(['case', 'line-breaks', 'hash']);
  });

  it('leaves the search as it was: a favourite does not outrank a better match', async () => {
    star('hash').click();
    store.setSearch('a');
    await fixture.whenStable();

    // The catalogue's order, and no section of chips over the results.
    expect(tools('tools-entry')).toEqual(['case', 'line-breaks', 'hash']);
    expect(tools('tools-favourite')).toEqual([]);
  });
});
