import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SplitHandleComponent } from './split-handle.component';

@Component({
  imports: [SplitHandleComponent],
  template: `
    <div class="box">
      <div class="upper"></div>
      <app-split-handle
        testId="line"
        label="Hauteur de la requête"
        [orientation]="orientation()"
        [share]="share()"
        [upperFloor]="150"
        [lowerFloor]="160"
        [valueText]="share() > 0 ? share() + ' %' : 'automatique'"
        (shareChanged)="emitted.push($event); share.set($event)"
      />
      <div class="lower"></div>
    </div>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class HostComponent {
  readonly share = signal(0);
  readonly orientation = signal<'horizontal' | 'vertical'>('horizontal');
  readonly emitted: number[] = [];
}

/**
 * The space between a request and its answer could not be adjusted: sixteen headers left the
 * answer a third of the window, and a short request kept a panel a long JSON could not have.
 */
describe('SplitHandleComponent', () => {
  let fixture: ComponentFixture<HostComponent>;

  const root = () => fixture.nativeElement as HTMLElement;
  const line = () => root().querySelector<HTMLElement>('app-split-handle')!;
  const emitted = () => fixture.componentInstance.emitted;
  const key = async (name: string) => {
    const event = new KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true });
    line().dispatchEvent(event);
    await fixture.whenStable();
    return event;
  };
  /** jsdom has no `PointerEvent`: a mouse event carrying the fields the line reads. */
  const pointer = (type: string, init: MouseEventInit = {}) => {
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init });
    Object.defineProperty(event, 'pointerId', { value: 1 });
    line().dispatchEvent(event);
    return event;
  };
  const rect = (top: number, height: number) => ({ top, height }) as DOMRect;

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [HostComponent] });
    fixture = TestBed.createComponent(HostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
    // A box of 600px starting 100px down; the upper pane takes 240 of it left to itself.
    root().querySelector<HTMLElement>('.box')!.getBoundingClientRect = () => rect(100, 600);
    root().querySelector<HTMLElement>('.upper')!.getBoundingClientRect = () => rect(100, 240);
  });

  it('is a separator the keyboard reaches, and says its share in words', async () => {
    expect(line().getAttribute('role')).toBe('separator');
    expect(line().getAttribute('tabindex')).toBe('0');
    expect(line().getAttribute('aria-orientation')).toBe('horizontal');
    expect(line().getAttribute('aria-label')).toBe('Hauteur de la requête');
    expect(line().dataset['testid']).toBe('line');
    // Left to the content there is no number to give: the words say it.
    expect(line().getAttribute('aria-valuenow')).toBeNull();
    expect(line().getAttribute('aria-valuetext')).toBe('automatique');

    fixture.componentInstance.share.set(42);
    await fixture.whenStable();
    expect(line().getAttribute('aria-valuenow')).toBe('42');
    expect(line().getAttribute('aria-valuetext')).toBe('42 %');
    expect(line().dataset['share']).toBe('42');
  });

  it('moves with the arrows from where the line stands, measured while it is automatic', async () => {
    // 240 of 600 is 40%: the first nudge starts there, not from nothing.
    expect((await key('ArrowDown')).defaultPrevented).toBe(true);
    await key('ArrowDown');
    await key('ArrowUp');
    expect(emitted()).toEqual([44, 48, 44]);

    expect((await key('a')).defaultPrevented).toBe(false);
    expect(emitted()).toHaveLength(3);
  });

  it('stops at each floor: neither pane is dragged out of reach', async () => {
    // 150 of 600 above, 160 below.
    await key('Home');
    expect(emitted()).toEqual([25]);
    await key('ArrowUp');
    expect(emitted()).toEqual([25]);

    await key('End');
    expect(emitted()).toEqual([25, 73.3]);
    await key('ArrowDown');
    expect(emitted()).toEqual([25, 73.3]);
  });

  it('lets the upper floor win in a box too short for both', async () => {
    root().querySelector<HTMLElement>('.box')!.getBoundingClientRect = () => rect(100, 250);

    await key('End');

    expect(emitted()).toEqual([60]);
  });

  it('follows the pointer once it holds it, measured from the top of its box', () => {
    let held = false;
    line().setPointerCapture = () => (held = true);
    line().hasPointerCapture = () => held;
    line().releasePointerCapture = () => (held = false);

    pointer('pointermove', { clientY: 400 });
    expect(emitted()).toEqual([]);

    expect(pointer('pointerdown').defaultPrevented).toBe(true);
    pointer('pointermove', { clientY: 400 });
    pointer('pointermove', { clientY: 120 });
    pointer('pointermove', { clientY: 5000 });
    expect(emitted()).toEqual([50, 25, 73.3]);

    pointer('pointerup');
    pointer('pointermove', { clientY: 300 });
    expect(emitted()).toHaveLength(3);
  });

  it('leaves the other buttons of the mouse alone', () => {
    let held = false;
    line().setPointerCapture = () => (held = true);

    expect(pointer('pointerdown', { button: 2 }).defaultPrevented).toBe(false);
    expect(held).toBe(false);
  });

  it('goes back to automatic on a double-click, and on Enter, once', async () => {
    fixture.componentInstance.share.set(50);
    await fixture.whenStable();

    line().dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    await fixture.whenStable();
    expect(emitted()).toEqual([0]);
    // Already automatic: nothing more to say.
    line().dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    expect(emitted()).toEqual([0]);

    fixture.componentInstance.share.set(50);
    await fixture.whenStable();
    expect((await key('Enter')).defaultPrevented).toBe(true);
    expect(emitted()).toEqual([0, 0]);
  });

  it('does nothing in a box that has no height yet', async () => {
    root().querySelector<HTMLElement>('.box')!.getBoundingClientRect = () => rect(0, 0);

    await key('ArrowDown');

    expect(emitted()).toEqual([]);
  });
  it('stands between panes side by side, sharing their width, the left and right arrows moving it', async () => {
    fixture.componentInstance.orientation.set('vertical');
    await fixture.whenStable();
    const side = (left: number, width: number) => ({ left, width, top: 0, height: 10 }) as DOMRect;
    root().querySelector<HTMLElement>('.box')!.getBoundingClientRect = () => side(100, 1000);
    root().querySelector<HTMLElement>('.upper')!.getBoundingClientRect = () => side(100, 400);
    let held = false;
    line().setPointerCapture = () => (held = true);
    line().hasPointerCapture = () => held;

    expect(line().getAttribute('aria-orientation')).toBe('vertical');
    expect(line().classList).toContain('vertical');
    await key('ArrowRight');
    await key('ArrowDown');
    expect(emitted()).toEqual([44]);

    pointer('pointerdown');
    pointer('pointermove', { clientX: 600, clientY: 9999 });
    expect(emitted()).toEqual([44, 50]);
  });
});
