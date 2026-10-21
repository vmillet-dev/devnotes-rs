import type { DrawingPictures } from '@core/model/note.model';
import {
  Canvas,
  Ellipse,
  FabricObject,
  Path,
  PencilBrush,
  Polygon,
  Rect,
  Textbox,
  util,
  type TPointerEvent,
  type TPointerEventInfo,
} from 'fabric';
import {
  Box,
  Ends,
  Hue,
  HUES,
  ShapeTool,
  Tool,
  XY,
  arrowPath,
  boundEnds,
  centreOf,
  diamondPoints,
  dragBox,
  pictureScale,
  sceneBox,
} from './drawing-geometry';

/** What the scene keeps on its objects, past Fabric's own properties. */
const OWN = ['id', 'hue', 'from', 'to', 'ends'];

/** A shape let go smaller than this was a click: it takes the size a click gives. */
const CLICKED = 6;
const CLICK_SIZE = { width: 140, height: 80 };
/** A card's picture, and a Note's: the longest side, and the most a pixel is enlarged. */
const THUMBNAIL = { side: 480, most: 1 };
const PICTURE = { side: 1600, most: 2 };
const HISTORY = 60;
const TEXT_WIDTH = 180;

/** What a scene is saved with, besides Fabric's: the colours follow the theme, by name. */
interface Own {
  id?: string;
  hue?: Hue;
  from?: string | null;
  to?: string | null;
  ends?: Ends;
}
type Drawn = FabricObject & Own;

/** The colours each hue takes in the theme on screen. */
export interface Palette {
  readonly stroke: Readonly<Record<Hue, string>>;
  readonly fill: Readonly<Record<Hue, string>>;
  readonly paper: string;
  readonly accent: string;
}

const STROKE_TOKENS: Readonly<Record<Hue, string>> = {
  ink: '--text-0',
  blue: '--blue',
  green: '--green',
  amber: '--amber-fill',
  red: '--red',
  purple: '--purple',
};

const RGB_TOKENS: Readonly<Record<Hue, string>> = {
  ink: '--text-1-rgb',
  blue: '--blue-rgb',
  green: '--green-rgb',
  amber: '--amber-rgb',
  red: '--red-rgb',
  purple: '--purple-rgb',
};

/** Read from the tokens on `:root`, as Mermaid's colours are: a canvas knows no CSS variable. */
export function paletteOf(style: CSSStyleDeclaration): Palette {
  const token = (name: string): string => style.getPropertyValue(name).trim();
  const each = (read: (hue: Hue) => string): Record<Hue, string> =>
    Object.fromEntries(HUES.map((hue) => [hue, read(hue)])) as Record<Hue, string>;
  return {
    stroke: each((hue) => token(STROKE_TOKENS[hue])),
    fill: each((hue) => `rgba(${token(RGB_TOKENS[hue])}, ${hue === 'ink' ? 0.06 : 0.14})`),
    paper: token('--bg-1'),
    accent: token('--amber-edge'),
  };
}

/**
 * The body a Dessin is stored as: its scene alone, which Rust reads for its words. Its pictures
 * are stored beside it, where a list never opens them.
 */
export interface DrawingBody {
  readonly version: 1;
  readonly scene: Record<string, unknown>;
}

/** The scene a stored body holds; `null` for a new drawing, or a body that is none. */
export function sceneOf(content: string): Record<string, unknown> | null {
  try {
    const body: unknown = JSON.parse(content);
    const scene = (body as Partial<DrawingBody> | null)?.scene;
    return scene && typeof scene === 'object' ? scene : null;
  } catch {
    return null;
  }
}

export interface SceneHooks {
  /** After every change the history kept, undone and redone ones included. */
  changed(): void;
  /** A shape or a text made: the tool goes back to the selection. */
  made(): void;
}

interface Making {
  readonly tool: ShapeTool | 'arrow';
  readonly from: XY;
  readonly object: Drawn;
  readonly fromId: string | null;
}

/**
 * The drawing itself, on Fabric: the tools, the arrows bound to the shapes they start and end
 * on, the history, and the pictures. What is drawn is kept by hue, so a theme changed later
 * draws it in its colours.
 */
