import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { FolderColour } from '@core/model/folder.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** Anything the menu can offer: a space, a folder, a language, a theme. */
export interface ChoiceOption {
  readonly id: string;
  readonly name: string;
  /** Only a folder has one, and it is the same swatch the card's chip draws. */
  readonly colour?: FolderColour;
}

/**
 * One choice among a list, as a menu rather than a `<select>`.
 *
 * ⚠️ The application draws every one of its own surfaces, and a native `<select>` brings
 * the operating system's back: a different border, a different arrow, a different focus
 * ring and, on Windows, a different font. Worse where the control is a **command** rather
 * than a field — the selection bar's two reset their value to `''` after every `change`, so
 * a screen reader announced a combobox whose current value was "Ranger dans".
 *
 * ⚠️ `naming: 'label'` is what tells the two apart. A field shows what is chosen; a command
 * shows what it does, and has no current value to announce.
 */
@Component({
  selector: 'app-choice-menu',
  imports: [MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  // ⚠️ This one is used **inside dialogs**, unlike every other menu in the application.
  // The trigger directive lets Escape bubble on purpose — a multi-level menu folds its
  // panel before closing — but the next listener up there is the dialog's own, so one
  // Escape closed the editor along with the menu. Swallowed only while the panel is open.
  host: { '(keydown.escape)': 'onEscape($event)' },
  templateUrl: './choice-menu.component.html',
  styleUrl: './choice-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChoiceMenuComponent {
  readonly label = input.required<string>();
  /** ⚠️ Stable, unlike `label`, which is translated: this is what the tests address. */
  readonly kind = input.required<string>();
  readonly options = input.required<readonly ChoiceOption[]>();
  readonly currentId = input<string | null>(null);
  /** The entry that chooses none of them — a folder can be left, a space cannot. */
  readonly noneLabel = input<string | null>(null);
  /** `value` names what is chosen; `label` names what the menu does. */
  readonly naming = input<'value' | 'label'>('value');

  readonly chosen = output<string | null>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly current = computed(
    () => this.options().find((option) => option.id === this.currentId()) ?? null,
  );

  protected readonly triggerText = computed(() =>
    this.naming() === 'label' ? this.label() : (this.current()?.name ?? this.noneLabel() ?? this.label()),
  );

  /** A command has no current entry to mark, and nothing to leave. */
  protected readonly showsNone = computed(() => this.naming() === 'value' && this.noneLabel() !== null);

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
    // Asking for where it already is is asking for nothing — but a command has no "already".
    if (this.naming() === 'value' && id === this.currentId()) return;

    this.chosen.emit(id);
  }
}
