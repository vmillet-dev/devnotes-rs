import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SETTINGS_KEYS } from '@core/services/settings/app-settings.model';
import { provideAppTesting } from '@testing/testing.providers';
import { KEPT_RECENT_TOOLS, MAX_RECENT_TOOLS } from './tool.model';
import { ToolSessions } from './tool-sessions';
import { ToolsStore } from './tools.store';

describe('ToolsStore', () => {
  let store: ToolsStore;

  beforeEach(() => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({ providers: [provideAppTesting()] });
    store = TestBed.inject(ToolsStore);
  });

  it('opens a tool and remembers it first, once', () => {
    store.open('hash');
    store.open('slug');
    store.open('hash');

    expect(store.openId()).toBe('hash');
    expect(store.recents().map((recent) => recent.id)).toEqual(['hash', 'slug']);
    expect(Number.isNaN(Date.parse(store.recents()[0]!.at))).toBe(false);
  });

  /** More than are shown: the rail leaves out the ones its section lists, and still shows five. */
  it('keeps more recents than it shows, and no more than that', () => {
    for (let index = 0; index < KEPT_RECENT_TOOLS + 3; index++) {
      store.open(`tool-${index}`);
    }

    expect(KEPT_RECENT_TOOLS).toBeGreaterThan(MAX_RECENT_TOOLS);
    expect(store.recents()).toHaveLength(KEPT_RECENT_TOOLS);
    expect(store.recents()[0]!.id).toBe(`tool-${KEPT_RECENT_TOOLS + 2}`);
  });

  it('opens a reference on a row, until it is opened plainly, searched by hand or left', () => {
    store.open('http-status', '418');
    expect(store.entry()).toBe('418');

    store.forgetEntry();
    expect(store.entry()).toBeNull();

    store.open('http-status', '404');
    store.open('http-status');
    expect(store.entry()).toBeNull();

    store.open('http-status', '404');
    store.home();
    expect(store.entry()).toBeNull();
  });

  /** « Récents » moves with every tool opened: the five used every day are starred instead. */
  it('stars a tool and unstars it, in the order they were starred, kept with the application', () => {
    store.toggleFavourite('hash');
    store.toggleFavourite('case');
    store.toggleFavourite('lorem');
    expect(store.favourites()).toEqual(['hash', 'case', 'lorem']);

    store.toggleFavourite('case');
    expect(store.favourites()).toEqual(['hash', 'lorem']);
    // Starred again, it goes last: the order is the order of starring.
    store.toggleFavourite('case');
    expect(store.favourites()).toEqual(['hash', 'lorem', 'case']);

    // Ids, and nothing a tool holds.
    expect(TestBed.inject(PreferencesService).read(SETTINGS_KEYS.favouriteTools)).toBe(
      JSON.stringify(['hash', 'lorem', 'case']),
    );
  });

  it('widens a home narrowed to the favourites once the last one is unstarred', () => {
    store.toggleFavourite('hash');
    store.toggleFavourite('case');
    store.home('favourites');

    store.toggleFavourite('hash');
    expect(store.category()).toBe('favourites');
    store.toggleFavourite('case');
    expect(store.category()).toBeNull();

    // A category chosen is left alone.
    store.toggleFavourite('hash');
    store.home('crypto');
    store.toggleFavourite('hash');
    expect(store.category()).toBe('crypto');
  });

  it('goes home, to the whole catalogue or to one category of it', () => {
    store.open('hash');
    store.home('crypto');

    expect(store.openId()).toBeNull();
    expect(store.category()).toBe('crypto');
  });

  it('asks the home for its search field until the home has taken it', () => {
    store.open('hash');
    store.requestSearch();

    expect(store.openId()).toBeNull();
    expect(store.searchWanted()).toBe(true);

    store.searchTaken();
    expect(store.searchWanted()).toBe(false);
  });
});

describe('ToolSessions', () => {
  it('hands back the same slot to a tool found again', () => {
    TestBed.resetTestingModule();
    const sessions = TestBed.inject(ToolSessions);

    sessions.slot('slug.text', '').set('Été 2026');

    expect(sessions.slot('slug.text', '')()).toBe('Été 2026');
    expect(sessions.slot('case.text', '')()).toBe('');
  });
});
