import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { TRANSLOCO_LOADER, TranslocoService } from '@jsverse/transloco';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { LocaleService } from './locale.service';
import { AppTranslocoLoader } from './transloco-loader';

describe('LocaleService', () => {
  /** Mirrors the app initializer: the settings first, the language it holds second. */
  function createService(): LocaleService {
    TestBed.inject(SettingsStore).restore();
    const service = TestBed.inject(LocaleService);
    void service.restore();
    return service;
  }

  /** Asserted through `PreferencesService`, not through whatever it happens to sit on. */
  function preferences(): PreferencesService {
    return TestBed.inject(PreferencesService);
  }

  /** jsdom reports `en-US`, which would make every assertion depend on the runner. */
  function stubSystemLanguages(...tags: string[]): void {
    Object.defineProperty(navigator, 'languages', { value: tags, configurable: true });
    Object.defineProperty(navigator, 'language', { value: tags[0] ?? '', configurable: true });
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    document.documentElement.lang = '';
    stubSystemLanguages('de-DE');
    TestBed.configureTestingModule({ providers: [provideTranslocoTesting()] });
  });

  afterEach(() => {
    Reflect.deleteProperty(navigator, 'languages');
    Reflect.deleteProperty(navigator, 'language');
  });

  it('follows the system language when nothing has been chosen', () => {
    stubSystemLanguages('fr-FR');

    expect(createService().activeLocale()).toBe('fr');
  });

  it('falls back to English when the system speaks neither language', () => {
    expect(createService().activeLocale()).toBe('en');
  });

  it('takes the first supported entry of the system list', () => {
    stubSystemLanguages('de-DE', 'en-US', 'fr-FR');

    expect(createService().activeLocale()).toBe('en');
  });

  it('starts on "system" and writes nothing until a language is picked', () => {
    const service = createService();

    expect(service.preference()).toBe('system');
    expect(preferences().read('devnotes.locale')).toBeNull();
  });

  it('restores a previously persisted locale', () => {
    preferences().write('devnotes.locale', 'fr');

    expect(createService().activeLocale()).toBe('fr');
  });

  it('prefers a stored choice over the system language', () => {
    stubSystemLanguages('en-US');
    preferences().write('devnotes.locale', 'fr');

    expect(createService().activeLocale()).toBe('fr');
  });

  it('ignores an invalid persisted value and goes back to the system language', () => {
    stubSystemLanguages('fr-FR');
    preferences().write('devnotes.locale', 'de');

    expect(createService().activeLocale()).toBe('fr');
  });

  it('sets the active locale and persists the choice', () => {
    const service = createService();

    service.setLocale('en');
    TestBed.tick();

    expect(service.activeLocale()).toBe('en');
    expect(preferences().read('devnotes.locale')).toBe('en');
  });

  it('hands the decision back to the system when set to "system"', () => {
    stubSystemLanguages('fr-FR');
    preferences().write('devnotes.locale', 'en');
    const service = createService();

    service.setLocale('system');
    TestBed.tick();

    expect(service.activeLocale()).toBe('fr');
    expect(preferences().read('devnotes.locale')).toBe('system');
  });

  it('keeps the document language in sync, which drives screen-reader pronunciation', () => {
    stubSystemLanguages('fr-FR');
    const service = createService();
    TestBed.tick();
    expect(document.documentElement.lang).toBe('fr');

    service.setLocale('en');
    TestBed.tick();

    expect(document.documentElement.lang).toBe('en');
  });

  /** With the application's loader, which imports a chunk: the startup awaits this. */
  it('resolves once the active language is loaded, so the first render has its strings', async () => {
    TestBed.overrideProvider(TRANSLOCO_LOADER, { useValue: new AppTranslocoLoader() });
    stubSystemLanguages('en-US');
    TestBed.inject(SettingsStore).restore();

    await TestBed.inject(LocaleService).restore();

    expect(TestBed.inject(TranslocoService).translate('notes.untitled')).not.toBe('notes.untitled');
  });
});
