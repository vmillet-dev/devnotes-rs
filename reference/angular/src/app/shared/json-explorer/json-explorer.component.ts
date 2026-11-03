import {
  ChangeDetectionStrategy,
  Component,
  DOCUMENT,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  JsonEntry,
  JsonKind,
  JsonLine,
  JsonRow,
  JsonView,
  NodeMove,
  NodeOffset,
  Span,
  StructureFormat,
  ValueMenuAsk,
  FORMAT_NAMES,
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
  /** A string's text, unquoted: `null` for anything else, copied as it is written. */
  readonly text: string | null;
}

interface Crumb {
  readonly label: string;
  readonly path: string;
}

const CONTAINERS: readonly JsonKind[] = ['object', 'array'];

/** The menu's own size, to keep it inside the window. */
const MENU_WIDTH_PX = 300;
const MENU_HEIGHT_PX = 120;

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
  /** `false` where no code is beside the explorer to show a span in: an HTTP response. */
  readonly withCode = input(true);
  /** What the text is written in: the messages name it, and a YAML document says what it drops. */
  readonly format = input<StructureFormat>('json');
  /** Where the graph's nodes were moved to, by path. */
  readonly offsets = input<ReadonlyMap<string, NodeOffset>>(new Map());
  /** An HTTP answer: a value can be kept in a variable, from the menu or the selection. */
  readonly extractable = input(false);
  /** How a path is written: `$` in a note, `body` in an answer. */
  readonly root = input('$');

  readonly searchChanged = output<string>();
  readonly stepped = output<1 | -1>();
  readonly toggled = output<JsonEntry>();
  readonly selected = output<string>();
  readonly openedAll = output<void>();
  readonly collapsedAll = output<void>();
  readonly copied = output<string>();
  readonly shownInCode = output<Span>();
  readonly treeProposed = output<void>();
  readonly nodeMoved = output<NodeMove>();
  readonly layoutReset = output<void>();
  /** The path as the explorer writes it, from `$`. */
  readonly extractAsked = output<string>();

  private readonly graphView = viewChild(JsonGraphComponent);
  private readonly treeView = viewChild(JsonTreeComponent);
  private readonly injector = inject(Injector);
  private readonly document = inject(DOCUMENT);
  /** Set by the keys: the entry that had the focus is redrawn by the answer, and the focus goes with it. */
  private refocusOnAnswer = false;
  private readonly menuPanel = viewChild<ElementRef<HTMLElement>>('menuPanel');
  /** The right-click's menu, where it was asked. */
  protected readonly menu = signal<ValueMenuAsk | null>(null);

  protected readonly selection = computed<Selected | null>(() => {
    const view = this.view();
    const path = this.selectedPath() ?? view.graph.nodes[0]?.path ?? null;
    const found = findSelection(view.graph, path);
    if (found?.kind === 'node') {
      const { node } = found;
      return {
        path: node.path,
        kind: node.kind,
        size: node.size,
        preview: preview(node),
        span: node.span,
        text: null,
      };
    }
    const entry = found?.kind === 'row' ? found.row : view.tree.find((line) => line.path === path);
    if (!entry) return null;
    return {
      path: entry.path,
      kind: entry.kind,
      size: entry.size,
      preview: entry.value,
      span: entry.span,
      text: entry.text,
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

  protected readonly formatName = computed(() => FORMAT_NAMES[this.format()]);

  protected readonly matchLabel = computed(() => {
    const count = this.view().matchCount;
    const index = this.matchIndex();
    return index < 0 ? `${count}` : `${index + 1} / ${count}`;
  });

  constructor() {
    effect(() => {
      this.view();
      if (!this.refocusOnAnswer) return;
      this.refocusOnAnswer = false;
      untracked(() =>
        afterNextRender(() => (this.graphView() ?? this.treeView())?.focusCursor(), {
          injector: this.injector,
        }),
      );
    });
  }

  /** Shift with an arrow, on the graph or the tree: everything opens, or folds to the root. */
  protected onSurfaceKeydown(event: KeyboardEvent): void {
    if (!event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return;
    if (event.key === 'ArrowRight') this.openedAll.emit();
    else if (event.key === 'ArrowLeft') this.collapsedAll.emit();
    else return;
    event.preventDefault();
    this.refocusOnAnswer = true;
  }

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

  /** A path as the explorer writes it, from its root. */
  protected written(path: string): string {
    return path.startsWith('$') ? this.root() + path.slice(1) : path;
  }

  protected openMenu(asked: ValueMenuAsk): void {
    const view = this.document.defaultView;
    // Kept inside the window: a value near its right or bottom edge opens the menu inward.
    const x = Math.max(8, Math.min(asked.x, (view?.innerWidth ?? asked.x) - MENU_WIDTH_PX - 8));
    const y = Math.max(8, Math.min(asked.y, (view?.innerHeight ?? asked.y) - MENU_HEIGHT_PX - 8));
    this.menu.set({ ...asked, x, y });
    afterNextRender(
      {
        write: () => {
          const panel = this.menuPanel()?.nativeElement;
          if (panel && typeof panel.showPopover === 'function') panel.showPopover();
          panel?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
        },
      },
      { injector: this.injector },
    );
  }

  /** The focus gone elsewhere — a click outside, Tab — puts the menu away. */
  protected onMenuFocusOut(event: FocusEvent, panel: HTMLElement): void {
    if (!(event.relatedTarget instanceof Node && panel.contains(event.relatedTarget))) this.closeMenu();
  }

  protected closeMenu(): void {
    this.menu.set(null);
  }

  protected menuCopyValue(): void {
    const selected = this.selection();
    if (selected) this.copyValue(selected);
    this.closeMenu();
  }

  protected menuCopyPath(path: string): void {
    this.copied.emit(this.written(path));
    this.closeMenu();
  }

  protected menuExtract(path: string): void {
    this.closeMenu();
    this.extractAsked.emit(path);
  }

  protected copyValue(selected: Selected): void {
    this.copied.emit(selected.text ?? this.text().slice(selected.span.start, selected.span.end));
  }

  protected fit(): void {
    this.graphView()?.fit();
  }
}
