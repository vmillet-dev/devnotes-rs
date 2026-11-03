import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RailFrameComponent } from './rail-frame.component';

const BOUNDS = { min: 160, max: 520 };

describe('RailFrameComponent', () => {
  let fixture: ComponentFixture<RailFrameComponent>;
  let widths: number[];

  const host = () => fixture.nativeElement as HTMLElement;
  const edge = () => host().querySelector<HTMLElement>('[role="slider"]')!;
  const key = (name: string) => {
    const event = new KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true });
    edge().dispatchEvent(event);
    return event;
  };
  /** jsdom has no `PointerEvent`: a mouse event carrying the fields the frame reads. */
  const pointer = (type: string, init: MouseEventInit = {}) => {
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init });
    Object.defineProperty(event, 'pointerId', { value: 1 });
    edge().dispatchEvent(event);
    return event;
  };

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [RailFrameComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(RailFrameComponent);
    fixture.componentRef.setInput('width', 200);
    fixture.componentRef.setInput('bounds', BOUNDS);
    widths = [];
    fixture.componentInstance.widthChanged.subscribe((width) => widths.push(width));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('is as wide as it is told, and says so on its edge', () => {
    expect(host().style.width).toBe('200px');
    expect(edge().getAttribute('aria-valuenow')).toBe('200');
    expect(edge().getAttribute('aria-valuemin')).toBe('160');
    expect(edge().getAttribute('aria-valuemax')).toBe('520');
    expect(edge().getAttribute('aria-label')).toBe('Largeur de la barre latérale');
  });

  /** The arrow keys are not optional: the drag is pointer events, which no keyboard has. */
  it('nudges its width from the edge with the arrows, and takes no other key', () => {
    expect(key('ArrowRight').defaultPrevented).toBe(true);
    key('ArrowLeft');
    expect(widths).toEqual([216, 184]);

    expect(key('ArrowUp').defaultPrevented).toBe(false);
    expect(widths).toHaveLength(2);
  });

  it('stops at its bounds, and says nothing when the width would not change', async () => {
    fixture.componentRef.setInput('width', 160);
    await fixture.whenStable();
    key('ArrowLeft');
    expect(widths).toEqual([]);

    fixture.componentRef.setInput('width', 510);
    await fixture.whenStable();
    key('ArrowRight');
    expect(widths).toEqual([520]);
  });

  it('follows the pointer once it holds it, measured from its own left edge', () => {
    host().getBoundingClientRect = () => ({ left: 40 }) as DOMRect;
    let held = false;
    edge().setPointerCapture = () => (held = true);
    edge().hasPointerCapture = () => held;
    edge().releasePointerCapture = () => (held = false);

    pointer('pointermove', { clientX: 300 });
    expect(widths).toEqual([]);

    expect(pointer('pointerdown').defaultPrevented).toBe(true);
    pointer('pointermove', { clientX: 300 });
    pointer('pointermove', { clientX: 900 });
    expect(widths).toEqual([260, 520]);

    pointer('pointerup');
    pointer('pointermove', { clientX: 240 });
    expect(widths).toHaveLength(2);
  });

  it('leaves the other buttons of the mouse alone', () => {
    let held = false;
    edge().setPointerCapture = () => (held = true);

    expect(pointer('pointerdown', { button: 2 }).defaultPrevented).toBe(false);
    expect(held).toBe(false);
  });

  /** The notes' rail stays in the document behind the other areas: one test id each. */
  it('names its edge after the page that draws it', async () => {
    expect(edge().dataset['testid']).toBe('rail-edge');

    fixture.componentRef.setInput('edgeTestId', 'http-rail-edge');
    await fixture.whenStable();

    expect(edge().dataset['testid']).toBe('http-rail-edge');
  });
});
