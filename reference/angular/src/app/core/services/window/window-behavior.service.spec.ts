import { TestBed } from '@angular/core/testing';
import { type MockInstance, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@core/ipc/bindings';
import { SettingsStore } from '@core/services/settings/settings.store';
import { WindowBehaviorService } from './window-behavior.service';

describe('WindowBehaviorService', () => {
  let setWindowBehavior: MockInstance<typeof commands.setWindowBehavior>;
  let service: WindowBehaviorService;
  let settings: SettingsStore;

  function lastBehavior() {
    return setWindowBehavior.mock.calls.at(-1)![0];
  }

  beforeEach(() => {
    setWindowBehavior = vi.spyOn(commands, 'setWindowBehavior').mockResolvedValue(undefined);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    settings = TestBed.inject(SettingsStore);
    service = TestBed.inject(WindowBehaviorService);
  });

  it('pushes what the close and minimise buttons should do', () => {
    service.start();
    TestBed.tick();

    expect(lastBehavior()).toEqual({ closeToTray: true, minimizeToTray: false });
  });

  it('pushes again when the preference changes', () => {
    service.start();
    TestBed.tick();

    settings.setCloseToTray(false);
    TestBed.tick();

    expect(lastBehavior()).toEqual({ closeToTray: false, minimizeToTray: false });
  });

  it('stays silent when there is no bridge to talk to', async () => {
    setWindowBehavior.mockRejectedValue(new Error('no bridge'));

    expect(() => service.start()).not.toThrow();
    TestBed.tick();
    await Promise.resolve();
  });
});
