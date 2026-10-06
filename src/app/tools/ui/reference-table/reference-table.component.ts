import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  Directive,
  TemplateRef,
  computed,
  contentChild,
  inject,
  input,
  model,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { toolMatches } from '@core/services/tools/tool.model';
import { IconComponent } from '@shared/icon/icon.component';
import { CopyValueComponent } from '../copy-value/copy-value.component';

export interface ReferenceRow {
  /** Unique in its table, and what the row's button copies: `404`, `Content-Type`. */
  readonly key: string;
  readonly group: string;
  /** What a search reads, case and accents folded: the key, the name, the words on screen. */
  readonly searchable: readonly string[];
  /** What the row's button copies instead of the key, and its name aloud: a character, keyed by its code. */
  readonly copy?: { readonly value: string; readonly what: string };
}

export interface ReferenceGroup {
  readonly id: string;
  readonly label: string;
}

export interface ReferenceColumn {
  readonly id: string;
  readonly labelKey: string;
}

export interface ReferenceCellsContext<T> {
  readonly $implicit: T;
}

/** A row's `<td>`s: `<ng-template [appReferenceCells]="rows()" let-row>`. */
@Directive({ selector: 'ng-template[appReferenceCells]' })
export class ReferenceCellsDirective<T extends ReferenceRow> {
  readonly template = inject<TemplateRef<ReferenceCellsContext<T>>>(TemplateRef);
  /** Read for its type alone, so that `let-row` is the reference's own row. */
  readonly appReferenceCells = input.required<readonly T[]>();

  static ngTemplateContextGuard<T extends ReferenceRow>(
    _directive: ReferenceCellsDirective<T>,
    _context: unknown,
  ): _context is ReferenceCellsContext<T> {
    return true;
  }
}

export function matchesSearchable(row: ReferenceRow, query: string): boolean {
  return toolMatches(query, row.searchable);
}

/**
 * A reference's table: a search that filters as it is typed, the rows under their groups, a
 * count, and each row's key to copy. The field sits outside everything that re-renders, so
 * typing never loses the focus.
 */
@Component({
  selector: 'app-reference-table',
  imports: [CopyValueComponent, IconComponent, NgTemplateOutlet, TranslocoPipe],
  templateUrl: './reference-table.component.html',
  styleUrl: './reference-table.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReferenceTableComponent<T extends ReferenceRow> {
  readonly rows = input.required<readonly T[]>();
  readonly groups = input.required<readonly ReferenceGroup[]>();
  /** The headers of the template's cells, in their order. */
  readonly columns = input.required<readonly ReferenceColumn[]>();
  readonly caption = input.required<string>();
  readonly placeholder = input.required<string>();
  /** For a reference that reads a query its own way: a code in any base. */
  readonly matches = input<(row: T, query: string) => boolean>(matchesSearchable);
  readonly query = model('');

  protected readonly cells = contentChild.required<ReferenceCellsDirective<T>>(ReferenceCellsDirective);

  protected readonly searching = computed(() => this.query().trim() !== '');

  protected readonly shown = computed(() => {
    const query = this.query();
    const matches = this.matches();
    return this.searching() ? this.rows().filter((row) => matches(row, query)) : this.rows();
  });

  protected readonly sections = computed(() => {
    const shown = this.shown();
    return this.groups()
      .map((group) => ({ ...group, rows: shown.filter((row) => row.group === group.id) }))
      .filter((section) => section.rows.length > 0);
  });

  protected onSearch(event: Event): void {
    this.query.set((event.target as HTMLInputElement).value);
  }
}
