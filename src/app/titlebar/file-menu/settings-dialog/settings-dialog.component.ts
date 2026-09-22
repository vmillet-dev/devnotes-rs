import { NgComponentOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, Type, computed, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
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
 * **No "OK / Cancel / Apply"**: everything applies as it is typed, which is already the
 * application's idiom.
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

  protected readonly pages = PAGES;

  private readonly requestedPageId = signal(GENERAL_PAGE.id);

  protected readonly activePage = computed<SettingsPage>(
    () => PAGES.find((page) => page.id === this.requestedPageId()) ?? GENERAL_PAGE,
  );

  protected select(id: string): void {
    this.requestedPageId.set(id);
  }
}
