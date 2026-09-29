import { ChangeDetectionStrategy, Component, computed, input, output, viewChild } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  JsonEntry,
  JsonKind,
  JsonLine,
  JsonRow,
  JsonView,
  Span,
  findSelection,
  lineage,
  preview,
} from '@core/model/json.model';
import { JsonGraphComponent } from './json-graph/json-graph.component';
import { JsonTreeComponent } from './json-tree/json-tree.component';

export type JsonExplorerMode = 'graph' | 'tree';

interface Selected {
  readonly path: string;
  readonly kind: JsonKind;
  readonly size: number;
  readonly preview: string;
  readonly span: Span;
}

interface Crumb {
  readonly label: string;
  readonly path: string;
}

const CONTAINERS: readonly JsonKind[] = ['object', 'array'];

/**
 * The visualiser: the graph or the tree, the search over them, what is selected and the
 * document. Inputs and outputs only — whoever shows a document owns its explorer's store.
 */
@Component({
  selector: 'app-json-explorer',
  imports: [TranslocoPipe, JsonGraphComponent, JsonTreeComponent],
  templateUrl: './json-explorer.component.html',
  styleUrl: './json-explorer.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonExplorerComponent {
  readonly view = input.required<JsonView>();
  readonly mode = input.required<JsonExplorerMode>();
  /** The text the view was made of, which the spans point into. */
  readonly text = input.required<string>();
  readonly selectedPath = input<string | null>(null);
  readonly search = input('');
  readonly matchIndex = input(-1);
  /** For the document's size, written the way the interface's language writes numbers. */
  readonly locale = input('fr');

  readonly searchChanged = output<string>();
  readonly stepped = output<1 | -1>();
  readonly toggled = output<JsonEntry>();
  readonly selected = output<string>();
  readonly openedAll = output<void>();
  readonly copied = output<string>();
  readonly shownInCode = output<Span>();
  readonly treeProposed = output<void>();

  private readonly graphView = viewChild(JsonGraphComponent);

  protected readonly selection = computed<Selected | null>(() => {
    const view = this.view();
    const path = this.selectedPath() ?? view.graph.nodes[0]?.path ?? null;
    const found = findSelection(view.graph, path);
    if (found?.kind === 'node') {
      const { node } = found;
      return { path: node.path, kind: node.kind, size: node.size, preview: preview(node), span: node.span };
    }
    const entry = found?.kind === 'row' ? found.row : view.tree.find((line) => line.path === path);
    if (!entry) return null;
    return {
      path: entry.path,
      kind: entry.kind,
      size: 'size' in entry ? entry.size : 0,
      preview: entry.value,
      span: entry.span,
    };
  });

  protected readonly crumbs = computed<readonly Crumb[]>(() => {
    const view = this.view();
    const found = findSelection(view.graph, this.selectedPath());
    const node = found?.node ?? view.graph.nodes[0];
    if (!node) return [];
    const chain = lineage(view.graph, node).map((each) => ({ label: each.label, path: each.path }));
    return found?.kind === 'row' ? [...chain, { label: found.row.key, path: found.row.path }] : chain;
  });

  protected readonly size = computed(() => {
    const bytes = this.view().stats.bytes;
    const format = (value: number) =>
      new Intl.NumberFormat(this.locale(), { maximumFractionDigits: 1 }).format(value);
    if (bytes < 1024) return { key: 'json.bytes', value: format(bytes) };
    if (bytes < 1024 * 1024) return { key: 'json.kilobytes', value: format(bytes / 1024) };
    return { key: 'json.megabytes', value: format(bytes / 1024 / 1024) };
  });

  protected readonly matchLabel = computed(() => {
    const count = this.view().matchCount;
    const index = this.matchIndex();
    return index < 0 ? `${count}` : `${index + 1} / ${count}`;
  });

  protected isContainer(kind: JsonKind): boolean {
    return CONTAINERS.includes(kind);
  }

  protected onRow(row: JsonRow): void {
    this.toggled.emit({ path: row.path, opens: row.opens, open: row.child !== null });
  }

  protected onLine(line: JsonLine): void {
    this.toggled.emit(line);
  }

  protected onSearchKeydown(event: KeyboardEvent, field: HTMLInputElement): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      this.stepped.emit(event.shiftKey ? -1 : 1);
    } else if (event.key === 'Escape' && field.value !== '') {
      event.preventDefault();
      event.stopPropagation();
      this.searchChanged.emit('');
    }
  }

  protected copyValue(span: Span): void {
    this.copied.emit(this.text().slice(span.start, span.end));
  }

  protected fit(): void {
    this.graphView()?.fit();
  }
}
