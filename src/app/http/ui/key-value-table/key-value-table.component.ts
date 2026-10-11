import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  resource,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { HeaderFault, KeyValueRow } from '@core/model/http.model';

type Field = 'key' | 'value' | 'description';

const BLANK: KeyValueRow = { enabled: true, key: '', value: '', description: '' };

/** The faults a sentence each is written for, in the order they are said. */
const FAULTS: readonly HeaderFault[] = ['name', 'value'];

/**
 * A request's parameters or headers: a row to tick, its key, value and description, and one
 * blank row at the end that becomes a row as soon as something is typed in it.
 *
 * As `headers`, a row that cannot be sent is marked with why, and `Nom: valeur` typed or
 * pasted in a name is split into its two cells. Both are Rust's to say: a parameter or a form
 * field takes any character, and is asked nothing.
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
  /** The rows are headers: their names are HTTP tokens, and a name may be pasted with its value. */
  readonly headers = input(false);

  readonly rowsChange = output<readonly KeyValueRow[]>();

  private readonly repository = inject(HttpRepository);

  private readonly faultsResource = resource({
    params: () => (this.headers() ? this.rows() : undefined),
    loader: ({ params }) => this.repository.checkHeaders(params),
  });
  /** By rank. The last answer stays while the next is read: a mark must not blink at each key. */
  protected readonly faults = linkedSignal<
    readonly (HeaderFault | null)[] | undefined,
    readonly (HeaderFault | null)[]
  >({
    source: () => (this.faultsResource.hasValue() ? this.faultsResource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? [],
  });
  /** The sentences under the table: one a fault, however many rows carry it. */
  protected readonly said = computed(() => FAULTS.filter((fault) => this.faults().includes(fault)));

  /** The rows, then the blank one typing into creates. */
  protected readonly shown = computed(() => [...this.rows(), BLANK]);
  protected readonly listId = computed(() => `http-${this.kind()}-names`);

  protected isBlank(index: number): boolean {
    return index === this.rows().length;
  }

  protected faultId(fault: HeaderFault): string {
    return `http-${this.kind()}-fault-${fault}`;
  }

  protected onInput(index: number, field: Field, event: Event): void {
    const cell = event.target as HTMLInputElement;
    const rows = [...this.rows()];
    rows[index] = { ...(rows[index] ?? BLANK), [field]: cell.value };
    this.rowsChange.emit(rows);
    if (field === 'key' && this.headers() && cell.value.includes(':')) void this.split(index, cell);
  }

  /**
   * Lines pasted in a name — a browser's network tab copies headers so — fill a row each. The
   * field alone would take them on one line, the names lost in the first one's value.
   */
  protected async onPaste(index: number, event: ClipboardEvent): Promise<void> {
    const text = event.clipboardData?.getData('text/plain') ?? '';
    if (!this.headers() || !/[\r\n]/.test(text.trim())) return;
    event.preventDefault();
    const read = await this.repository.readHeaders(text);
    if (read.length === 0) return;
    const rows = [...this.rows()];
    const current = rows[index];
    // A row already written keeps its place: the lines pasted go after it.
    const taken = current === undefined || (current.key === '' && current.value === '') ? 1 : 0;
    rows.splice(taken === 1 ? index : index + 1, taken, ...read);
    this.rowsChange.emit(rows);
  }

  /**
   * `Nom: valeur` in a name is what was meant as a name and its value: what follows the first
   * colon goes to the value, when there is none yet, and the caret with it.
   */
  private async split(index: number, cell: HTMLInputElement): Promise<void> {
    const text = cell.value;
    const [read] = await this.repository.readHeaders(text);
    const row = this.rows()[index];
    // Typed on meanwhile, or a value already there: nothing is moved.
    if (read === undefined || row?.key !== text || row.value !== '') return;
    const rows = [...this.rows()];
    rows[index] = { ...row, key: read.key, value: read.value };
    this.rowsChange.emit(rows);
    cell.closest('tr')?.querySelector<HTMLInputElement>('[data-testid="http-kv-value"]')?.focus();
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
