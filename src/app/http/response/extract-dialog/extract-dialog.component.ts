import { ChangeDetectionStrategy, Component, computed, input, linkedSignal, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ExtractionRow, looksSecret, variableNameOf } from '@core/model/http.model';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/**
 * « Extraire vers une variable » : the value at a path of the answer becomes a rule of the
 * request, written after each of its sends into the active environment.
 */
@Component({
  selector: 'app-extract-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './extract-dialog.component.html',
  styleUrl: './extract-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExtractDialogComponent {
  /** As the rule reads it: `body.access_token`. */
  readonly path = input.required<string>();
  readonly requestName = input.required<string>();
  /** `null` with « Aucun »: the globals. */
  readonly environmentName = input<string | null>(null);

  readonly extracted = output<ExtractionRow>();
  readonly closed = output<void>();

  /** The key in camelCase; and secret for a key that reads as a token, a password or a key. */
  protected readonly name = linkedSignal(() => variableNameOf(this.path()));
  protected readonly secret = linkedSignal(() => looksSecret(this.path()));
  protected readonly valid = computed(() => this.name().trim() !== '');

  protected onName(event: Event): void {
    this.name.set((event.target as HTMLInputElement).value);
  }

  protected onSecret(event: Event): void {
    this.secret.set((event.target as HTMLInputElement).checked);
  }

  protected submit(event: Event): void {
    event.preventDefault();
    if (!this.valid()) return;
    this.extracted.emit({ path: this.path(), variable: this.name().trim(), secret: this.secret() });
  }
}
