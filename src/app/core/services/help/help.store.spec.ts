import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { HelpStore } from './help.store';

describe('HelpStore', () => {
  let store: HelpStore;

  beforeEach(() => {
    store = TestBed.inject(HelpStore);
  });

  it('shows one panel at a time, the guide included', () => {
    store.show('about');
    store.open('folders');

    expect(store.panel()).toBe('gettingStarted');
    expect(store.chapter()).toBe('folders');

    store.show('shortcuts');

    expect(store.panel()).toBe('shortcuts');
    expect(store.chapter()).toBeNull();
  });

  it('puts every panel away at once', () => {
    store.open();
    store.close();

    expect(store.panel()).toBeNull();
    expect(store.chapter()).toBeNull();
  });
});
