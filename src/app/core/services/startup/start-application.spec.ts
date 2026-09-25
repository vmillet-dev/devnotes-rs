import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { AutostartService } from '@core/services/autostart/autostart.service';
import { LocaleService } from '@core/services/i18n/locale.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { GlobalShortcutsService } from '@core/services/shortcuts/global-shortcuts.service';
import { TrayService } from '@core/services/tray/tray.service';
import { WindowBehaviorService } from '@core/services/window/window-behavior.service';
import { LibrariesStore } from '@core/state/libraries.store';
import { VaultStore } from '@core/state/vault.store';
import { startApplication } from './start-application';

describe('startApplication', () => {
  let order: string[];

  /** Each step records itself; the asynchronous ones yield first, as the real ones do. */
  function step(name: string): () => void {
    return () => void order.push(name);
  }

  function later(name: string): () => Promise<void> {
    return async () => {
      await Promise.resolve();
      order.push(name);
    };
  }

  function pending(name: string): () => Promise<void> {
    return () => {
      order.push(name);
      return new Promise<void>(() => undefined);
    };
  }

  beforeEach(() => {
    order = [];
    TestBed.configureTestingModule({
      providers: [
        { provide: PreferencesService, useValue: { hydrate: later('preferences') } },
        { provide: SettingsStore, useValue: { restore: step('settings') } },
        { provide: LocaleService, useValue: { restore: later('locale') } },
        { provide: TrayService, useValue: { start: step('tray') } },
        { provide: LibrariesStore, useValue: { load: later('libraries') } },
        { provide: VaultStore, useValue: { load: later('vault') } },
        { provide: GlobalShortcutsService, useValue: { start: step('shortcuts') } },
        { provide: WindowBehaviorService, useValue: { start: step('window') } },
        // Asked and never answered: were it awaited, the sequence would never finish.
        { provide: AutostartService, useValue: { start: pending('autostart') } },
      ],
    });
  });

  it('runs every step, in the order each one depends on', async () => {
    await TestBed.runInInjectionContext(() => startApplication());

    expect(order).toEqual([
      'preferences',
      'settings',
      'locale',
      'tray',
      'libraries',
      'vault',
      'shortcuts',
      'window',
      'autostart',
    ]);
  });

  /** The system is asked about autostart, and the first render does not wait for it. */
  it('finishes without an answer from autostart', async () => {
    await expect(TestBed.runInInjectionContext(() => startApplication())).resolves.toBeUndefined();
  });
});
