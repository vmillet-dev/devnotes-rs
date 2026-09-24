import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeUpdater } from '@testing/fake-updater';
import { UpdateStore } from './update.store';
import { UpdaterService } from './updater.service';

describe('UpdateStore', () => {
  let updater: FakeUpdater;
  let store: UpdateStore;
  let notifier: ErrorNotifier;
  let settings: SettingsStore;

  beforeEach(() => {
    TestBed.resetTestingModule();
    updater = new FakeUpdater();
    TestBed.configureTestingModule({
      providers: [{ provide: UpdaterService, useValue: updater }],
    });
    store = TestBed.inject(UpdateStore);
    notifier = TestBed.inject(ErrorNotifier);
    settings = TestBed.inject(SettingsStore);
  });

  it('stays idle when the app is already up to date', async () => {
    await store.check();

    expect(store.status()).toBe('idle');
    expect(store.update()).toBeNull();
  });

  it('proposes the update without installing anything', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0', notes: 'Corrections' };

    await store.check();

    expect(store.status()).toBe('available');
    expect(store.update()?.version).toBe('0.2.0');
    expect(updater.installCalls).toBe(0);
  });

  it('reports a failed check to the console but not to the user', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    updater.checkError = new Error('network unreachable');

    await store.check();

    expect(store.status()).toBe('idle');
    expect(notifier.notice()).toBeNull();
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it('installs and relaunches once the user accepts', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    await store.check();

    await store.accept();

    expect(updater.installCalls).toBe(1);
    expect(updater.relaunched).toBe(true);
    expect(store.status()).toBe('installed');
  });

  it('exposes download progress as a whole percentage', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    updater.progressSteps = [0.4237];
    await store.check();

    await store.accept();

    expect(store.progressPercent()).toBe(42);
  });

  it('leaves progress indeterminate when the total size is unknown', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    updater.progressSteps = [null];
    await store.check();

    await store.accept();

    expect(store.progressPercent()).toBeNull();
  });

  it('surfaces an install failure and keeps the offer open for a retry', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    updater.installError = new Error('disk full');
    await store.check();

    await store.accept();

    expect(notifier.notice()?.ref.key).toBe('errors.updateFailed');
    expect(notifier.notice()?.detail).toBe('disk full');
    expect(store.status()).toBe('available');
    expect(store.update()).not.toBeNull();
    error.mockRestore();
  });

  it('releases the native resource when the user defers', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    await store.check();

    await store.dismiss();

    // The offer closes, the knowledge stays: the dot is fed by the latter.
    expect(store.offered()).toBeNull();
    expect(store.update()?.version).toBe('0.2.0');
    expect(store.status()).toBe('idle');
    expect(updater.discarded).toBe(true);
  });

  describe('silencing', () => {
    async function offer(version: string): Promise<void> {
      updater.available = { version, currentVersion: '0.1.0' };
      await store.check();
    }

    it('says nothing about a version the user set aside, and still knows it is there', async () => {
      await offer('0.2.0');
      await store.dismiss(true);

      expect(settings.skippedUpdate()).toBe('0.2.0');

      // The next launch, with the same version on the server.
      await offer('0.2.0');

      expect(store.offered()).toBeNull();
      expect(store.hasPendingUpdate()).toBe(true);
    });

    it('offers the one after it without being asked again', async () => {
      settings.setSkippedUpdate('0.2.0');

      await offer('0.2.1');

      expect(store.offered()?.version).toBe('0.2.1');
    });

    it('keeps offering a version the user only deferred', async () => {
      await offer('0.2.0');
      await store.dismiss();

      await offer('0.2.0');

      expect(store.offered()?.version).toBe('0.2.0');
    });

    it('stays quiet on every version once notifications are off', async () => {
      settings.setUpdateNotifications(false);

      await offer('0.2.0');

      expect(store.offered()).toBeNull();
      expect(store.hasPendingUpdate()).toBe(true);
    });

    /** A question asked out loud deserves an answer, whatever the settings say. */
    it('answers an explicit check even for a version it was told to skip', async () => {
      settings.setSkippedUpdate('0.2.0');
      updater.available = { version: '0.2.0', currentVersion: '0.1.0' };

      await store.checkNow();

      expect(store.offered()?.version).toBe('0.2.0');
    });

    it('forgets the skip when the user asks to be told again', async () => {
      settings.setSkippedUpdate('0.2.0');

      settings.setUpdateNotifications(true);

      expect(settings.skippedUpdate()).toBe('');
    });
  });

  it('ignores a dismissal while the installer is running', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    updater.deferInstall = true;
    await store.check();
    const installing = store.accept();

    await store.dismiss();

    expect(store.status()).toBe('installing');
    expect(store.update()).not.toBeNull();

    updater.finishInstall();
    await installing;
  });

  it('confirms in place that the app is up to date, which the startup check never says', async () => {
    await store.check();
    expect(store.checkState()).toBe('upToDate');

    await store.checkNow();

    expect(store.checkState()).toBe('upToDate');
    expect(notifier.notice()).toBeNull();
  });

  it('says nothing in the menu when a check turns up an update — the prompt speaks', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };

    await store.checkNow();

    expect(store.checkState()).toBe('idle');
    expect(store.update()).not.toBeNull();
  });

  it('passes through a checking state so the menu entry can lock', async () => {
    updater.deferCheck = true;
    const running = store.checkNow();

    expect(store.checkState()).toBe('checking');

    updater.finishCheck();
    await running;
    expect(store.checkState()).toBe('upToDate');
  });

  it('surfaces a manual check failure, where the automatic one stays silent', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    updater.checkError = new Error('network unreachable');

    await store.check();
    expect(notifier.notice()).toBeNull();

    await store.checkNow();

    expect(notifier.notice()?.ref.key).toBe('errors.updateCheckFailed');
    expect(store.checkState()).toBe('failed');
    warn.mockRestore();
  });

  it('refuses a manual check while the installer is running', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    updater.deferInstall = true;
    await store.check();
    const installing = store.accept();
    const callsBefore = updater.checkCalls;

    await store.checkNow();

    expect(updater.checkCalls).toBe(callsBefore);

    updater.finishInstall();
    await installing;
  });

  it('ignores a second check while an update is already on the table', async () => {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
    await store.check();

    updater.available = { version: '0.3.0', currentVersion: '0.1.0' };
    await store.check();

    expect(store.update()?.version).toBe('0.2.0');
  });
});
