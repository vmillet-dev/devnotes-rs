import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Canvas, FabricObject, Path, Rect, Textbox, type TPointerEventInfo } from 'fabric';
import { DrawingScene, Palette, paletteOf, sceneOf } from './drawing-scene';

const PALETTE: Palette = {
  stroke: {
    ink: '#eeeeee',
    blue: '#6f9bd1',
    green: '#6fbf8b',
    amber: '#e8a33d',
    red: '#dc8281',
    purple: '#a98fd1',
  },
  fill: {
    ink: 'rgba(170, 176, 192, 0.06)',
    blue: 'rgba(111, 155, 209, 0.14)',
    green: 'rgba(111, 191, 139, 0.14)',
    amber: 'rgba(232, 163, 61, 0.14)',
    red: 'rgba(220, 130, 129, 0.14)',
    purple: 'rgba(169, 143, 209, 0.14)',
  },
  paper: '#1b1e25',
  accent: '#e8a33d',
};

type Own = FabricObject & {
  id?: string;
  hue?: string;
  from?: string | null;
  to?: string | null;
  ends?: number[];
};

describe('DrawingScene', () => {
  let element: HTMLCanvasElement;
  let scene: DrawingScene;
  let changes: number;
  let made: number;

  const canvas = (): Canvas => scene['canvas'];
  const objects = (): Own[] => canvas().getObjects() as Own[];
  const arrows = (): Own[] => objects().filter((object) => object.ends);

  /** Fabric's own events, as its upper canvas would fire them from the pointer. */
  function fire(
    type: 'mouse:down' | 'mouse:move' | 'mouse:up',
    x: number,
    y: number,
    target?: FabricObject,
  ): void {
    const e = new MouseEvent(type.replace(':', ''), { clientX: x, clientY: y });
    canvas().fire(type, { e, target } as unknown as TPointerEventInfo);
  }

  function drag(
    from: [number, number],
    to: [number, number],
    targets: { down?: FabricObject; up?: FabricObject } = {},
  ): void {
    fire('mouse:down', ...from, targets.down);
    fire('mouse:move', ...to);
    fire('mouse:up', ...to, targets.up);
  }

  function draw(
    tool: 'rectangle' | 'ellipse' | 'diamond',
    from: [number, number],
    to: [number, number],
  ): Own {
    scene.setTool(tool);
    drag(from, to);
    return objects().at(-1)!;
  }

  beforeEach(async () => {
    element = document.createElement('canvas');
    document.body.appendChild(element);
    changes = 0;
    made = 0;
    scene = new DrawingScene(element, PALETTE, {
      changed: () => (changes += 1),
      made: () => (made += 1),
    });
    scene.resize(800, 600);
    await scene.load(null);
  });

  afterEach(() => {
    scene.dispose();
    element.remove();
  });

  it('starts empty, with nothing to undo nor any picture', () => {
    expect(scene.shapes()).toBe(0);
    expect(scene.canUndo()).toBe(false);
    expect(scene.png()).toBeNull();
    expect(scene.svg()).toBeNull();
    expect(scene.pictures()).toEqual({ thumbnail: '', picture: '' });
    expect(sceneOf(scene.body())).toMatchObject({ objects: [] });
  });

  it('draws a rectangle over the drag, by its centre, then goes back to the selection', () => {
    const rect = draw('rectangle', [10, 20], [110, 70]);

    expect(rect).toBeInstanceOf(Rect);
    expect([rect.left, rect.top, rect.width, rect.height]).toEqual([60, 45, 100, 50]);
    expect(rect.stroke).toBe(PALETTE.stroke.ink);
    expect(rect.fill).toBe(PALETTE.fill.ink);
    expect(made).toBe(1);
    expect(changes).toBe(1);
    expect(canvas().getActiveObject()).toBe(rect);
  });

  it('gives a shape a click makes the size of one, and draws ellipses and diamonds alike', () => {
    const clicked = draw('rectangle', [100, 100], [101, 101]);
    const ellipse = draw('ellipse', [0, 0], [60, 40]);
    const diamond = draw('diamond', [200, 200], [240, 260]);

    expect([clicked.width, clicked.height]).toEqual([140, 80]);
    expect([ellipse.get('rx'), ellipse.get('ry'), ellipse.left, ellipse.top]).toEqual([30, 20, 30, 20]);
    expect([diamond.left, diamond.top]).toEqual([220, 230]);
    expect(scene.shapes()).toBe(3);
  });

  it('binds an arrow drawn from a shape to another, and keeps it on their edges as they move', () => {
    const left = draw('rectangle', [0, 0], [100, 50]);
    const right = draw('rectangle', [300, 0], [400, 50]);

    scene.setTool('arrow');
    drag([50, 25], [350, 25], { down: left, up: right });
    const [arrow] = arrows();

    expect(arrow).toBeInstanceOf(Path);
    expect([arrow.from, arrow.to]).toEqual([left.id, right.id]);
    // On the outer edge of each stroke, a pixel past the shape it bounds.
    expect(arrow.ends).toEqual([101, 25, 299, 25]);

    right.set({ left: 350, top: 125 });
    right.setCoords();
    canvas().fire('object:moving', { target: right } as never);
    expect(arrows()[0].ends?.slice(2)).not.toEqual([299, 25]);

    canvas().fire('object:modified', { target: right } as never);
    expect(arrows()[0].from).toBe(left.id);
  });

  it('leaves an arrow drawn in the open where it was drawn, bound to nothing', () => {
    scene.setTool('arrow');
    drag([10, 10], [90, 40]);

    expect(arrows()[0].ends).toEqual([10, 10, 90, 40]);
    expect([arrows()[0].from, arrows()[0].to]).toEqual([null, null]);
  });

  it('keeps the arrows of a shape removed, which then hold on to nothing', () => {
    const left = draw('rectangle', [0, 0], [100, 50]);
    const right = draw('rectangle', [300, 0], [400, 50]);
    scene.setTool('arrow');
    drag([50, 25], [350, 25], { down: left, up: right });

    canvas().setActiveObject(left);
    expect(scene.removeSelection()).toBe(true);

    expect(scene.shapes()).toBe(2);
    expect(arrows()[0].from).toBeNull();
    expect(arrows()[0].to).toBe(right.id);
    canvas().discardActiveObject();
    expect(scene.removeSelection()).toBe(false);
  });

  it('colours what is drawn next and what is selected now', () => {
    const rect = draw('rectangle', [0, 0], [50, 50]);
    const before = changes;

    scene.setHue('blue');

    expect([rect.hue, rect.stroke, rect.fill]).toEqual(['blue', PALETTE.stroke.blue, PALETTE.fill.blue]);
    expect(changes).toBe(before + 1);
    expect(draw('ellipse', [100, 100], [150, 150]).stroke).toBe(PALETTE.stroke.blue);
  });

  it('draws everything again in a theme changed meanwhile', () => {
    const rect = draw('rectangle', [0, 0], [50, 50]);

    scene.recolour({ ...PALETTE, stroke: { ...PALETTE.stroke, ink: '#111111' } });

    expect(rect.stroke).toBe('#111111');
  });

  it('undoes and redoes, the selection tool kept', async () => {
    draw('rectangle', [0, 0], [50, 50]);
    draw('ellipse', [100, 100], [150, 150]);

    await scene.undo();
    expect(scene.shapes()).toBe(1);
    expect(scene.canRedo()).toBe(true);

    await scene.redo();
    expect(scene.shapes()).toBe(2);
    expect(scene.canRedo()).toBe(false);
    expect(objects().every((object) => object.selectable)).toBe(true);
  });

  it('keeps a text that says something, and drops one left empty', () => {
    scene.setTool('text');
    fire('mouse:down', 40, 40);
    const text = objects().at(-1) as Textbox & Own;

    expect(text).toBeInstanceOf(Textbox);
    expect(text.isEditing).toBe(true);
    expect(scene.editingText()).toBe(true);
    text.exitEditing();
    expect(scene.shapes()).toBe(0);

    scene.setTool('text');
    fire('mouse:down', 40, 40);
    const kept = objects().at(-1) as Textbox;
    kept.set({ text: 'API gateway' });
    kept.exitEditing();
    expect(scene.shapes()).toBe(1);
    expect(sceneOf(scene.body())).toMatchObject({ objects: [{ text: 'API gateway' }] });
  });

  it('takes a freehand line in the colour it was drawn in', () => {
    scene.setTool('pen');
    expect(canvas().isDrawingMode).toBe(true);
    const line = new Path('M 0 0 L 40 40');
    canvas().add(line);

    canvas().fire('path:created', { path: line } as never);

    expect((line as Own).hue).toBe('ink');
    expect(line.fill).toBeNull();
    expect(changes).toBe(1);
  });

  it('lets no shape be grabbed while another tool draws', () => {
    const rect = draw('rectangle', [0, 0], [50, 50]);

    scene.setTool('ellipse');
    expect([rect.selectable, rect.evented, canvas().selection]).toEqual([false, false, false]);
    scene.setTool('arrow');
    expect(rect.evented).toBe(true);
  });

  it('takes its pictures and its SVG framed to what is drawn, on its paper', () => {
    draw('rectangle', [10, 10], [110, 60]);

    expect(scene.png()?.startsWith('data:image/png;base64,')).toBe(true);
    const pictures = scene.pictures();
    expect(pictures.thumbnail.startsWith('data:image/png;base64,')).toBe(true);
    expect(pictures.picture.length).toBeGreaterThan(pictures.thumbnail.length / 2);
    const svg = scene.svg()!;
    expect(svg).toContain(`fill="${PALETTE.paper}"`);
    expect(svg).toMatch(/viewBox="-?\d/);
  });

  it('opens a stored scene as it was saved, ids and colours by name', async () => {
    draw('rectangle', [0, 0], [50, 50]);
    scene.setHue('red');
    const body = scene.body();

    const other = new DrawingScene(document.createElement('canvas'), PALETTE, {
      changed: () => undefined,
      made: () => undefined,
    });
    await other.load(sceneOf(body));

    const [rect] = other['canvas'].getObjects() as Own[];
    expect([rect.hue, rect.stroke]).toEqual(['red', PALETTE.stroke.red]);
    expect(other.canUndo()).toBe(false);
    other.dispose();
  });
});

describe('sceneOf', () => {
  it('reads the scene of a body, and nothing of what is none', () => {
    expect(sceneOf('{"version":1,"scene":{"objects":[]}}')).toEqual({ objects: [] });
    expect(sceneOf('')).toBeNull();
    expect(sceneOf('{"scene":3}')).toBeNull();
    expect(sceneOf('null')).toBeNull();
  });
});

describe('paletteOf', () => {
  it('reads each hue from the tokens on the root, a tint of it for a fill', () => {
    const tokens: Record<string, string> = {
      '--text-0': '#eef0f4',
      '--blue': ' #6f9bd1 ',
      '--blue-rgb': '111, 155, 209',
      '--bg-1': '#1b1e25',
      '--amber-edge': '#e8a33d',
    };
    const style = { getPropertyValue: (name: string) => tokens[name] ?? '' } as CSSStyleDeclaration;

    const palette = paletteOf(style);

    expect(palette.stroke.blue).toBe('#6f9bd1');
    expect(palette.fill.blue).toBe('rgba(111, 155, 209, 0.14)');
    expect([palette.paper, palette.accent]).toEqual(['#1b1e25', '#e8a33d']);
  });
});
