import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { HelpStore } from '@core/services/help/help.store';
import { UpdateStore } from '@core/services/updates/update.store';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';
import { AboutDialogComponent } from '@titlebar/about-menu/about-dialog/about-dialog.component';
import { GettingStartedDialogComponent } from '@titlebar/about-menu/getting-started-dialog/getting-started-dialog.component';
import { ShortcutsDialogComponent } from '@titlebar/about-menu/shortcuts-dialog/shortcuts-dialog.component';
import { WhatsNewDialogComponent } from '@titlebar/about-menu/whats-new-dialog/whats-new-dialog.component';

/** One signal rather than four booleans, which would allow a state with two stacked. */
export type AboutPanel = 'whatsNew' | 'gettingStarted' | 'shortcuts' | 'about';

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
  /** ⚠️ Shared, because the guide is reached from the thing it explains as well as here. */
  protected readonly help = inject(HelpStore);
  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly panel = signal<AboutPanel | null>(null);

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

  /**
   * ⚠️ The guide is the one panel with a second way in: `HelpStore` is what an empty canvas
   * or an empty board opens it through, so the state has to be the same either way.
   */
  protected readonly showing = computed<AboutPanel | null>(() =>
    this.help.chapter() !== null ? 'gettingStarted' : this.panel(),
  );

  protected openPanel(panel: AboutPanel): void {
    if (panel === 'gettingStarted') {
      this.help.open();
    } else {
      this.panel.set(panel);
    }
    // No focus restored: the modal opening takes it itself.
    this.menu.close(false);
  }

  /** The modal's focus trap would hand back to the menu entry, destroyed since. */
  protected closePanel(): void {
    this.panel.set(null);
    this.help.close();
    this.menu.focusAnchor();
  }
}
