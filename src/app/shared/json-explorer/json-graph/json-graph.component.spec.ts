import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { JsonRow, NodeMove } from '@core/model/json.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { jsonView } from '@testing/json-view.fixture';
import { JsonGraphComponent } from './json-graph.component';

describe('JsonGraphComponent', () => {
  let fixture: ComponentFixture<JsonGraphComponent>;
  let selected: string[];
  let activated: JsonRow[];
  let moved: NodeMove[];
  let resets: number;

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [JsonGraphComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(JsonGraphComponent);
    fixture.componentRef.setInput('graph', jsonView().graph);
    selected = [];
    activated = [];
    moved = [];
    resets = 0;
    fixture.componentInstance.selected.subscribe((path) => selected.push(path));
    fixture.componentInstance.rowActivated.subscribe((row) => activated.push(row));
    fixture.componentInstance.moved.subscribe((move) => moved.push(move));
    fixture.componentInstance.layoutReset.subscribe(() => resets++);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  function node(path: string): HTMLElement {
    return fixture.nativeElement.querySelector(`[data-testid="json-node"][data-path="${path}"]`);
  }

  function row(path: string): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="json-row"][data-path="${path}"]`);
  }

  function ground(): HTMLElement {
    return fixture.nativeElement.querySelector('[data-testid="json-graph"]');
  }

  function press(key: string): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
    const focused = document.activeElement;
    (focused && ground().contains(focused) ? focused : ground()).dispatchEvent(event);
    return event;
  }

  async function select(path: string): Promise<void> {
    fixture.componentRef.setInput('selectedPath', path);
    await fixture.whenStable();
  }

  function plane(): HTMLElement {
    return fixture.nativeElement.querySelector('.plane');
  }

  it('places each node where Rust put it, a column a character and a row a line', () => {
    const data = node('$.data');

    expect(data.style.left).toBe('22ch');
    expect(data.style.top).toBe('72px');
    expect(data.style.width).toBe('16ch');
  });

  it('draws an edge from each open row to its child', () => {
    expect(fixture.nativeElement.querySelectorAll('.edges path')).toHaveLength(3);
  });

  it('colours a row by its kind and says whether it is open', () => {
    expect(row('$.id').dataset['kind']).toBe('string');
    expect(row('$.data').getAttribute('aria-expanded')).toBe('true');
    expect(row('$.id').getAttribute('aria-expanded')).toBeNull();
  });

  it('selects a node by its header, and hands a row over to be opened', () => {
    node('$.data').querySelector<HTMLElement>('[data-testid="json-node-head"]')!.click();
    row('$.data.lines[0].a').click();

    expect(selected).toEqual(['$.data']);
    expect(activated.map((each) => each.path)).toEqual(['$.data.lines[0].a']);
  });

  it('lights the nodes from the root down to the selection', async () => {
    await select('$.data.lines[0].a');

    expect(node('$').classList).toContain('lit');
    expect(node('$.data.lines[0]').classList).toContain('lit');
    expect(fixture.nativeElement.querySelectorAll('.edges path.lit')).toHaveLength(3);
    expect(row('$.data.lines[0].a').getAttribute('tabindex')).toBe('0');
  });

  it('keeps a single tab stop, on the root before anything is chosen', () => {
    const stops = fixture.nativeElement.querySelectorAll('[tabindex="0"]');

    expect(stops).toHaveLength(1);
    expect(stops[0].closest('[data-testid="json-node"]').dataset.path).toBe('$');
  });

  describe('keyboard', () => {
    it('enters the root on a first arrow', () => {
      ground().dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));

      expect(selected).toEqual(['$']);
    });

    it('walks the rows, into a child and back up', async () => {
      await select('$');

      press('ArrowDown');
      press('ArrowDown');
      expect(selected).toEqual(['$.id', '$.data']);

      press('ArrowRight');
      expect(selected.at(-1)).toBe('$.data');
      expect(fixture.componentInstance['cursor']()).toEqual({ nodePath: '$.data', row: -1 });

      press('ArrowLeft');
      expect(fixture.componentInstance['cursor']()).toEqual({ nodePath: '$', row: 1 });

      press('ArrowUp');
      expect(selected.at(-1)).toBe('$.id');
    });

    it('goes up on Escape, and lets it go at the root', async () => {
      await select('$.data');

      expect(press('Escape').defaultPrevented).toBe(true);
      expect(fixture.componentInstance['cursor']()).toEqual({ nodePath: '$', row: 1 });
      press('Escape');
      expect(press('Escape').defaultPrevented).toBe(false);
    });

    it('opens or closes a row on Enter, and opens a closed one on the right arrow', async () => {
      const closed = jsonView().graph;
      const root = {
        ...closed.nodes[0]!,
        rows: [closed.nodes[0]!.rows[0]!, { ...closed.nodes[0]!.rows[1]!, child: null }],
      };
      fixture.componentRef.setInput('graph', { ...closed, nodes: [root] });
      await select('$.data');

      press('Enter');
      press('ArrowRight');

      expect(activated.map((each) => each.path)).toEqual(['$.data', '$.data']);
    });

    it('moves into the rows from a header on the right arrow', async () => {
      await select('$.data.lines[0]');

      press('ArrowRight');

      expect(selected.at(-1)).toBe('$.data.lines[0].a');
    });
  });

  describe('the ground', () => {
    it('zooms by steps around its middle, and back to 100 %', () => {
      const zoom = fixture.nativeElement.querySelector('[data-testid="json-zoom-reset"]');

      fixture.nativeElement.querySelector('[data-testid="json-zoom-in"]').click();
      fixture.detectChanges();
      expect(zoom.textContent.trim()).toBe('125 %');

      fixture.nativeElement.querySelector('[data-testid="json-zoom-out"]').click();
      fixture.nativeElement.querySelector('[data-testid="json-zoom-out"]').click();
      fixture.detectChanges();
      expect(zoom.textContent.trim()).toBe('80 %');

      zoom.click();
      fixture.detectChanges();
      expect(plane().style.transform).toContain('scale(1)');
    });

    it('pans with the pointer, and not from a node', () => {
      ground().dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 100, bubbles: true }),
      );
      ground().dispatchEvent(new PointerEvent('pointermove', { clientX: 140, clientY: 90, bubbles: true }));
      ground().dispatchEvent(new PointerEvent('pointerup', { bubbles: true }));
      fixture.detectChanges();
      expect(plane().style.transform).toContain('translate(56px, 50px)');

      row('$.id').dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 0, clientY: 0, bubbles: true }),
      );
      ground().dispatchEvent(new PointerEvent('pointermove', { clientX: 500, clientY: 500, bubbles: true }));
      fixture.detectChanges();
      expect(plane().style.transform).toContain('translate(56px, 50px)');
    });

    it('never scrolls to a focused node: the pan is the only way it moves', () => {
      ground().scrollTop = 40;
      ground().dispatchEvent(new Event('scroll'));

      expect(ground().scrollTop).toBe(0);
    });

    it('scrolls with the wheel', () => {
      ground().dispatchEvent(
        new WheelEvent('wheel', { deltaX: 10, deltaY: 20, bubbles: true, cancelable: true }),
      );
      fixture.detectChanges();

      expect(plane().style.transform).toContain('translate(6px, 40px)');
    });

    it('fits nothing into a ground that has no size', () => {
      fixture.componentInstance.fit();

      expect(plane().style.transform).toContain('scale(1)');
    });
  });
  describe('moving a node', () => {
    const pointer = (type: string, x: number, y: number) =>
      new PointerEvent(type, { button: 0, clientX: x, clientY: y, bubbles: true });
    const head = (path: string) =>
      node(path).querySelector<HTMLButtonElement>('[data-testid="json-node-head"]')!;

    it('drags a node from its header, lets it go where it was dropped, and keeps the click from choosing', async () => {
      head('$.data').dispatchEvent(pointer('pointerdown', 100, 100));
      ground().dispatchEvent(pointer('pointermove', 114, 148));
      fixture.detectChanges();

      // Followed while it travels, the store told only on release: 14 px is two 7 px columns.
      expect(node('$.data').style.left).toBe('24ch');
      expect(node('$.data').style.top).toBe('120px');
      expect(node('$.data').classList).toContain('dragged');
      expect(moved).toEqual([]);

      ground().dispatchEvent(pointer('pointerup', 114, 148));
      head('$.data').click();
      await fixture.whenStable();

      expect(moved).toEqual([{ path: '$.data', offset: { x: 2, y: 2 } }]);
      expect(selected).toEqual([]);
      await new Promise((resolve) => setTimeout(resolve));
      head('$.data').click();
      expect(selected).toEqual(['$.data']);
    });

    it('takes a press that barely moves for a click, and pans from anywhere but a node', () => {
      row('$.id').dispatchEvent(pointer('pointerdown', 10, 10));
      ground().dispatchEvent(pointer('pointermove', 12, 11));
      ground().dispatchEvent(pointer('pointerup', 12, 11));
      row('$.id').click();

      expect(moved).toEqual([]);
      expect(activated.map((each) => each.path)).toEqual(['$.id']);
    });

    it('draws the node where the store says, its edges following', async () => {
      const edges = () =>
        Array.from(fixture.nativeElement.querySelectorAll('.edges path'), (path: Element) =>
          path.getAttribute('d'),
        );
      const before = edges();
      fixture.componentRef.setInput('offsets', new Map([['$.data', { x: 10, y: -2 }]]));
      await fixture.whenStable();

      expect(node('$.data').style.left).toBe('32ch');
      expect(node('$.data').style.top).toBe('24px');
      // Into it from the root, and out of it to its child: two edges of three moved.
      expect(edges().filter((d, index) => d !== before[index])).toHaveLength(2);
    });

    it('nudges the node the cursor is in with Alt and an arrow, from where it already was', async () => {
      fixture.componentRef.setInput('offsets', new Map([['$.data', { x: 2, y: 0 }]]));
      // The cursor on the row $.data.lines: the node it sits in moves, not the node it opens.
      row('$.data.lines').click();
      await fixture.whenStable();
      row('$.data.lines').focus();

      const event = new KeyboardEvent('keydown', {
        key: 'ArrowDown',
        altKey: true,
        bubbles: true,
        cancelable: true,
      });
      row('$.data.lines').dispatchEvent(event);
      row('$.data.lines').dispatchEvent(
        new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true }),
      );
      row('$.data.lines').dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Home', altKey: true, bubbles: true }),
      );

      expect(event.defaultPrevented).toBe(true);
      expect(moved).toEqual([
        { path: '$.data', offset: { x: 2, y: 1 } },
        { path: '$.data', offset: { x: 0, y: 0 } },
      ]);
      expect(selected).toEqual([]);
      expect(activated.map((each) => each.path)).toEqual(['$.data.lines']);
    });

    it('offers to put everything back only once something moved', async () => {
      const reset = () => fixture.nativeElement.querySelector('[data-testid="json-reset-layout"]');
      expect(reset()).toBeNull();

      fixture.componentRef.setInput('offsets', new Map([['$', { x: 1, y: 1 }]]));
      await fixture.whenStable();
      reset().click();

      expect(resets).toBe(1);
    });

    it('fits the moved nodes too', async () => {
      fixture.componentRef.setInput('offsets', new Map([['$.data.lines[0]', { x: 100, y: 40 }]]));
      await fixture.whenStable();
      const box = {
        width: 800,
        height: 400,
        top: 0,
        left: 0,
        right: 800,
        bottom: 400,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      };
      ground().getBoundingClientRect = () => box as DOMRect;
      fixture.componentInstance.fit();
      fixture.detectChanges();

      expect(plane().style.transform).toMatch(/scale\(0\.\d+\)/);
    });
  });
});
