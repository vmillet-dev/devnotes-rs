import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { KeyValueRow } from '@core/model/http.model';

type Field = 'key' | 'value' | 'description';

const BLANK: KeyValueRow = { enabled: true, key: '', value: '', description: '' };

/**
 * A request's parameters or headers: a row to tick, its key, value and description, and one
 * blank row at the end that becomes a row as soon as something is typed in it.
 */
@Component({
  selector: 'app-key-value-table',
  imports: [TranslocoPipe],
  templateUrl: './key-value-table.component.html',
  styleUrl: './key-value-table.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class KeyValueTableComponent {
  readonly rows = input.required<readonly KeyValueRow[]>();
  /** What names the table to a screen reader: « Paramètres », « En-têtes ». */
  readonly label = input.required<string>();
  /** Names a key field may propose, as a `<datalist>`. */
  readonly suggestions = input<readonly string[]>([]);
  readonly kind = input.required<string>();

  readonly rowsChange = output<readonly KeyValueRow[]>();

  /** The rows, then the blank one typing into creates. */
  protected readonly shown = computed(() => [...this.rows(), BLANK]);
  protected readonly listId = computed(() => `http-${this.kind()}-names`);

  protected isBlank(index: number): boolean {
    return index === this.rows().length;
  }

  protected onInput(index: number, field: Field, event: Event): void {
    const text = (event.target as HTMLInputElement).value;
    const rows = [...this.rows()];
    rows[index] = { ...(rows[index] ?? BLANK), [field]: text };
    this.rowsChange.emit(rows);
  }

  protected onToggle(index: number, event: Event): void {
    const enabled = (event.target as HTMLInputElement).checked;
    const rows = this.rows().map((row, at) => (at === index ? { ...row, enabled } : row));
    this.rowsChange.emit(rows);
  }

  protected remove(index: number): void {
    this.rowsChange.emit(this.rows().filter((_, at) => at !== index));
  }
}
