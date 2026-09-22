import { NgComponentOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, Type, computed, inject, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { VariablesStore } from '@core/state/variables.store';
import { VariablesPageComponent } from '@titlebar/file-menu/settings-dialog/variables-page/variables-page.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { SecurityPageComponent } from './security-page/security-page.component';
import { SettingsPageComponent } from './settings-page/settings-page.component';
import { ShortcutsPageComponent } from './shortcuts-page/shortcuts-page.component';

/** `component` is rendered through `NgComponentOutlet`: the rail is one list, the pages
 * are components the panel only hosts. */
interface SettingsPage {
  readonly id: string;
  readonly labelKey: string;
  readonly component: Type<unknown>;
}

const GENERAL_PAGE: SettingsPage = {
  id: 'general',
  labelKey: 'settings.pages.general',
  component: SettingsPageComponent,
};

/**
 * The order on screen is the order here: what the application looks like, then the keys
 * that reach it, then what protects the library, then the corpus's own variables.
 */
const PAGES: readonly SettingsPage[] = [
  GENERAL_PAGE,
  { id: 'shortcuts', labelKey: 'settings.pages.shortcuts', component: ShortcutsPageComponent },
  { id: 'security', labelKey: 'settings.pages.security', component: SecurityPageComponent },
  { id: 'notes.variables', labelKey: 'settings.pages.variables', component: VariablesPageComponent },
];

/**
 * The controls edit `SettingsDraftStore`; Appliquer and OK write it through, Annuler
 * drops it.
 *
 * ⚠️ Applying as it was typed cost more than it bought: a shortcut is **captured**, so a
 * half-entered one was live across the whole machine until it was finished, and there
 * was no way back from a change other than remembering what it was. What that idiom did
 * buy is kept where it means something — the theme and the density show on screen while
 * they are chosen, without anything reaching the file.
 */
@Component({
  selector: 'app-settings-dialog',
  imports: [DialogComponent, NgComponentOutlet, TranslocoPipe],
  templateUrl: './settings-dialog.component.html',
  styleUrl: './settings-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsDialogComponent {
  readonly closed = output<void>();

  protected readonly draft = inject(SettingsDraftStore);
  /**
   * ⚠️ The variables are corpus data rather than a preference, so they have a store of
   * their own — but they are edited under the same footer, and a panel where one page
   * commits on blur and three wait for a button would be worse than either rule alone.
   * The panel is what holds the three of them to one gesture.
   */
  private readonly variables = inject(VariablesStore);

  protected readonly pages = PAGES;

  protected readonly hasPending = computed(() => this.draft.isDirty() || this.variables.isDirty());

  protected readonly pendingCount = computed(
    () => this.draft.pendingCount() + (this.variables.isDirty() ? 1 : 0),
  );

  private readonly requestedPageId = signal(GENERAL_PAGE.id);

  /**
   * ⚠️ Shown in place of the buttons once a close was attempted with work in hand.
   * Escape and the backdrop produce no click, so without it either one is a silent
   * Annuler — and Annuler is the one outcome nobody would have chosen by accident.
   */
  protected readonly confirmingClose = signal(false);

  protected readonly activePage = computed<SettingsPage>(
    () => PAGES.find((page) => page.id === this.requestedPageId()) ?? GENERAL_PAGE,
  );

  protected select(id: string): void {
    this.requestedPageId.set(id);
  }

  protected apply(): void {
    this.draft.apply();
    if (this.variables.isDirty()) void this.variables.commit();
  }

  protected requestClose(): void {
    if (this.hasPending()) {
      this.confirmingClose.set(true);
      return;
    }

    this.finish();
  }

  protected applyAndClose(): void {
    this.apply();
    this.finish();
  }

  protected discardAndClose(): void {
    this.draft.cancel();
    void this.variables.discard();
    this.finish();
  }

  /** Every way out passes here, so the draft is always empty when the panel reopens. */
  private finish(): void {
    this.confirmingClose.set(false);
    this.closed.emit();
  }
}
