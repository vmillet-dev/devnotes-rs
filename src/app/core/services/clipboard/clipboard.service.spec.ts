import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { CLIPBOARD_ADAPTER, ClipboardAdapter, ClipboardService } from './clipboard.service';

function fakeAdapter(overrides: Partial<ClipboardAdapter> = {}): ClipboardAdapter {
  return {
    readText: vi.fn(async () => 'contenu'),
    writeText: vi.fn(async () => undefined),
    ...overrides,
  };
}

describe('ClipboardService', () => {
  let adapter: ClipboardAdapter;
  let service: ClipboardService;

  function setUp(double: ClipboardAdapter): void {
    adapter = double;
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [{ provide: CLIPBOARD_ADAPTER, useValue: adapter }],
    });
    service = TestBed.inject(ClipboardService);
  }

  beforeEach(() => setUp(fakeAdapter()));

  it('writes the value and reports success', async () => {
    await expect(service.copy('SELECT 1')).resolves.toBe(true);

    expect(adapter.writeText).toHaveBeenCalledWith('SELECT 1');
  });

  it('reads the value back', async () => {
    await expect(service.paste()).resolves.toBe('contenu');
  });

  it('reports failure rather than throwing when the plugin is unavailable', async () => {
    setUp(fakeAdapter({ writeText: vi.fn(async () => Promise.reject(new Error('no plugin'))) }));

    await expect(service.copy('x')).resolves.toBe(false);
  });

  it('reads an unavailable clipboard as empty', async () => {
    setUp(fakeAdapter({ readText: vi.fn(async () => Promise.reject(new Error('no plugin'))) }));

    await expect(service.paste()).resolves.toBe('');
  });

  it('acknowledges the copy, which is the only feedback four of its callers have', async () => {
    await service.copy('SELECT 1');

    expect(TestBed.inject(StatusNotifier).status()).toEqual({ key: 'settings.copied' });
  });

  it('stays quiet when the acknowledgement is turned off', async () => {
    TestBed.inject(SettingsStore).setCopyConfirmation(false);

    await service.copy('SELECT 1');

    expect(TestBed.inject(StatusNotifier).status()).toBeNull();
  });

  /** PRF-10: off, the card's « « zq-isolation » copiée dans le presse-papier. » still showed. */
  it('says what the caller copied, and that too only while the acknowledgement is on', async () => {
    const notifier = TestBed.inject(StatusNotifier);
    await service.copy('psql', { key: 'notes.copiedNote', params: { title: 'Connexion psql' } });
    expect(notifier.status()).toEqual({ key: 'notes.copiedNote', params: { title: 'Connexion psql' } });

    notifier.dismiss();
    await service.copy('psql', null);
    expect(notifier.status()).toBeNull();

    TestBed.inject(SettingsStore).setCopyConfirmation(false);
    await service.copy('psql', { key: 'notes.copiedNote', params: { title: 'Connexion psql' } });
    expect(notifier.status()).toBeNull();
    expect(service.acknowledges()).toBe(false);
  });

  it('says nothing about a copy that did not happen', async () => {
    setUp(fakeAdapter({ writeText: vi.fn(async () => Promise.reject(new Error('no plugin'))) }));

    await service.copy('x');

    expect(TestBed.inject(StatusNotifier).status()).toBeNull();
  });

  it('reads an empty clipboard as an empty string rather than null', async () => {
    // The plugin returns null on some platforms; callers test truthiness.
    setUp(fakeAdapter({ readText: vi.fn(async () => null) }));

    await expect(service.paste()).resolves.toBe('');
  });
});
