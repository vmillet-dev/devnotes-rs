import { ChangeDetectionStrategy, Component, computed, inject, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { MINIMUM_PASSPHRASE_LENGTH } from '@core/model/vault.model';
import { VaultStore } from '@core/state/vault.store';

/**
 * Changing the phrase re-wraps the library's key; it re-encrypts nothing. So there is
 * no progress to report beyond the two derivations, and nothing to roll back.
 */
@Component({
  selector: 'app-change-passphrase-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './change-passphrase-dialog.component.html',
  styleUrl: './change-passphrase-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChangePassphraseDialogComponent {
  protected readonly vault = inject(VaultStore);

  readonly closed = output<void>();

  protected readonly current = signal('');
  protected readonly next = signal('');
  protected readonly confirmation = signal('');

  protected readonly tooShort = computed(
    () => this.next().length > 0 && this.next().length < MINIMUM_PASSPHRASE_LENGTH,
  );

  protected readonly mismatched = computed(
    () => this.confirmation().length > 0 && this.confirmation() !== this.next(),
  );

  protected readonly canSubmit = computed(() => {
    if (this.vault.isWorking() || this.current().length === 0) return false;

    return this.next().length >= MINIMUM_PASSPHRASE_LENGTH && this.confirmation() === this.next();
  });

  protected readonly minimumLength = MINIMUM_PASSPHRASE_LENGTH;

  protected onCurrent(value: string): void {
    this.current.set(value);
    this.vault.clearRefusal();
  }

  protected onNext(value: string): void {
    this.next.set(value);
  }

  protected onConfirmation(value: string): void {
    this.confirmation.set(value);
  }

  protected close(): void {
    if (this.vault.isWorking()) return;

    this.closed.emit();
  }

  /**
   * The three fields are cleared before the round trip, success or not: a phrase left
   * in a DOM node outlives the dialog.
   */
  protected async submit(event: Event): Promise<void> {
    event.preventDefault();
    if (!this.canSubmit()) return;

    const from = this.current();
    const to = this.next();
    this.current.set('');
    this.next.set('');
    this.confirmation.set('');

    // The report comes from the store, which is what knows how much the change
    // reached. This only closes.
    if (await this.vault.changePassphrase(from, to)) {
      this.closed.emit();
    }
  }
}
