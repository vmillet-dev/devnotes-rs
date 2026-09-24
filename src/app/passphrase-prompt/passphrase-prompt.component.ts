import { ChangeDetectionStrategy, Component, computed, inject, linkedSignal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { PassphrasePromptStore, PassphraseRequest } from '@core/state/passphrase-prompt.store';
import { MINIMUM_PASSPHRASE_LENGTH } from '@core/model/vault.model';

/**
 * The phrase a transfer needs, asked for at the moment it is needed.
 *
 * It sits at the root, like the gate: an export starts from the titlebar and a
 * selection export from the canvas header, and a prompt owned by either would be torn
 * down by the menu that closes under it.
 */
@Component({
  selector: 'app-passphrase-prompt',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './passphrase-prompt.component.html',
  styleUrl: './passphrase-prompt.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PassphrasePromptComponent {
  private readonly prompt = inject(PassphrasePromptStore);

  protected readonly request = this.prompt.request;
  protected readonly working = this.prompt.working;

  protected readonly isProtecting = computed(() => this.request()?.purpose === 'protect');

  /** A new request — a retry included — starts from an empty field. */
  protected readonly passphrase = linkedSignal<PassphraseRequest | null, string>({
    source: this.request,
    computation: () => '',
  });

  protected readonly confirmation = linkedSignal<PassphraseRequest | null, string>({
    source: this.request,
    computation: () => '',
  });

  /** Typing again is what withdraws the refusal; it must not outlive the correction. */
  protected readonly refused = linkedSignal<PassphraseRequest | null, boolean>({
    source: this.request,
    computation: (request) => request?.refused ?? false,
  });

  protected readonly tooShort = computed(
    () =>
      this.isProtecting() &&
      this.passphrase().length > 0 &&
      this.passphrase().length < MINIMUM_PASSPHRASE_LENGTH,
  );

  protected readonly mismatched = computed(
    () => this.isProtecting() && this.confirmation().length > 0 && this.confirmation() !== this.passphrase(),
  );

  protected readonly canSubmit = computed(() => {
    if (this.working()) return false;
    if (!this.isProtecting()) return this.passphrase().length > 0;

    return this.passphrase().length >= MINIMUM_PASSPHRASE_LENGTH && this.confirmation() === this.passphrase();
  });

  protected readonly minimumLength = MINIMUM_PASSPHRASE_LENGTH;

  protected onPassphrase(value: string): void {
    this.passphrase.set(value);
    this.refused.set(false);
  }

  protected onConfirmation(value: string): void {
    this.confirmation.set(value);
  }

  /**
   * The fields are cleared before the answer leaves: a phrase left in a DOM node
   * outlives the dialog, and the store never holds one either.
   */
  protected submit(event: Event): void {
    event.preventDefault();
    if (!this.canSubmit()) return;

    const typed = this.passphrase();
    this.clear();
    this.prompt.answer({ kind: 'phrase', value: typed });
  }

  /** The plain export, taken deliberately: the warning above the button is the point. */
  protected exportInTheClear(): void {
    if (this.working()) return;

    this.clear();
    this.prompt.answer({ kind: 'none' });
  }

  /** Refused while a phrase is being derived from: the operation is under way, and
   *  the dialog is showing that rather than waiting for an answer. */
  protected cancel(): void {
    if (this.working()) return;

    this.clear();
    this.prompt.answer({ kind: 'cancelled' });
  }

  private clear(): void {
    this.passphrase.set('');
    this.confirmation.set('');
  }
}
