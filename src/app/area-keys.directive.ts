import { Directive, Injector, inject } from '@angular/core';
import { Area, RAIL_ACTION, areaForKey, isSearchKey, isToolSearchKey } from '@core/services/areas/area.model';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { canvasKeystrokeFromEvent } from '@core/services/shortcuts/shortcut.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { VaultStore } from '@core/state/vault.store';
import { focusCard } from '@notes/canvas/focus-card';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';

/** Where a key means text: a bare letter there is typed, and Ctrl+B is "bold" in an editor. */
function typesInto(target: EventTarget | null, bare: boolean): boolean {
  if (!(target instanceof HTMLElement)) return false;

  return target.isContentEditable || (bare && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
}

const FOCUSABLE =
  'input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [contenteditable="true"], button:not([disabled])';

/** The first element of the first of these places that can take the keyboard. */
function focusWithin(...places: string[]): boolean {
  for (const place of places) {
    const target = document.querySelector(place)?.querySelector<HTMLElement>(FOCUSABLE);
    if (target) {
      target.focus();
      if (document.activeElement === target) return true;
    }
  }
  return false;
}

/** Frames given to an area to appear: the tools and the requests are created as they are shown. */
const LANDING_FRAMES = 60;

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
  /** ⚠️ Asked for on landing, never at construction: the canvas's stores load the library, locked at startup. */
  private readonly injector = inject(Injector);

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
      this.land(area, LANDING_FRAMES);
      return;
    }

    // One answer for the three areas: the rail is one state, so is its key.
    const chord = canvasKeystrokeFromEvent(event);
    if (chord === null || chord !== this.bindings.binding(RAIL_ACTION)) return;
    if (typesInto(event.target, !chord.includes('+'))) return;

    event.preventDefault();
    this.settings.toggleLibraryRail();
  }

  /** The keyboard goes with the area: the card last focused, the tool open, the request. */
  private land(area: Area, frames: number): void {
    requestAnimationFrame(() => {
      if (!this.landed(area) && frames > 1) this.land(area, frames - 1);
    });
  }

  private landed(area: Area): boolean {
    switch (area) {
      case 'notes': {
        const selection = this.injector.get(NoteSelectionStore);
        return focusCard(selection, selection.focusedNoteId()) || focusWithin('app-search-box');
      }
      case 'tools':
        return focusWithin('[data-testid="tool-body"]', '[data-testid="tools-page"] .workspace');
      case 'http':
        return focusWithin('[data-testid="http-split"]', '[data-testid="http-page"] .workspace');
    }
  }
}
