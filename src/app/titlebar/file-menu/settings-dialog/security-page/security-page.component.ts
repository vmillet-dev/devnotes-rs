import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
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
  protected readonly draft = inject(SettingsDraftStore);

  protected readonly isChangingPassphrase = signal(false);

  protected openPassphraseChange(): void {
    this.isChangingPassphrase.set(true);
  }

  protected closePassphraseChange(): void {
    this.isChangingPassphrase.set(false);
  }

  protected onAutomaticBackups(event: Event): void {
    this.draft.set('automaticBackups', (event.target as HTMLInputElement).checked);
  }
}
