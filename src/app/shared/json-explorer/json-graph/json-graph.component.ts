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
import {
  JsonGraph,
  JsonNode,
  JsonRow,
  NodeMove,
  NodeOffset,
  findSelection,
  lineage,
} from '@core/model/json.model';

/** One line of a node, in pixels: Rust places in lines, and a column is the font's `ch`. */
export const ROW_PX = 24;
const SCALES = [0.25, 0.4, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2];
/** Room for the search bar drawn over the ground's top-left corner. */
const START = { x: 16, y: 60 };

/** Pointer travel under which a press on a node is still a click. */
const DRAG_FROM_PX = 4;
/** One press of Alt with an arrow: two columns across, one line down. */
const NUDGE = { x: 2, y: 1 };
const NO_OFFSET: NodeOffset = { x: 0, y: 0 };

/** A header is row -1. Kept by path: an answer re-numbers the nodes. */
interface Cursor {
  readonly nodePath: string;
  readonly row: number;
}

interface Edge {
  readonly d: string;
  readonly lit: boolean;
}

interface NodeDrag {
  readonly path: string;
  readonly x: number;
  readonly y: number;
  readonly from: NodeOffset;
  readonly pointerId: number;
}

/**
 * The document as nodes on a ground. It takes inputs only: the editor's JSON snippet today, an
 * HTTP response later. ⚠️ Pointer events for the pan and for moving a node, never HTML5 drag &
 * drop: a gesture that starts on a node moves it, anywhere else it pans. The arrows walk the
 * rows and nodes; Alt with an arrow moves the node, their keyboard twin.
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
  /** Where nodes were moved to, by path, over the layout Rust computed. */
  readonly offsets = input<ReadonlyMap<string, NodeOffset>>(new Map());

  /** A click on a node's header, or the cursor moved there. */
  readonly selected = output<string>();
  /** A click on a row, or Enter on it: the container opens or closes, a value is selected. */
  readonly rowActivated = output<JsonRow>();
  /** A node let go where it was dragged to, or nudged with the keys. */
  readonly moved = output<NodeMove>();
  /** Back to the layout Rust computed. */
  readonly layoutReset = output<void>();

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

  /** The node under a drag, ahead of the store: committed on `pointerup`. */
  protected readonly dragged = signal<NodeMove | null>(null);

  /** Each node where it is drawn: Rust's place, plus what it was moved by. */
  protected readonly placed = computed<ReadonlyMap<string, NodeOffset>>(() => {
    const offsets = this.offsets();
    const dragged = this.dragged();
    return new Map(
      this.graph().nodes.map((node) => {
        const offset = dragged?.path === node.path ? dragged.offset : (offsets.get(node.path) ?? NO_OFFSET);
        return [node.path, { x: node.x + offset.x, y: node.y + offset.y }];
      }),
    );
  });

  protected readonly moves = computed(() => this.offsets().size > 0);

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
    const placed = this.placed();
    return graph.nodes.flatMap((node) =>
      node.rows.flatMap((row, index) => {
        const child = row.child === null ? undefined : graph.nodes[row.child];
        if (!child) return [];
        const from = placed.get(node.path)!;
        const to = placed.get(child.path)!;
        const x1 = from.x + node.width;
        const y1 = from.y + 1.5 + index;
        const x2 = to.x;
        const y2 = to.y + 0.5;
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
  private nodeDrag: NodeDrag | null = null;
  /** A drag ends in a click on what it started on: that click is the drag's, not a choice. */
  private dragJustEnded = false;

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
    if (this.swallowDragClick()) return;
    this.cursor.set({ nodePath: node.path, row: -1 });
    this.selected.emit(node.path);
  }

  protected chooseRow(node: JsonNode, index: number): void {
    if (this.swallowDragClick()) return;
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

  /** The whole graph in the ground, moved nodes included, never past 100 %. */
  fit(): void {
    const graph = this.graph();
    const placed = this.placed();
    if (graph.nodes.length === 0) return;
    const box = this.ground().nativeElement.getBoundingClientRect();
    const ch = this.charWidth();
    const left = Math.min(...graph.nodes.map((node) => placed.get(node.path)!.x));
    const top = Math.min(...graph.nodes.map((node) => placed.get(node.path)!.y));
    const right = Math.max(...graph.nodes.map((node) => placed.get(node.path)!.x + node.width));
    const bottom = Math.max(...graph.nodes.map((node) => placed.get(node.path)!.y + node.height));
    const width = (right - left) * ch;
    const height = (bottom - top) * ROW_PX;
    if (width === 0 || height === 0) return;

    const room = { x: box.width - 2 * START.x, y: box.height - START.y - START.x };
    const scale = Math.max(SCALES[0]!, Math.min(1, room.x / width, room.y / height));
    this.scale.set(scale);
    this.pan.set({
      x: (box.width - width * scale) / 2 - left * ch * scale,
      y: START.y + (room.y - height * scale) / 2 - top * ROW_PX * scale,
    });
  }

  protected onPointerDown(event: PointerEvent): void {
    this.dragJustEnded = false;
    if (event.button !== 0) return;
    const node = (event.target as Element).closest<HTMLElement>('[data-testid="json-node"]');
    const path = node?.dataset['path'];
    if (path !== undefined) {
      const from = this.offsets().get(path) ?? NO_OFFSET;
      this.nodeDrag = { path, x: event.clientX, y: event.clientY, from, pointerId: event.pointerId };
      return;
    }
    if ((event.target as Element).closest('button')) return;
    this.dragFrom = { x: event.clientX, y: event.clientY, pan: this.pan() };
    this.ground().nativeElement.setPointerCapture?.(event.pointerId);
    this.panning.set(true);
  }

  protected onPointerMove(event: PointerEvent): void {
    const drag = this.nodeDrag;
    if (drag) {
      const dx = event.clientX - drag.x;
      const dy = event.clientY - drag.y;
      if (!this.dragged() && Math.hypot(dx, dy) < DRAG_FROM_PX) return;
      // Captured only once it is a drag: captured at the press, a click lands on the ground.
      if (!this.dragged()) this.ground().nativeElement.setPointerCapture?.(drag.pointerId);
      const scale = this.scale();
      this.dragged.set({
        path: drag.path,
        offset: { x: drag.from.x + dx / (this.charWidth() * scale), y: drag.from.y + dy / (ROW_PX * scale) },
      });
      return;
    }
    const from = this.dragFrom;
    if (!from) return;
    this.pan.set({ x: from.pan.x + event.clientX - from.x, y: from.pan.y + event.clientY - from.y });
  }

  protected onPointerUp(): void {
    const moved = this.dragged();
    this.nodeDrag = null;
    if (moved) {
      this.dragged.set(null);
      this.dragJustEnded = true;
      // The click a release makes is dispatched in the same task; one that never comes must
      // not swallow the next choice, made with Enter or by a WebDriver click.
      setTimeout(() => (this.dragJustEnded = false));
      this.moved.emit(moved);
    }
    this.dragFrom = null;
    this.panning.set(false);
  }

  private swallowDragClick(): boolean {
    const swallowed = this.dragJustEnded;
    this.dragJustEnded = false;
    return swallowed;
  }

  protected onWheel(event: WheelEvent): void {
    event.preventDefault();
    const pan = this.pan();
    this.pan.set({ x: pan.x - event.deltaX, y: pan.y - event.deltaY });
  }

  protected onKeydown(event: KeyboardEvent): void {
    // Shift with an arrow opens or folds everything: the explorer's, not a move.
    if (event.shiftKey && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) return;
    const node = this.cursorNode();
    const cursor = this.cursor();
    if (!node || !cursor) {
      if (event.key.startsWith('Arrow')) this.moveTo(this.graph().nodes[0], -1, event);
      return;
    }
    if (event.altKey && event.key.startsWith('Arrow')) {
      this.nudge(node, event);
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

  private nudge(node: JsonNode, event: KeyboardEvent): void {
    const step = {
      ArrowLeft: { x: -NUDGE.x, y: 0 },
      ArrowRight: { x: NUDGE.x, y: 0 },
      ArrowUp: { x: 0, y: -NUDGE.y },
      ArrowDown: { x: 0, y: NUDGE.y },
    }[event.key];
    if (!step) return;
    event.preventDefault();
    event.stopPropagation();
    const from = this.offsets().get(node.path) ?? NO_OFFSET;
    this.moved.emit({ path: node.path, offset: { x: from.x + step.x, y: from.y + step.y } });
    afterNextRender(() => this.reveal(), { injector: this.injector });
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

  focusCursor(): void {
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
