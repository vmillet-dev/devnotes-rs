import { Directive, inject } from '@angular/core';
import { RAIL_ACTION, areaForKey, isSearchKey, isToolSearchKey } from '@core/services/areas/area.model';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { canvasKeystrokeFromEvent } from '@core/services/shortcuts/shortcut.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { VaultStore } from '@core/state/vault.store';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';

/** Where a key means text: a bare letter there is typed, and Ctrl+B is "bold" in an editor. */
function typesInto(target: EventTarget | null, bare: boolean): boolean {
  if (!(target instanceof HTMLElement)) return false;

  return target.isContentEditable || (bare && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
}

/** From any field too: Ctrl and a digit types nothing anywhere. Never behind a modal. */
@Directive({
  selector: '[appAreaKeys]',
  host: { '(document:keydown)': 'onKeydown($event)' },
})
export class AreaKeysDirective {
  private readonly areas = inject(AreaStore);
  private readonly vault = inject(VaultStore);
  private readonly dialogs = inject(DialogStack);
  private readonly tools = inject(ToolsStore);
  private readonly settings = inject(SettingsStore);
  private readonly bindings = inject(ShortcutBindingsStore);

  protected onKeydown(event: KeyboardEvent): void {
    if (!this.vault.isUnlocked() || this.dialogs.hasOpenDialog() || event.defaultPrevented) return;

    if (isToolSearchKey(event)) {
      event.preventDefault();
      this.areas.show('tools');
      this.tools.requestSearch();
      return;
    }

    // The notes' field takes Ctrl+K itself; the tools' is reached from here, an open tool included.
    if (isSearchKey(event) && this.areas.current() === 'tools') {
      event.preventDefault();
      this.tools.requestSearch();
      return;
    }

    const area = areaForKey(event);
    if (area !== null) {
      event.preventDefault();
      this.areas.show(area);
      return;
    }

    // One answer for the three areas: the rail is one state, so is its key.
    const chord = canvasKeystrokeFromEvent(event);
    if (chord === null || chord !== this.bindings.binding(RAIL_ACTION)) return;
    if (typesInto(event.target, !chord.includes('+'))) return;

    event.preventDefault();
    this.settings.toggleLibraryRail();
  }
}