export class DrawingScene {
  private readonly canvas: Canvas;
  private tool: Tool = 'select';
  private hue: Hue = 'ink';
  private making: Making | null = null;
  private history: string[] = [];
  private at = -1;
  private restoring = false;
  private count = 0;

  constructor(
    element: HTMLCanvasElement,
    private palette: Palette,
    private readonly hooks: SceneHooks,
  ) {
    this.canvas = new Canvas(element, {
      preserveObjectStacking: true,
      selectionColor: 'rgba(0, 0, 0, 0.04)',
      selectionBorderColor: palette.accent,
      selectionLineWidth: 1,
    });
    this.canvas.on('mouse:down', (event) => this.down(event));
    this.canvas.on('mouse:move', (event) => this.move(event));
    this.canvas.on('mouse:up', (event) => this.up(event));
    this.canvas.on('object:moving', ({ target }) => this.follow(target as Drawn));
    this.canvas.on('object:scaling', ({ target }) => this.follow(target as Drawn));
    this.canvas.on('object:modified', () => this.settle());
    this.canvas.on('path:created', ({ path }) => this.adopt(path as Drawn));
    this.canvas.on('text:editing:exited', ({ target }) => this.leaveText(target as Drawn));
  }

  async load(scene: Record<string, unknown> | null): Promise<void> {
    this.restoring = true;
    // A new drawing opened over another starts from nothing, not from what that one held;
    // so does a scene Fabric cannot read, rather than an editor left restoring for good.
    this.canvas.remove(...this.canvas.getObjects());
    if (scene) await this.canvas.loadFromJSON(scene).catch(() => undefined);
    this.count = this.canvas.getObjects().length;
    this.dress();
    this.restoring = false;
    this.history = [this.snapshot()];
    this.at = 0;
  }

  resize(width: number, height: number): void {
    this.canvas.setDimensions({ width, height });
  }

  recolour(palette: Palette): void {
    this.palette = palette;
    this.canvas.selectionBorderColor = palette.accent;
    this.dress();
  }

  setTool(tool: Tool): void {
    this.tool = tool;
    this.canvas.isDrawingMode = tool === 'pen';
    if (tool === 'pen') {
      const brush = new PencilBrush(this.canvas);
      brush.width = 2.5;
      brush.color = this.palette.stroke[this.hue];
      this.canvas.freeDrawingBrush = brush;
    }
    this.canvas.selection = tool === 'select';
    for (const object of this.objects()) {
      object.selectable = tool === 'select';
      // An arrow needs to find the shape under the pointer, to start or end on it.
      object.evented = tool === 'select' || tool === 'arrow';
    }
    if (tool !== 'select') this.canvas.discardActiveObject();
    this.canvas.requestRenderAll();
  }

  /** For what is drawn next, and for what is selected now. */
  setHue(hue: Hue): void {
    this.hue = hue;
    if (this.canvas.freeDrawingBrush) this.canvas.freeDrawingBrush.color = this.palette.stroke[hue];
    const selected = this.canvas.getActiveObjects() as Drawn[];
    for (const object of selected) {
      object.hue = hue;
      this.paint(object);
    }
    if (selected.length > 0) {
      this.canvas.requestRenderAll();
      this.keep();
    }
  }

  /** What the arrows bound to it hold on to is let go: they stay where they were. */
  removeSelection(): boolean {
    const selected = this.canvas.getActiveObjects() as Drawn[];
    if (selected.length === 0 || this.editingText()) return false;

    const gone = new Set(selected.map((object) => object.id));
    this.canvas.discardActiveObject();
    this.canvas.remove(...selected);
    for (const arrow of this.arrows()) {
      if (gone.has(arrow.from ?? undefined)) arrow.from = null;
      if (gone.has(arrow.to ?? undefined)) arrow.to = null;
    }
    this.canvas.requestRenderAll();
    this.keep();
    return true;
  }

  /** `false` when nothing was selected, which leaves Escape to whatever holds the drawing. */
  deselect(): boolean {
    if (!this.canvas.getActiveObject()) return false;
    this.canvas.discardActiveObject();
    this.canvas.requestRenderAll();
    return true;
  }

