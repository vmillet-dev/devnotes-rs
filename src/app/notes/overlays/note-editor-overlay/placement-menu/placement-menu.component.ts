import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FolderColour } from '@core/model/folder.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** A space or a folder, which is all this control needs to know about either. */
export interface PlacementOption {
  readonly id: string;
  readonly name: string;
  /** Only a folder has one, and it is the same swatch the card's chip draws. */
  readonly colour?: FolderColour;
}

/**
 * Where a note sits, as a menu rather than a `<select>`: it is a command, not a field.
 *
 * ⚠️ One component for both placements, and the editor shows them **together**. A folder
 * belongs to one space, so `NotePatch::apply` clears `folder_id` whenever `space_id` moves
 * — a folder control without the space control beside it would lie about what the other
 * one just did.
 */
@Component({
  selector: 'app-placement-menu',
  imports: [TranslocoPipe, MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  // ⚠️ This one lives **inside a dialog**, unlike every other menu in the application. The
  // trigger directive lets Escape bubble on purpose — a multi-level menu folds its panel
  // before closing — but here the next listener up is the editor's own, so one Escape
  // closed the note along with the menu. Swallowed only while the panel is open.
  host: { '(keydown.escape)': 'onEscape($event)' },
  templateUrl: './placement-menu.component.html',
  styleUrl: './placement-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PlacementMenuComponent {
  readonly label = input.required<string>();
  /** ⚠️ Stable, unlike `label`, which is translated: this is what the tests address. */
  readonly kind = input.required<'space' | 'folder'>();
  readonly options = input.required<readonly PlacementOption[]>();
  readonly currentId = input<string | null>(null);
  /** A note always has a space; only a folder can be left out of. */
  readonly noneLabel = input<string | null>(null);

  readonly chosen = output<string | null>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly current = computed(
    () => this.options().find((option) => option.id === this.currentId()) ?? null,
  );

  /**
   * ⚠️ Closed from here rather than from `escaped`, which is the trigger directive's own
   * listener on the same element: subscribing to it would close the menu **before** this
   * runs, and there would be nothing left to justify swallowing the key. One handler, one
   * decision, and no dependence on which listener was registered first.
   */
  protected onEscape(event: Event): void {
    if (!this.menu.open()) return;

    event.stopPropagation();
    this.menu.close();
  }

  protected pick(id: string | null): void {
    this.menu.close();
    if (id === this.currentId()) return;

    this.chosen.emit(id);
  }
}
