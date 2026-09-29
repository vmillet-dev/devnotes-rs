import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  linkedSignal,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { JsonGraph, JsonNode, JsonRow, findSelection, lineage } from '@core/model/json.model';

/** One line of a node, in pixels: Rust places in lines, and a column is the font's `ch`. */
export const ROW_PX = 24;
const SCALES = [0.25, 0.4, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2];
/** Room for the search bar drawn over the ground's top-left corner. */
const START = { x: 16, y: 60 };

/** A header is row -1. Kept by path: an answer re-numbers the nodes. */
interface Cursor {
  readonly nodePath: string;
  readonly row: number;
}

interface Edge {
  readonly d: string;
  readonly lit: boolean;
}

/**
 * The document as nodes on a ground. It takes inputs only: the editor's JSON snippet today, an
 * HTTP response later. ⚠️ Pointer events for the pan, never HTML5 drag & drop, and the arrows
 * as its keyboard twin: they move between rows and nodes, and the ground follows.
 */
@Component({
  selector: 'app-json-graph',
  imports: [TranslocoPipe],
  templateUrl: './json-graph.component.html',
  styleUrl: './json-graph.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonGraphComponent {
  private readonly injector = inject(Injector);

  readonly graph = input.required<JsonGraph>();
  readonly selectedPath = input<string | null>(null);

  /** A click on a node's header, or the cursor moved there. */
  readonly selected = output<string>();
  /** A click on a row, or Enter on it: the container opens or closes, a value is selected. */
  readonly rowActivated = output<JsonRow>();

  protected readonly rowPx = ROW_PX;
  protected readonly scale = signal(1);
  protected readonly pan = signal(START);
  protected readonly panning = signal(false);

  private readonly ground = viewChild.required<ElementRef<HTMLElement>>('ground');
  private readonly probe = viewChild.required<ElementRef<HTMLElement>>('probe');

  protected readonly cursor = linkedSignal<{ graph: JsonGraph; path: string | null }, Cursor | null>({
    source: () => ({ graph: this.graph(), path: this.selectedPath() }),
    computation: ({ graph, path }, previous) => {
      const kept = previous?.value;
      if (kept && this.pathOf(graph, kept) === path) return kept;
      const found = findSelection(graph, path);
      if (!found) return kept && this.nodeOf(graph, kept.nodePath) ? kept : null;
      return { nodePath: found.node.path, row: found.kind === 'row' ? found.index : -1 };
    },
  });

  private readonly cursorNode = computed(() => {
    const cursor = this.cursor();
    return cursor ? this.nodeOf(this.graph(), cursor.nodePath) : undefined;
  });

  /** The nodes from the root down to the one the cursor is in. */
  protected readonly lit = computed<ReadonlySet<number>>(() => {
    const node = this.cursorNode();
    return new Set(node ? lineage(this.graph(), node).map((each) => each.id) : []);
  });

  protected readonly edges = computed<readonly Edge[]>(() => {
    const graph = this.graph();
    const lit = this.lit();
    return graph.nodes.flatMap((node) =>
      node.rows.flatMap((row, index) => {
        const child = row.child === null ? undefined : graph.nodes[row.child];
        if (!child) return [];
        const x1 = node.x + node.width;
        const y1 = node.y + 1.5 + index;
        const x2 = child.x;
        const y2 = child.y + 0.5;
        const bend = (x2 - x1) / 2;
        return [
          {
            d: `M${x1} ${y1} C${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}`,
            lit: lit.has(node.id) && lit.has(child.id),
          },
        ];
      }),
    );
  });

  protected readonly zoomLabel = computed(() => `${Math.round(this.scale() * 100)} %`);

  private dragFrom: { x: number; y: number; pan: { x: number; y: number } } | null = null;

  constructor() {
    // A cursor moved from outside — a match stepped to — is brought into view.
    effect(() => {
      const cursor = this.cursor();
      if (cursor) untracked(() => afterNextRender(() => this.reveal(), { injector: this.injector }));
    });
  }

  protected isCursor(node: JsonNode, row: number): boolean {
    const cursor = this.cursor();
    return cursor !== null && cursor.nodePath === node.path && cursor.row === row;
  }

  /** The one tab stop: the cursor, or the root before there is one. */
  protected tabStop(node: JsonNode, row: number): number {
    return this.isCursor(node, row) || (this.cursor() === null && node.id === 0 && row === -1) ? 0 : -1;
  }

  protected chooseHeader(node: JsonNode): void {
    this.cursor.set({ nodePath: node.path, row: -1 });
    this.selected.emit(node.path);
  }

  protected chooseRow(node: JsonNode, index: number): void {
    this.cursor.set({ nodePath: node.path, row: index });
    this.rowActivated.emit(node.rows[index]!);
  }

  zoomBy(step: 1 | -1): void {
    const index = SCALES.findIndex((scale) => scale >= this.scale());
    const next =
      SCALES[Math.min(SCALES.length - 1, Math.max(0, (index < 0 ? SCALES.length - 1 : index) + step))]!;
    this.zoomTo(next);
  }

  zoomTo(scale: number): void {
    // Around the middle of the ground, which is what stays put under the eye.
    const box = this.ground().nativeElement.getBoundingClientRect();
    const middle = { x: box.width / 2, y: box.height / 2 };
    const ratio = scale / this.scale();
    const pan = this.pan();
    this.pan.set({ x: middle.x - (middle.x - pan.x) * ratio, y: middle.y - (middle.y - pan.y) * ratio });
    this.scale.set(scale);
  }

  /** The whole graph in the ground, never past 100 %. */
  fit(): void {
    const graph = this.graph();
    const box = this.ground().nativeElement.getBoundingClientRect();
    const width = graph.width * this.charWidth();
    const height = graph.height * ROW_PX;
    if (width === 0 || height === 0) return;

    const room = { x: box.width - 2 * START.x, y: box.height - START.y - START.x };
    const scale = Math.max(SCALES[0]!, Math.min(1, room.x / width, room.y / height));
    this.scale.set(scale);
    this.pan.set({ x: (box.width - width * scale) / 2, y: START.y + (room.y - height * scale) / 2 });
  }

  protected onPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || (event.target as Element).closest('button')) return;
    this.dragFrom = { x: event.clientX, y: event.clientY, pan: this.pan() };
    this.ground().nativeElement.setPointerCapture?.(event.pointerId);
    this.panning.set(true);
  }

  protected onPointerMove(event: PointerEvent): void {
    const from = this.dragFrom;
    if (!from) return;
    this.pan.set({ x: from.pan.x + event.clientX - from.x, y: from.pan.y + event.clientY - from.y });
  }

  protected onPointerUp(): void {
    this.dragFrom = null;
    this.panning.set(false);
  }

  protected onWheel(event: WheelEvent): void {
    event.preventDefault();
    const pan = this.pan();
    this.pan.set({ x: pan.x - event.deltaX, y: pan.y - event.deltaY });
  }

  protected onKeydown(event: KeyboardEvent): void {
    const node = this.cursorNode();
    const cursor = this.cursor();
    if (!node || !cursor) {
      if (event.key.startsWith('Arrow')) this.moveTo(this.graph().nodes[0], -1, event);
      return;
    }
    const row = cursor.row >= 0 ? node.rows[cursor.row] : undefined;
    const child = row && row.child !== null ? this.graph().nodes[row.child] : undefined;

    switch (event.key) {
      case 'ArrowDown':
        this.moveTo(node, Math.min(node.rows.length - 1, cursor.row + 1), event);
        break;
      case 'ArrowUp':
        this.moveTo(node, Math.max(-1, cursor.row - 1), event);
        break;
      case 'ArrowRight':
        if (child) this.moveTo(child, -1, event);
        else if (row?.opens) this.activate(node, cursor.row, event);
        else if (!row && node.rows.length > 0) this.moveTo(node, 0, event);
        break;
      case 'ArrowLeft':
      case 'Escape':
        this.goUp(node, cursor.row, event);
        break;
      case 'Enter':
      case ' ':
        if (row) this.activate(node, cursor.row, event);
        break;
    }
  }

  /** From a row to its header, from a header to the row that opened it; the root lets go. */
  private goUp(node: JsonNode, row: number, event: KeyboardEvent): void {
    if (row >= 0) {
      this.moveTo(node, -1, event);
      return;
    }
    const parent = node.parent === null ? undefined : this.graph().nodes[node.parent];
    if (!parent) return;
    this.moveTo(
      parent,
      parent.rows.findIndex((each) => each.child === node.id),
      event,
    );
  }

  private moveTo(node: JsonNode | undefined, row: number, event: KeyboardEvent): void {
    if (!node) return;
    event.preventDefault();
    event.stopPropagation();
    this.cursor.set({ nodePath: node.path, row });
    this.selected.emit(row >= 0 ? node.rows[row]!.path : node.path);
    afterNextRender(() => this.focusCursor(), { injector: this.injector });
  }

  private activate(node: JsonNode, row: number, event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.rowActivated.emit(node.rows[row]!);
    afterNextRender(() => this.focusCursor(), { injector: this.injector });
  }

  private focusCursor(): void {
    this.ground()
      .nativeElement.querySelector<HTMLElement>('[data-cursor="true"]')
      ?.focus({ preventScroll: true });
    this.reveal();
  }

  /** Pans just enough to bring the cursor inside the ground. */
  private reveal(): void {
    const ground = this.ground().nativeElement;
    const target = ground.querySelector<HTMLElement>('[data-cursor="true"]');
    if (!target) return;
    const box = ground.getBoundingClientRect();
    const rect = target.getBoundingClientRect();
    const margin = 24;
    let dx = 0;
    let dy = 0;
    if (rect.right > box.right - margin) dx = box.right - margin - rect.right;
    if (rect.left + dx < box.left + margin) dx = box.left + margin - rect.left;
    if (rect.bottom > box.bottom - margin) dy = box.bottom - margin - rect.bottom;
    if (rect.top + dy < box.top + START.y) dy = box.top + START.y - rect.top;
    if (dx !== 0 || dy !== 0) {
      const pan = this.pan();
      this.pan.set({ x: pan.x + dx, y: pan.y + dy });
    }
  }

  private charWidth(): number {
    return this.probe().nativeElement.getBoundingClientRect().width / this.scale() || 7;
  }

  private nodeOf(graph: JsonGraph, path: string): JsonNode | undefined {
    return graph.nodes.find((node) => node.path === path);
  }

  private pathOf(graph: JsonGraph, cursor: Cursor): string | null {
    const node = this.nodeOf(graph, cursor.nodePath);
    if (!node) return null;
    return cursor.row >= 0 ? (node.rows[cursor.row]?.path ?? null) : node.path;
  }
}
