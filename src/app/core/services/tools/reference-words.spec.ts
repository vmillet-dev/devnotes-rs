import { Injector, runInInjectionContext } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { referenceWords } from './reference-words';

describe('referenceWords', () => {
  const asked: string[] = [];
  const loaders = {
    fr: async () => (asked.push('fr'), { default: { hello: 'bonjour' } }),
    en: async () => (asked.push('en'), { default: { hello: 'hello' } }),
  };

  beforeEach(() => {
    asked.length = 0;
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting()] });
  });

  const words = () => runInInjectionContext(TestBed.inject(Injector), () => referenceWords(loaders));

  it('has nothing until the file of the language on screen lands', async () => {
    const said = words();

    expect(said()).toBeNull();
    await expect.poll(() => said()?.hello).toBe('bonjour');
    expect(asked).toEqual(['fr']);
  });

  it('imports the other file when the language changes', async () => {
    const said = words();
    await expect.poll(() => said()?.hello).toBe('bonjour');

    TestBed.inject(TranslocoService).setActiveLang('en');

    await expect.poll(() => said()?.hello).toBe('hello');
  });

  it('falls back on French for a language it has no file for', async () => {
    TestBed.inject(TranslocoService).setActiveLang('de');
    const said = words();

    await expect.poll(() => said()?.hello).toBe('bonjour');
  });
});
