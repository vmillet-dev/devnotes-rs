import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  resource,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpEnvironmentsRepository } from '@core/data/http-environments.repository';
import {
  FilledBy,
  VARIABLE_KINDS,
  VariableFault,
  VariableKind,
  VariableRow,
} from '@core/model/http-environment.model';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { IconComponent } from '@shared/icon/icon.component';

type Field = 'name' | 'value';

const BLANK: VariableRow = { enabled: true, name: '', value: '', kind: 'text' };

/** The faults a sentence each is written for, in the order they are said. */
const FAULTS: readonly VariableFault[] = ['name', 'duplicate', 'number'];

/**
 * A set of variables — the globals, an environment, a collection, a folder, a request: a row
 * to tick, its name, its value and its type, and a blank row at the end that becomes one as
 * soon as something is typed in it. A secret's value is masked, and shown one row at a time.
 */
@Component({
  selector: 'app-variables-table',
  imports: [ChoiceMenuComponent, IconComponent, TranslocoPipe],
  templateUrl: './variables-table.component.html',
  styleUrl: './variables-table.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VariablesTableComponent {
  readonly rows = input.required<readonly VariableRow[]>();
  readonly label = input.required<string>();
  /** Stable, unlike the translated label: what the tests address. */
  readonly kind = input.required<string>();

  readonly rowsChange = output<readonly VariableRow[]>();
  /** « remplie par Login »: the request that writes it, to open on its scripts. */
  readonly fillerOpened = output<FilledBy>();

  private readonly repository = inject(HttpEnvironmentsRepository);

  /** ⚠️ What was last emitted, until the host answers: `rows` follows a change detection later. */
  private readonly latest = linkedSignal(() => this.rows());

  private readonly faultsResource = resource({
    params: () => this.rows(),
    loader: ({ params }) => this.repository.check(params),
  });
  /** By rank. The last answer stays while the next is read: a mark must not blink at each key. */
  protected readonly faults = linkedSignal<
    readonly (VariableFault | null)[] | undefined,
    readonly (VariableFault | null)[]
  >({
    source: () => (this.faultsResource.hasValue() ? this.faultsResource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? [],
  });
  protected readonly said = computed(() => FAULTS.filter((fault) => this.faults().includes(fault)));

  protected readonly shown = computed(() => [...this.rows(), BLANK]);
  protected readonly kinds: readonly ChoiceOption[] = VARIABLE_KINDS.map((kind) => ({
    id: kind,
    name: `http.variables.kinds.${kind}`,
    nameIsKey: true,
  }));

  /** The secrets shown in clear, by rank: one at a time, as asked, until the table is left. */
  protected readonly revealed = signal<ReadonlySet<number>>(new Set());

  protected isBlank(index: number): boolean {
    return index === this.rows().length;
  }

  protected faultId(fault: VariableFault): string {
    return `http-${this.kind()}-variable-fault-${fault}`;
  }

  protected masked(row: VariableRow, index: number): boolean {
    return row.kind === 'secret' && !this.revealed().has(index);
  }

  protected toggleReveal(index: number): void {
    const next = new Set(this.revealed());
    if (!next.delete(index)) next.add(index);
    this.revealed.set(next);
  }

  protected onInput(index: number, field: Field, event: Event): void {
    const rows = [...this.latest()];
    rows[index] = { ...(rows[index] ?? BLANK), [field]: (event.target as HTMLInputElement).value };
    this.emit(rows);
  }

  protected onKind(index: number, kind: string | null): void {
    if (kind === null) return;
    const rows = [...this.latest()];
    rows[index] = { ...(rows[index] ?? BLANK), kind: kind as VariableKind };
    // A row turned secret is masked at once: what was shown in clear was a text.
    if (kind === 'secret') this.revealed.update((shown) => new Set([...shown].filter((at) => at !== index)));
    this.emit(rows);
  }

  protected onToggle(index: number, event: Event): void {
    const enabled = (event.target as HTMLInputElement).checked;
    this.emit(this.latest().map((row, at) => (at === index ? { ...row, enabled } : row)));
  }

  protected remove(index: number): void {
    this.revealed.set(new Set());
    this.emit(this.latest().filter((_, at) => at !== index));
  }

  private emit(rows: readonly VariableRow[]): void {
    this.latest.set(rows);
    this.rowsChange.emit(rows);
  }
}
