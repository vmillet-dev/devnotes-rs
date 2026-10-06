import { ChangeDetectionStrategy, Component, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { debounced } from '@core/services/time/debounce';
import { IconComponent } from '@shared/icon/icon.component';

const FEEDBACK_MS = 2000;

/** "Copier", beside a result: through the native clipboard, which keeps a CRLF a CRLF. */
@Component({
  selector: 'app-copy-value',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './copy-value.component.html',
  styleUrl: './copy-value.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CopyValueComponent {
  private readonly clipboard = inject(ClipboardService);

  readonly value = input.required<string>();
  /** What is copied, for a screen reader: "Copier camelCase". */
  readonly what = input.required<string>();
  /** The icon alone, at the end of a row: « Copier » on hover, « Copié » once done. */
  readonly compact = input(false);
  readonly labelKey = input('tools.copy');

  /** For a row to tint itself while « Copié » shows. */
  readonly copiedChange = output<boolean>();

  protected readonly copied = signal(false);

  private readonly settle = debounced<void>(() => {
    this.copied.set(false);
    this.copiedChange.emit(false);
  }, FEEDBACK_MS);

  protected async copy(): Promise<void> {
    if (!(await this.clipboard.copy(this.value()))) return;

    this.copied.set(true);
    this.copiedChange.emit(true);
    this.settle();
  }
}
