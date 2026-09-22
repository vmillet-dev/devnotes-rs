import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ChangePassphraseDialogComponent } from '../change-passphrase-dialog/change-passphrase-dialog.component';

/**
 * The two things that decide who can read the library: the phrase that unwraps its key,
 * and whether a copy of it is taken at every launch.
 */
@Component({
  selector: 'app-security-page',
  imports: [TranslocoPipe, ChangePassphraseDialogComponent],
  templateUrl: './security-page.component.html',
  styleUrl: './security-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SecurityPageComponent {
  protected readonly settings = inject(SettingsStore);

  protected readonly isChangingPassphrase = signal(false);

  protected openPassphraseChange(): void {
    this.isChangingPassphrase.set(true);
  }

  protected closePassphraseChange(): void {
    this.isChangingPassphrase.set(false);
  }

  protected onAutomaticBackups(event: Event): void {
    this.settings.setAutomaticBackups((event.target as HTMLInputElement).checked);
  }
}
