import { ChangeDetectionStrategy, Component, viewChild } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FloatingPanelDirective } from './floating-panel.directive';
import { MenuPanelDirective } from './menu-panel.directive';
import { MenuTriggerDirective } from './menu-trigger.directive';

@Component({
  selector: 'app-floating-panel-host',
  imports: [FloatingPanelDirective, MenuPanelDirective, MenuTriggerDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div id="elsewhere"></div>
    <div id="scroller">
      <div appMenuTrigger #menu="appMenu">
        <button type="button" appMenuAnchor id="anchor" (click)="menu.toggle()">Type</button>
        @if (menu.open()) {
          <div appMenuPanel appFloatingPanel id="panel">
            <button type="button" appMenuItem id="entry">Bearer</button>
          </div>
        }
      </div>
    </div>
  `,
})
class FloatingPanelHostComponent {
  readonly menu = viewChild.required(MenuTriggerDirective);
}

/** jsdom lays nothing out: the anchor's rectangle and the panel's size are told to it. */
const PANEL = { width: 180, height: 200 };

/** On the document's own window: the test's global one is a stand-in for it. */
function resizeWindow(width: number, height: number): void {
  const view = document.defaultView!;
  Object.defineProperty(view, 'innerWidth', { configurable: true, value: width });
  Object.defineProperty(view, 'innerHeight', { configurable: true, value: height });
}

describe('FloatingPanelDirective', () => {
  let fixture: ComponentFixture<FloatingPanelHostComponent>;
  let shown: HTMLElement[];

  function element(id: string): HTMLElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  function panel(): HTMLElement | null {
    return fixture.nativeElement.querySelector('#panel');
  }

  async function openWithAnchorAt(box: Partial<DOMRect>): Promise<HTMLElement> {
    element('anchor').getBoundingClientRect = () => ({ left: 0, top: 0, bottom: 0, ...box }) as DOMRect;
    element('anchor').click();
    await fixture.whenStable();
    return panel()!;
  }

  beforeEach(async () => {
    shown = [];
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
      return this.id === 'panel' ? PANEL.height : 0;
    });
    vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockImplementation(function (this: HTMLElement) {
      return this.id === 'panel' ? PANEL.width : 0;
    });
    HTMLElement.prototype.showPopover = function (this: HTMLElement) {
      shown.push(this);
    };
    resizeWindow(1024, 768);

    TestBed.configureTestingModule({ imports: [FloatingPanelHostComponent] });
    fixture = TestBed.createComponent(FloatingPanelHostComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    delete (HTMLElement.prototype as Partial<HTMLElement>).showPopover;
    resizeWindow(1024, 768);
    vi.restoreAllMocks();
  });

  it('opens in the top layer, under its anchor, with the first entry focused', async () => {
    const opened = await openWithAnchorAt({ left: 40, top: 100, bottom: 124 });

    expect(opened.getAttribute('popover')).toBe('manual');
    expect(shown).toEqual([opened]);
    expect(opened.dataset['placement']).toBe('below');
    expect(opened.style.top).toBe('128px');
    expect(opened.style.left).toBe('40px');
    expect(opened.style.maxHeight).toBe('');
    expect(document.activeElement).toBe(element('entry'));
  });

  it('opens above its anchor when the room under it is short', async () => {
    const opened = await openWithAnchorAt({ left: 40, top: 676, bottom: 700 });

    expect(opened.dataset['placement']).toBe('above');
    expect(opened.style.bottom).toBe('96px');
    expect(opened.style.top).toBe('');
  });

  it('takes the larger side and scrolls inside it when neither holds it whole', async () => {
    resizeWindow(1024, 300);

    const opened = await openWithAnchorAt({ left: 40, top: 140, bottom: 164 });

    expect(opened.dataset['placement']).toBe('above');
    expect(opened.style.maxHeight).toBe('128px');
  });

  it('stays inside the window on the right, and off its left edge', async () => {
    const right = await openWithAnchorAt({ left: 980, top: 100, bottom: 124 });
    expect(right.style.left).toBe('836px');

    fixture.componentInstance.menu().close();
    await fixture.whenStable();
    resizeWindow(150, 768);

    const narrow = await openWithAnchorAt({ left: 2, top: 100, bottom: 124 });
    expect(narrow.style.left).toBe('8px');
  });

  it('closes when what holds its anchor scrolls, and gives the focus back without a jump', async () => {
    await openWithAnchorAt({ left: 40, top: 100, bottom: 124 });
    const focus = vi.spyOn(element('anchor'), 'focus');

    element('scroller').dispatchEvent(new Event('scroll'));
    await fixture.whenStable();

    expect(panel()).toBeNull();
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  });

  it('stays open while its own entries scroll, or a list elsewhere does', async () => {
    const opened = await openWithAnchorAt({ left: 40, top: 100, bottom: 124 });

    opened.dispatchEvent(new Event('scroll'));
    element('elsewhere').dispatchEvent(new Event('scroll'));
    await fixture.whenStable();

    expect(panel()).not.toBeNull();
  });

  it('closes when the window is resized, and stops listening once gone', async () => {
    await openWithAnchorAt({ left: 40, top: 100, bottom: 124 });
    const menu = fixture.componentInstance.menu();
    const close = vi.spyOn(menu, 'close');

    document.defaultView!.dispatchEvent(new Event('resize'));
    await fixture.whenStable();
    expect(panel()).toBeNull();
    expect(close).toHaveBeenCalledTimes(1);

    document.defaultView!.dispatchEvent(new Event('resize'));
    document.dispatchEvent(new Event('scroll'));
    expect(close).toHaveBeenCalledTimes(1);
  });

  it('is placed all the same where the top layer does not exist', async () => {
    delete (HTMLElement.prototype as Partial<HTMLElement>).showPopover;

    const opened = await openWithAnchorAt({ left: 40, top: 100, bottom: 124 });

    expect(opened.style.top).toBe('128px');
  });
});