  /** While a text is typed into, its keys are the text's. */
  editingText(): boolean {
    const active = this.canvas.getActiveObject();
    return active instanceof Textbox && active.isEditing;
  }

  canUndo(): boolean {
    return this.at > 0;
  }

  canRedo(): boolean {
    return this.at < this.history.length - 1;
  }

  async undo(): Promise<void> {
    const before = this.history[this.at - 1];
    if (before === undefined) return;
    this.at -= 1;
    await this.restore(before);
  }

  async redo(): Promise<void> {
    const after = this.history[this.at + 1];
    if (after === undefined) return;
    this.at += 1;
    await this.restore(after);
  }

  shapes(): number {
    return this.count;
  }

  body(): string {
    const body: DrawingBody = { version: 1, scene: this.canvas.toObject(OWN) as Record<string, unknown> };
    return JSON.stringify(body);
  }

  /** A card's and a Note's pictures; both empty for a drawing with nothing on it any more. */
  pictures(): DrawingPictures {
    return {
      thumbnail: this.png(THUMBNAIL.side, THUMBNAIL.most) ?? '',
      picture: this.png(PICTURE.side, PICTURE.most) ?? '',
    };
  }

  /** The whole drawing at its picture's scale, on its paper; `null` when nothing is drawn. */
  png(side = PICTURE.side, most = PICTURE.most): string | null {
    const box = this.box();
    if (!box) return null;
    return this.onPaper(() =>
      this.canvas.toDataURL({ format: 'png', multiplier: pictureScale(box, side, most), ...box }),
    );
  }

  /** The drawing framed to what is drawn, its paper laid under it. */
  svg(): string | null {
    const box = this.box();
    if (!box) return null;
    const svg = this.canvas.toSVG({
      viewBox: { x: box.left, y: box.top, width: box.width, height: box.height },
      width: String(Math.round(box.width)),
      height: String(Math.round(box.height)),
    });
    const paper = `<rect x="${box.left}" y="${box.top}" width="${box.width}" height="${box.height}" fill="${this.palette.paper}"/>`;
    return svg.replace(/<svg\b[^>]*>/, (open) => `${open}\n${paper}`);
  }

  dispose(): void {
    void this.canvas.dispose();
  }

  private down(event: TPointerEventInfo<TPointerEvent>): void {
    if (this.tool === 'select' || this.tool === 'pen') return;
    const from = this.canvas.getScenePoint(event.e);

    if (this.tool === 'text') {
      this.makeText(from);
      return;
    }
    const fromId = this.tool === 'arrow' ? this.shapeAt(event.target) : null;
    const object =
      this.tool === 'arrow'
        ? this.makeArrow([from.x, from.y, from.x, from.y], fromId, null)
        : this.makeShape(this.tool, dragBox(from, from));
    object.evented = false;
    this.canvas.add(object);
    this.making = { tool: this.tool, from, object, fromId };
  }

  private move(event: TPointerEventInfo<TPointerEvent>): void {
    const making = this.making;
    if (!making) return;
    const to = this.canvas.getScenePoint(event.e);

    if (making.tool === 'arrow') {
      this.replaceArrow(making.object, [making.from.x, making.from.y, to.x, to.y]);
    } else {
      this.reshape(making.object, making.tool, dragBox(making.from, to));
    }
    this.canvas.requestRenderAll();
  }

  private up(event: TPointerEventInfo<TPointerEvent>): void {
    const making = this.making;
    if (!making) return;
    this.making = null;
    const to = this.canvas.getScenePoint(event.e);

    if (making.tool === 'arrow') {
      const toId = this.shapeAt(event.target);
      const arrow = this.objects().find((object) => object.id === making.object.id) ?? making.object;
      arrow.from = making.fromId;
      arrow.to = toId === making.fromId ? null : toId;
      this.bind(arrow, [making.from.x, making.from.y, to.x, to.y]);
    } else {
      const box = dragBox(making.from, to);
      const clicked = box.width < CLICKED && box.height < CLICKED;
      this.reshape(
        making.object,
        making.tool,
        clicked ? { left: making.from.x, top: making.from.y, ...CLICK_SIZE } : box,
      );
    }
    this.finish();
  }

