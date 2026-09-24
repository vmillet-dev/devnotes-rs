import { ChangeDetectionStrategy, Component, computed, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { GLOBAL_ACTIONS, ShortcutGroup, acceleratorKeys } from '@core/services/shortcuts/shortcut.model';
import { NOTES_SHORTCUT_GROUPS } from '@titlebar/about-menu/shortcuts-dialog/notes-shortcuts';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/**
 * Every row that can be moved is resolved through `ShortcutBindingsStore`, so the
 * sheet shows the key that is really bound rather than the one that shipped — the rest
 * are drawn as declared, being the ones nothing can move.
 */
@Component({
  selector: 'app-shortcuts-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './shortcuts-dialog.component.html',
  styleUrl: './shortcuts-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ShortcutsDialogComponent {
  private readonly bindings = inject(ShortcutBindingsStore);

  readonly closed = output<void>();

  protected readonly groups = computed<readonly ShortcutGroup[]>(() =>
    [
      {
        id: 'global',
        labelKey: 'shortcuts.groups.global',
        // First: these keys work with the window closed.
        shortcuts: GLOBAL_ACTIONS.map((action) => ({
          keys: acceleratorKeys(action.fallback),
          labelKey: action.labelKey,
          action,
        })),
      },
      ...NOTES_SHORTCUT_GROUPS,
    ].map((group) => ({
      ...group,
      shortcuts: group.shortcuts.map((entry) => ({
        ...entry,
        keys: entry.action ? acceleratorKeys(this.bindings.binding(entry.action)) : entry.keys,
      })),
    })),
  );
}
