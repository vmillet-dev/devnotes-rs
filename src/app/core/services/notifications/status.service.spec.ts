import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { STATUS_TTL_MS, StatusNotifier } from './status.service';

describe('StatusNotifier', () => {
  let notifier: StatusNotifier;

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    notifier = TestBed.inject(StatusNotifier);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('says after the page reloads what was said as it reloaded, and only once', () => {
    notifier.notifyAfterReload({ key: 'backups.restored', params: { path: 'replaced/x' } });
    expect(notifier.status()).toBeNull();

    notifier.resumeAfterReload();
    expect(notifier.status()).toEqual({ key: 'backups.restored', params: { path: 'replaced/x' } });

    notifier.dismiss();
    notifier.resumeAfterReload();
    expect(notifier.status()).toBeNull();
  });

  it('says it at once when there is nothing to keep it in', () => {
    vi.stubGlobal('sessionStorage', {
      setItem: () => {
        throw new Error('blocked');
      },
    });

    notifier.notifyAfterReload({ key: 'backups.restored' });

    expect(notifier.status()).toEqual({ key: 'backups.restored' });
    vi.unstubAllGlobals();
  });

  it('starts with nothing to say', () => {
    expect(notifier.status()).toBeNull();
  });

  it('holds the reference it was handed, untranslated', () => {
    notifier.notify({ key: 'file.exported', params: { notes: '3' } });

    expect(notifier.status()).toEqual({ key: 'file.exported', params: { notes: '3' } });
  });

  it('clears itself after its time is up', async () => {
    notifier.notify({ key: 'file.exported' });

    await vi.advanceTimersByTimeAsync(STATUS_TTL_MS);

    expect(notifier.status()).toBeNull();
  });

  it('restarts the countdown on a second message', async () => {
    notifier.notify({ key: 'file.exported' });
    await vi.advanceTimersByTimeAsync(STATUS_TTL_MS - 1);

    notifier.notify({ key: 'file.copied' });
    await vi.advanceTimersByTimeAsync(STATUS_TTL_MS - 1);

    expect(notifier.status()?.key).toBe('file.copied');
  });

  it('can be dismissed before its time', () => {
    notifier.notify({ key: 'file.exported' });

    notifier.dismiss();

    expect(notifier.status()).toBeNull();
  });
});
