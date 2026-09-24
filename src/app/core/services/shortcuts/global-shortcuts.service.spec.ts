import { TestBed } from '@angular/core/testing';
import { type MockInstance, beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { commands } from '@core/ipc/bindings';
import { SettingsStore } from '@core/services/settings/settings.store';
import { GlobalShortcutsService } from './global-shortcuts.service';

describe('GlobalShortcutsService', () => {
  // The generated bindings call `invoke` directly: what a spec substitutes is the commands
  // object.
  let setGlobalShortcuts: MockInstance<typeof commands.setGlobalShortcuts>;
  let service: GlobalShortcutsService;
  let settings: SettingsStore;

  /** Combinations of the last call. */
  function lastBindings() {
    return setGlobalShortcuts.mock.calls.at(-1)![0];
  }

  beforeEach(() => {
    setGlobalShortcuts = vi.spyOn(commands, 'setGlobalShortcuts').mockResolvedValue([]);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    settings = TestBed.inject(SettingsStore);
    service = TestBed.inject(GlobalShortcutsService);
  });

  it('pushes the three shortcuts, the palette carrying the preference', () => {
    service.start();
    TestBed.tick();

    expect(lastBindings()).toEqual({
      capture: 'Ctrl+Alt+V',
      newNote: 'Ctrl+Alt+N',
      palette: 'Ctrl+Alt+P',
    });
  });

  it('pushes again when the palette shortcut changes', () => {
    service.start();
    TestBed.tick();

    settings.setPaletteShortcut('Ctrl+Shift+K');
    TestBed.tick();

    expect(lastBindings().palette).toBe('Ctrl+Shift+K');
  });

  it('says which combinations another application already holds', async () => {
    setGlobalShortcuts.mockResolvedValue(['Ctrl+Alt+P']);
    service.start();
    TestBed.tick();
    await Promise.resolve();

    // The count travels beside the list: the sentence agrees three times over, and a
    // list is not something a translation can count.
    expect(TestBed.inject(ErrorNotifier).notice()?.ref).toEqual({
      key: 'shortcuts.unavailable',
      params: { count: 1, list: 'Ctrl+Alt+P' },
    });
  });

  it('stays silent when everything was taken', async () => {
    service.start();
    TestBed.tick();
    await Promise.resolve();

    expect(TestBed.inject(ErrorNotifier).notice()).toBeNull();
  });

  it('stays silent when there is no bridge to talk to', async () => {
    setGlobalShortcuts.mockRejectedValue(new Error('no bridge'));

    expect(() => service.start()).not.toThrow();
    TestBed.tick();
    await Promise.resolve();

    expect(TestBed.inject(ErrorNotifier).notice()).toBeNull();
  });
});
