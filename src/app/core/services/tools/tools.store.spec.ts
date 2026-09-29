import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { MAX_RECENT_TOOLS } from './tool.model';
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

  it('keeps no more than a handful', () => {
    for (let index = 0; index < MAX_RECENT_TOOLS + 3; index++) {
      store.open(`tool-${index}`);
    }

    expect(store.recents()).toHaveLength(MAX_RECENT_TOOLS);
    expect(store.recents()[0]!.id).toBe(`tool-${MAX_RECENT_TOOLS + 2}`);
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
