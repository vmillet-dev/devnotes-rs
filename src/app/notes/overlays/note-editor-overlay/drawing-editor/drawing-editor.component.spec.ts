import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Rect, type Canvas, type TPointerEventInfo } from 'fabric';
import type { DrawingPictures } from '@core/model/note.model';
import { DrawingExportService, DrawingFormat } from '@core/services/drawing/drawing-export.service';
import { DrawingPicturesStore } from '@core/state/drawing-pictures.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { DrawingEditorComponent, SAVE_AFTER_MS } from './drawing-editor.component';
import type { DrawingScene } from './drawing-scene';

describe('DrawingEditorComponent', () => {
  let fixture: ComponentFixture<DrawingEditorComponent>;
  let bodies: string[];
  let pictures: DrawingPictures[];
  let blurs: number;
  let counts: number[];
  let exported: { title: string; format: DrawingFormat; picture: string }[];
  /** The drawings asked whether they have pictures; `imported` has none. */
  let asked: string[];

  const el = <T extends HTMLElement = HTMLElement>(selector: string): T | null =>
    (fixture.nativeElement as HTMLElement).querySelector<T>(selector);
  const tool = (id: string) => el<HTMLButtonElement>(`[data-testid="drawing-tool"][data-tool="${id}"]`)!;
  const scene = (): DrawingScene => fixture.componentInstance['scene']!;
  const canvas = (): Canvas => scene()['canvas'];

  async function mount(content = '', noteId: string | null = null): Promise<void> {
    fixture = TestBed.createComponent(DrawingEditorComponent);
    fixture.componentRef.setInput('content', content);
    fixture.componentRef.setInput('noteId', noteId);
    fixture.componentRef.setInput('title', 'Archi');
    fixture.componentInstance.changed.subscribe((body) => bodies.push(body));
    fixture.componentInstance.pictured.subscribe((taken) => pictures.push(taken));
    fixture.componentInstance.blurred.subscribe(() => (blurs += 1));
    fixture.componentInstance.counted.subscribe((count) => counts.push(count));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(fixture.componentInstance['scene']).not.toBeNull());
    await fixture.whenStable();
  }

  /** A rectangle drawn with the pointer, as Fabric's upper canvas reports it. */
  async function drawRectangle(): Promise<void> {
    tool('rectangle').click();
    for (const [type, x, y] of [
      ['mouse:down', 10, 10],
      ['mouse:move', 90, 60],
      ['mouse:up', 90, 60],
    ] as const) {
      const e = new MouseEvent(type.replace(':', ''), { clientX: x, clientY: y });
      canvas().fire(type, { e } as unknown as TPointerEventInfo);
    }
    await fixture.whenStable();
  }

  async function press(key: string, init: KeyboardEventInit = {}): Promise<KeyboardEvent> {
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
    el('[data-testid="drawing-stage"]')!.dispatchEvent(event);
    await fixture.whenStable();
    return event;
  }

  beforeEach(() => {
    bodies = [];
    pictures = [];
    blurs = 0;
    counts = [];
    exported = [];
    asked = [];
    TestBed.configureTestingModule({
      imports: [DrawingEditorComponent],
      providers: [
        provideTranslocoTesting(),
        {
          provide: DrawingPicturesStore,
          useValue: {
            hasPictures: (id: string) => {
              asked.push(id);
              return Promise.resolve(id !== 'imported');
            },
          },
        },
        {
          provide: DrawingExportService,
          useValue: {
            save: (title: string, format: DrawingFormat, picture: string) => {
              exported.push({ title, format, picture });
              return Promise.resolve();
            },
          },
        },
      ],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    fixture.destroy();
    fixture.nativeElement.remove();
  });

  it('opens empty, says how to start, and offers no export', async () => {
    await mount();

    expect(el('.drawing-hint')?.textContent).toContain('Choisissez un outil');
    expect(el<HTMLButtonElement>('[data-testid="drawing-export"]')!.disabled).toBe(true);
    expect(el<HTMLButtonElement>('[data-testid="drawing-undo"]')!.disabled).toBe(true);
    expect(tool('select').getAttribute('aria-pressed')).toBe('true');
  });

  it('saves once the hand pauses, and takes the pictures only when flushed', async () => {
    await mount();
    await drawRectangle();

    expect(el('.drawing-hint')).toBeNull();
    expect(tool('select').getAttribute('aria-pressed')).toBe('true');
    expect(counts.at(-1)).toBe(1);
    expect(bodies).toEqual([]);
    await vi.waitFor(() => expect(bodies).toHaveLength(1), { timeout: SAVE_AFTER_MS * 4 });
    expect(JSON.parse(bodies[0]!)).toMatchObject({ version: 1, scene: { objects: [{ type: 'Rect' }] } });
    expect(pictures).toEqual([]);

    fixture.componentInstance.flush();

    expect(pictures).toHaveLength(1);
    expect(pictures[0]!.thumbnail.startsWith('data:image/png;base64,')).toBe(true);
    fixture.componentInstance.flush();
    expect(pictures).toHaveLength(1);
  });

  it('gives what is pending at once when flushed, the body and its pictures', async () => {
    await mount();
    await drawRectangle();

    fixture.componentInstance.flush();

    expect(bodies).toHaveLength(1);
    expect(pictures).toHaveLength(1);
  });

  it('flushes and says so when the focus leaves for the rest of the editor, not within it', async () => {
    await mount();
    await drawRectangle();
    const stage = el('[data-testid="drawing-stage"]')!;

    stage.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: tool('ellipse') }));
    expect(blurs).toBe(0);

    stage.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: document.body }));
    expect(blurs).toBe(1);
    expect(bodies).toHaveLength(1);
    expect(pictures).toHaveLength(1);
  });

  it('picks a tool by its letter, and Escape drops it, then the selection, then nothing', async () => {
    await mount();

    await press('r');
    expect(tool('rectangle').getAttribute('aria-pressed')).toBe('true');
    const dropped = await press('Escape');
    expect(dropped.defaultPrevented).toBe(true);
    expect(tool('select').getAttribute('aria-pressed')).toBe('true');

    await drawRectangle();
    const deselected = await press('Escape');
    expect(deselected.defaultPrevented).toBe(true);
    expect(canvas().getActiveObject()).toBeUndefined();

    const left = await press('Escape');
    expect(left.defaultPrevented).toBe(false);
    expect((await press('q')).defaultPrevented).toBe(false);
    expect((await press('r', { altKey: true })).defaultPrevented).toBe(false);
  });

  it('deletes the selection from the key and the button, and undoes and redoes', async () => {
    await mount();
    await drawRectangle();

    await press('Delete');
    expect(scene().shapes()).toBe(0);
    expect((await press('Delete')).defaultPrevented).toBe(false);

    await press('z', { ctrlKey: true });
    await vi.waitFor(() => expect(scene().shapes()).toBe(1));
    await press('z', { ctrlKey: true, shiftKey: true });
    await vi.waitFor(() => expect(scene().shapes()).toBe(0));
    await press('z', { ctrlKey: true });
    await vi.waitFor(() => expect(scene().shapes()).toBe(1));
    await press('y', { ctrlKey: true });
    await vi.waitFor(() => expect(scene().shapes()).toBe(0));

    el<HTMLButtonElement>('[data-testid="drawing-undo"]')!.click();
    await vi.waitFor(() => expect(scene().shapes()).toBe(1));
    el<HTMLButtonElement>('[data-testid="drawing-redo"]')!.click();
    await vi.waitFor(() => expect(scene().shapes()).toBe(0));
    el<HTMLButtonElement>('[data-testid="drawing-undo"]')!.click();
    await vi.waitFor(() => expect(scene().shapes()).toBe(1));
    canvas().setActiveObject(canvas().getObjects()[0]!);
    el<HTMLButtonElement>('[data-testid="drawing-delete"]')!.click();
    expect(scene().shapes()).toBe(0);
  });

  it('colours with the swatch chosen', async () => {
    await mount();

    el<HTMLButtonElement>('[data-testid="drawing-hue"][data-hue="green"]')!.click();
    await fixture.whenStable();
    await drawRectangle();

    expect(el('[data-hue="green"]')!.getAttribute('aria-checked')).toBe('true');
    expect((canvas().getObjects()[0] as Rect & { hue?: string }).hue).toBe('green');
  });

  it('exports what is drawn under the drawing’s title, as a PNG or an SVG', async () => {
    await mount();
    await drawRectangle();

    el<HTMLButtonElement>('[data-testid="drawing-export"][data-format="png"]')!.click();
    el<HTMLButtonElement>('[data-testid="drawing-export"][data-format="svg"]')!.click();
    await fixture.whenStable();

    expect(exported.map(({ title, format }) => [title, format])).toEqual([
      ['Archi', 'png'],
      ['Archi', 'svg'],
    ]);
    expect(exported[0]!.picture.startsWith('data:image/png;base64,')).toBe(true);
    expect(exported[1]!.picture).toContain('<svg');
  });

  it('opens a stored drawing, and another one given afterwards', async () => {
    const stored = JSON.stringify({
      version: 1,
      scene: { objects: [new Rect({ width: 20, height: 20 }).toObject()] },
    });
    await mount(stored);
    await vi.waitFor(() => expect(scene().shapes()).toBe(1));

    fixture.componentRef.setInput('content', '');
    await vi.waitFor(() => expect(scene().shapes()).toBe(0));

    // The body it emitted itself is not reloaded over what is drawn.
    await drawRectangle();
    fixture.componentInstance.flush();
    fixture.componentRef.setInput('content', bodies.at(-1)!);
    await fixture.whenStable();
    expect(scene().canUndo()).toBe(true);
  });

  /** DES-15: an imported drawing comes without its pictures, and opening it never took them. */
  it('takes the pictures of a drawing that has none yet, and of no other, sending no body', async () => {
    const stored = JSON.stringify({
      version: 1,
      scene: { objects: [new Rect({ width: 20, height: 20 }).toObject()] },
    });
    await mount(stored, 'sketch');
    await vi.waitFor(() => expect(asked).toEqual(['sketch']));
    await fixture.whenStable();
    fixture.componentInstance.flush();
    expect(pictures).toEqual([]);

    fixture.componentRef.setInput('noteId', 'imported');
    fixture.componentRef.setInput('content', stored.replace('"version":1', '"version": 1'));
    await vi.waitFor(() => expect(asked).toEqual(['sketch', 'imported']));
    await fixture.whenStable();
    fixture.componentInstance.flush();

    expect(pictures).toHaveLength(1);
    expect(pictures[0]!.thumbnail.startsWith('data:image/png;base64,')).toBe(true);
    expect(bodies).toEqual([]);
  });
});
