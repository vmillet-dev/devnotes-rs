import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { MINIMUM_PASSPHRASE_LENGTH } from '@core/model/vault.model';
import { LibrariesStore } from '@core/state/libraries.store';
import { VaultStore } from '@core/state/vault.store';
import { ChoiceMenuComponent, ChoiceOption } from '../notes/ui/choice-menu/choice-menu.component';

/**
 * The screen that stands in front of everything until the library is open.
 *
 * The gate is here, at the root, and not in each store: the canvas is never mounted
 * while the library is locked, so no store has to hold a "locked" branch and no command
 * is called before it can be answered.
 */
@Component({
  selector: 'app-vault-gate',
  imports: [ChoiceMenuComponent, NgTemplateOutlet, TranslocoPipe],
  templateUrl: './vault-gate.component.html',
  styleUrl: './vault-gate.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VaultGateComponent {
  protected readonly vault = inject(VaultStore);
  protected readonly libraries = inject(LibrariesStore);

  private readonly unnamed = toSignal(inject(TranslocoService).selectTranslate<string>('libraries.unnamed'), {
    initialValue: '',
  });

  protected readonly libraryOptions = computed<readonly ChoiceOption[]>(() =>
    this.libraries.libraries().map((entry) => ({ id: entry.id, name: entry.name.trim() || this.unnamed() })),
  );

  /** Empty for the one library that predates names: "Library: Library" says nothing. */
  protected readonly libraryName = computed(() => this.libraries.open()?.name.trim() ?? '');

  protected readonly passphrase = signal('');
  protected readonly confirmation = signal('');

  protected readonly isCreating = computed(() => this.vault.needsCreating());

  /** Not while unlocking: a wrong phrase is the key file's to refuse, whatever its length. */
  protected readonly tooShort = computed(
    () =>
      this.isCreating() &&
      this.passphrase().length > 0 &&
      this.passphrase().length < MINIMUM_PASSPHRASE_LENGTH,
  );

  protected readonly mismatched = computed(
    () => this.isCreating() && this.confirmation().length > 0 && this.confirmation() !== this.passphrase(),
  );

  protected readonly canSubmit = computed(() => {
    if (this.vault.isWorking() || this.passphrase().length === 0) return false;
    if (!this.isCreating()) return true;

    return this.passphrase().length >= MINIMUM_PASSPHRASE_LENGTH && this.confirmation() === this.passphrase();
  });

  protected readonly minimumLength = MINIMUM_PASSPHRASE_LENGTH;

  private readonly passphraseField = viewChild<ElementRef<HTMLInputElement>>('passphraseField');

  /**
   * Shown instead of the form, not beside it. The field is the one thing that cannot
   * help here, and an offer to give up standing next to it would be read as a shortcut.
   */
  protected readonly isConfirmingArchive = signal(false);

  constructor() {
    // Not the `autofocus` attribute, which the linter refuses: this screen is the only
    // thing there is, and the user opened the application to type into this field.
    afterNextRender(() => this.passphraseField()?.nativeElement.focus());
  }

  /** The way out of a damaged library; the store says where everything went. */
  protected async setAside(): Promise<void> {
    await this.vault.setAsideDamagedLibrary();
  }

  protected askToArchive(): void {
    this.passphrase.set('');
    this.isConfirmingArchive.set(true);
  }

  protected keepTrying(): void {
    this.isConfirmingArchive.set(false);
  }

  /**
   * Recovers nothing, and the sentence above it says so: the notes leave **sealed**,
   * under the phrase nobody remembers. The store re-reads the state afterwards, and with
   * the key file gone the gate comes back asking for a new phrase rather than one nobody
   * has.
   */
  protected async archive(): Promise<void> {
    if (await this.vault.archiveLockedLibrary()) {
      this.isConfirmingArchive.set(false);
    }
  }

  /**
   * From here, because the File menu does not exist until a library is open — and the
   * gate was asking for a phrase the user may not have for this one.
   */
  protected async switchLibrary(id: string | null): Promise<void> {
    if (id === null) return;

    this.passphrase.set('');
    this.confirmation.set('');
    await this.libraries.openLibrary(id);
  }

  protected onPassphrase(value: string): void {
    this.passphrase.set(value);
    this.vault.clearRefusal();
  }

  protected onConfirmation(value: string): void {
    this.confirmation.set(value);
  }

  /**
   * The field is cleared whatever happens, success included: a passphrase left in a
   * DOM node is a passphrase in a memory dump, and the store never held it either.
   */
  protected async submit(event: Event): Promise<void> {
    event.preventDefault();
    if (!this.canSubmit()) return;

    const typed = this.passphrase();
    const created = this.isCreating();

    this.passphrase.set('');
    this.confirmation.set('');

    if (created) {
      await this.vault.create(typed);
    } else {
      await this.vault.unlock(typed);
    }
  }
}