  /** Back to the selection, the new object selected. */
  private finish(): void {
    this.setTool('select');
    this.hooks.made();
    const last = this.objects().at(-1);
    if (last) this.canvas.setActiveObject(last);
    this.canvas.requestRenderAll();
    this.keep();
  }

  private makeShape(tool: ShapeTool, box: Box): Drawn {
    let object: FabricObject;
    switch (tool) {
      case 'rectangle':
        object = new Rect({ rx: 6, ry: 6 });
        break;
      case 'ellipse':
        object = new Ellipse();
        break;
      case 'diamond':
        object = new Polygon(diamondPoints(box.width, box.height));
        break;
    }
    const shape = this.own(object, { strokeWidth: 2, strokeUniform: true, objectCaching: false });
    this.reshape(shape, tool, box);
    return shape;
  }

  /** ⚠️ Fabric 7 places an object by its centre: `left` and `top` are the box's middle. */
  private reshape(object: Drawn, tool: ShapeTool, box: Box): void {
    const { x, y } = centreOf(box);
    if (tool === 'rectangle') {
      object.set({ width: box.width, height: box.height });
    } else if (tool === 'ellipse') {
      object.set({ rx: box.width / 2, ry: box.height / 2 });
    } else {
      const polygon = object as Polygon & Own;
      polygon.set({ points: diamondPoints(box.width, box.height) });
      polygon.setDimensions();
    }
    object.set({ left: x, top: y });
    object.setCoords();
  }

  private makeText(at: XY): void {
    // Its left edge where the click was: Fabric places it by its centre.
    const text = this.own(
      new Textbox('', { left: at.x + TEXT_WIDTH / 2, top: at.y, width: TEXT_WIDTH, fontSize: 18 }),
      {
        fontFamily: 'Inter Variable, sans-serif',
      },
    );
    this.canvas.add(text);
    this.setTool('select');
    this.hooks.made();
    this.canvas.setActiveObject(text);
    (text as Textbox).enterEditing();
  }

  /** A text left empty was a click: it goes, and the history never knew it. */
  private leaveText(text: Drawn): void {
    if (text instanceof Textbox && text.text.trim() === '') {
      this.canvas.remove(text);
      this.canvas.requestRenderAll();
      this.count = this.canvas.getObjects().length;
      return;
    }
    this.keep();
  }

  private makeArrow(ends: Ends, from: string | null, to: string | null, id?: string): Drawn {
    const arrow = this.own(new Path(arrowPath(ends), { strokeWidth: 2, objectCaching: false }), {}, id);
    arrow.ends = ends;
    arrow.from = from;
    arrow.to = to;
    return arrow;
  }

  /**
   * ⚠️ A path's box is read from its data when it is made: an arrow whose ends move is made
   * again, in the same place in the stack, rather than given new data.
   */
  private replaceArrow(arrow: Drawn, ends: Ends): Drawn {
    const next = this.makeArrow(ends, arrow.from ?? null, arrow.to ?? null, arrow.id);
    next.hue = arrow.hue ?? this.hue;
    next.evented = arrow.evented;
    next.selectable = arrow.selectable;
    this.paint(next);
    const at = this.canvas.getObjects().indexOf(arrow);
    this.canvas.remove(arrow);
    this.canvas.insertAt(Math.max(at, 0), next);
    if (this.making?.object === arrow) this.making = { ...this.making, object: next };
    return next;
  }

  /** Its ends on the edges of the shapes it is bound to, or where they were drawn. */
  private bind(arrow: Drawn, drawn: Ends): void {
    this.replaceArrow(arrow, boundEnds(drawn, this.boxById(arrow.from), this.boxById(arrow.to)));
  }

  /** The arrows of a shape that moves follow it as it moves. */
  private follow(target: Drawn | undefined): void {
    if (!target?.id || target.ends) return;
    for (const arrow of this.arrows()) {
      if (arrow.from === target.id || arrow.to === target.id) this.bind(arrow, arrow.ends ?? [0, 0, 0, 0]);
    }
  }

