import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { HelpPanel, HelpStore } from '@core/services/help/help.store';
import { UpdateStore } from '@core/services/updates/update.store';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';
import { dialogRung } from '@shared/layout/dialog/dialog.model';
import { AboutDialogComponent } from '@titlebar/about-menu/about-dialog/about-dialog.component';
import { GettingStartedDialogComponent } from '@titlebar/about-menu/getting-started-dialog/getting-started-dialog.component';
import { ShortcutsDialogComponent } from '@titlebar/about-menu/shortcuts-dialog/shortcuts-dialog.component';
import { WhatsNewDialogComponent } from '@titlebar/about-menu/whats-new-dialog/whats-new-dialog.component';

@Component({
  selector: 'app-about-menu',
  imports: [
    TranslocoPipe,
    AboutDialogComponent,
    GettingStartedDialogComponent,
    ShortcutsDialogComponent,
    WhatsNewDialogComponent,
    MenuPanelDirective,
  ],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './about-menu.component.html',
  styleUrl: './about-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AboutMenuComponent {
  protected readonly store = inject(UpdateStore);
  /** Shared: the guide is reached from the thing it explains, and a global shortcut puts it away. */
  protected readonly help = inject(HelpStore);
  protected readonly menu = inject(MenuTriggerDirective);

  /** Over a full-screen editor, the one modal that leaves the titlebar in reach. */
  protected readonly menuRung = dialogRung('titlebar');

  protected readonly checking = computed(() => this.store.checkState() === 'checking');

  /** Exhaustive rather than defaulted: a state added to `CheckState` breaks the build. */
  protected readonly checkStatusRef = computed<TranslationRef | null>(() => {
    switch (this.store.checkState()) {
      case 'checking':
        return { key: 'about.checking' };
      case 'upToDate':
        return { key: 'about.upToDate' };
      case 'failed':
        return { key: 'about.checkFailed' };
      case 'idle':
        return null;
    }
  });

  protected checkUpdates(): void {
    if (this.checking()) return;
    void this.store.checkNow();
  }

  protected openPanel(panel: HelpPanel): void {
    if (panel === 'gettingStarted') {
      this.help.open();
    } else {
      this.help.show(panel);
    }
    // No focus restored: the modal opening takes it itself.
    this.menu.close(false);
  }

  /** The modal's focus trap would hand back to the menu entry, destroyed since. */
  protected closePanel(): void {
    this.help.close();
    this.menu.focusAnchor();
  }
}
