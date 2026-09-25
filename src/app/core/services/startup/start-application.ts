import { inject } from '@angular/core';
import { AutostartService } from '@core/services/autostart/autostart.service';
import { LocaleService } from '@core/services/i18n/locale.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { GlobalShortcutsService } from '@core/services/shortcuts/global-shortcuts.service';
import { TrayService } from '@core/services/tray/tray.service';
import { WindowBehaviorService } from '@core/services/window/window-behavior.service';
import { LibrariesStore } from '@core/state/libraries.store';
import { VaultStore } from '@core/state/vault.store';

/**
 * The boot sequence, run once before the first render. Its order is the point, and each
 * step says what it must come after.
 *
 * ⚠️ Everything is injected before the first `await`: an `inject()` after one leaves the
 * injection context and fails the bootstrap (NG0203) with a black window and nothing else.
 */
export async function startApplication(): Promise<void> {
  const preferences = inject(PreferencesService);
  const locale = inject(LocaleService);
  const settings = inject(SettingsStore);
  const tray = inject(TrayService);
  const shortcuts = inject(GlobalShortcutsService);
  const windowBehavior = inject(WindowBehaviorService);
  const autostart = inject(AutostartService);
  const vault = inject(VaultStore);
  const libraries = inject(LibrariesStore);

  await preferences.hydrate();
  // Before `locale.restore()`, which reads the language out of it.
  settings.restore();
  await locale.restore();
  // After `restore()`: the front creates the tray by giving it its labels, and earlier
  // would push the default language and a setting the user had changed.
  tray.start();

  // Before the vault: it says which library is open, and opens that library's own
  // preference file — reading the wrong one seeds a library that is full.
  await libraries.load();

  // Before the first render: the shell renders nothing until this answers, rather than
  // flashing a canvas it is about to replace with an unlock screen.
  await vault.load();

  shortcuts.start();
  windowBehavior.start();
  // Not awaited: asking the system must not delay the first render.
  void autostart.start();
}