  /** Several shapes moved together are read at the end, where they landed. */
  private settle(): void {
    for (const arrow of this.arrows()) {
      if (arrow.from || arrow.to) this.bind(arrow, arrow.ends ?? [0, 0, 0, 0]);
    }
    this.canvas.requestRenderAll();
    this.keep();
  }

  /** A freehand line drawn with the pen. */
  private adopt(path: Drawn): void {
    this.own(path, { fill: null, strokeLineCap: 'round', strokeLineJoin: 'round' });
    this.keep();
  }

  private shapeAt(target: FabricObject | undefined): string | null {
    const drawn = target as Drawn | undefined;
    return drawn?.id && !drawn.ends && !(drawn instanceof Textbox) ? drawn.id : null;
  }

  private own<T extends FabricObject>(object: T, options: Record<string, unknown>, id?: string): T & Own {
    const drawn = object as T & Own;
    drawn.set(options);
    drawn.id = id ?? crypto.randomUUID();
    drawn.hue ??= this.hue;
    this.dressOne(drawn);
    return drawn;
  }

  /** Every object in its colours, with the selection's handles in the app's. */
  private dress(): void {
    for (const object of this.objects()) this.dressOne(object);
    this.canvas.requestRenderAll();
  }

  private dressOne(object: Drawn): void {
    object.set({
      borderColor: this.palette.accent,
      cornerColor: this.palette.accent,
      cornerStrokeColor: this.palette.paper,
      transparentCorners: false,
      cornerSize: 9,
    });
    this.paint(object);
  }

  private paint(object: Drawn): void {
    const hue = object.hue ?? 'ink';
    const stroke = this.palette.stroke[hue];
    if (object instanceof Textbox) {
      object.set({ fill: stroke });
    } else if (object.ends) {
      object.set({ stroke, fill: stroke });
    } else if (object instanceof Path) {
      object.set({ stroke });
    } else {
      object.set({ stroke, fill: this.palette.fill[hue] });
    }
  }

  /** One state of the history, after a change: the scene as it is saved. */
  private keep(): void {
    this.count = this.canvas.getObjects().length;
    if (this.restoring) return;
    const snapshot = this.snapshot();
    if (snapshot === this.history[this.at]) return;

    this.history = [...this.history.slice(0, this.at + 1), snapshot].slice(-HISTORY);
    this.at = this.history.length - 1;
    this.hooks.changed();
  }

  private async restore(snapshot: string): Promise<void> {
    this.restoring = true;
    this.canvas.discardActiveObject();
    await this.canvas.loadFromJSON(snapshot);
    this.count = this.canvas.getObjects().length;
    this.dress();
    this.setTool(this.tool);
    this.restoring = false;
    this.hooks.changed();
  }

  private snapshot(): string {
    return JSON.stringify(this.canvas.toObject(OWN));
  }

  private objects(): Drawn[] {
    return this.canvas.getObjects();
  }

  private arrows(): Drawn[] {
    return this.objects().filter((object) => object.ends);
  }

  private boxById(id: string | null | undefined): Box | null {
    const object = id ? this.objects().find((each) => each.id === id) : undefined;
    return object ? boxOf(object) : null;
  }

  private box(): Box | null {
    return sceneBox(this.objects().map(boxOf));
  }

  private onPaper<T>(take: () => T): T {
    const before = this.canvas.backgroundColor;
    const selected = this.canvas.getActiveObject();
    this.canvas.discardActiveObject();
    this.canvas.backgroundColor = this.palette.paper;
    try {
      return take();
    } finally {
      this.canvas.backgroundColor = before;
      if (selected) this.canvas.setActiveObject(selected);
      this.canvas.requestRenderAll();
    }
  }
}

/** In the scene's plane, groups and selections taken into account. */
function boxOf(object: FabricObject): Box {
  const { left, top, width, height } = util.makeBoundingBoxFromPoints(object.getCoords());
  return { left, top, width, height };
}
