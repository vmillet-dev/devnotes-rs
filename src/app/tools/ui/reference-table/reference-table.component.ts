import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  Directive,
  ElementRef,
  TemplateRef,
  afterRenderEffect,
  computed,
  contentChild,
  inject,
  input,
  model,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ReferenceRow, matchesSearchable } from '@core/services/tools/reference.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { IconComponent } from '@shared/icon/icon.component';
import { CopyValueComponent } from '../copy-value/copy-value.component';

export interface ReferenceGroup {
  readonly id: string;
  readonly label: string;
}

export interface ReferenceColumn {
  readonly id: string;
  readonly labelKey: string;
  /** A share of the card or a length; none takes what is left. */
  readonly width?: string;
}

/**
 * The columns in their order, each sized: every group is a table of its own, and only widths
 * set the same way line their columns up from one card to the next.
 */
export function referenceColumns(
  prefix: string,
  widths: Readonly<Record<string, string | null>>,
): readonly ReferenceColumn[] {
  return Object.entries(widths).map(([id, width]) => ({
    id,
    labelKey: `${prefix}.${id}`,
    ...(width === null ? {} : { width }),
  }));
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

  private readonly store = inject(ToolsStore);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  /** The row the home's search opened this reference on. */
  protected readonly lit = this.store.entry;
  private broughtIntoView: string | null = null;

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

  constructor() {
    // Once, when its row is drawn: the words land after the table does, and a language
    // changed later must not take the page back to it.
    afterRenderEffect(() => {
      const key = this.lit();
      if (key === null || key === this.broughtIntoView || this.sections().length === 0) return;
      const row = [...this.host.nativeElement.querySelectorAll<HTMLElement>('tr.row')].find(
        (candidate) => candidate.dataset['key'] === key,
      );
      if (row === undefined) return;
      this.broughtIntoView = key;
      this.centre(row);
    });
  }

  /**
   * ⚠️ Not `scrollIntoView`: it scrolls every ancestor that can be, and the window's own page
   * can — the title bar went off the top. The frame alone scrolls here.
   */
  private centre(row: HTMLElement): void {
    const frame = row.closest<HTMLElement>('.frame');
    if (frame === null) return;
    const [box, line] = [frame.getBoundingClientRect(), row.getBoundingClientRect()];
    frame.scrollTop += line.top - box.top - (box.height - line.height) / 2;
  }

  protected onSearch(event: Event): void {
    this.store.forgetEntry();
    this.query.set((event.target as HTMLInputElement).value);
  }
}
