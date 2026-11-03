import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  SUBMENU_CLOSE_DELAY_MS,
  SUBMENU_OPEN_DELAY_MS,
  SubmenuHoverDirective,
} from './submenu-hover.directive';

@Component({
  selector: 'app-submenu-hover-host',
  imports: [SubmenuHoverDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      #hover="appSubmenuHover"
      id="panel"
      [appSubmenuHover]="open()"
      (submenuOpened)="open.set(true)"
      (submenuClosed)="closedOnto.push($event); open.set(false)"
    >
      <button type="button" appMenuItem id="pin">Épingler</button>
      <button type="button" appMenuItem id="parent" aria-haspopup="menu">
        <span id="label">Priorité</span>
      </button>
      <button type="button" appMenuItem id="copy">Copier</button>
      <p id="title">Déplacer vers</p>
    </div>
    @if (open()) {
      <div id="submenu" (pointerenter)="hover.reached()"></div>
    }
  `,
})
class SubmenuHoverHostComponent {
  readonly open = signal(false);
  readonly closedOnto: HTMLElement[] = [];
}

describe('SubmenuHoverDirective', () => {
  let fixture: ComponentFixture<SubmenuHoverHostComponent>;

  const element = (id: string): HTMLElement => fixture.nativeElement.querySelector(`#${id}`);
  const isOpen = (): boolean => fixture.componentInstance.open();

  function point(id: string, pointerType = 'mouse'): void {
    element(id).dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType }));
    fixture.detectChanges();
  }

  function wait(ms: number): void {
    vi.advanceTimersByTime(ms);
    fixture.detectChanges();
  }

  beforeEach(() => {
    // Only the timers: a faked `requestAnimationFrame` hangs Angular's zoneless scheduler.
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [SubmenuHoverHostComponent] });
    fixture = TestBed.createComponent(SubmenuHoverHostComponent);
    fixture.detectChanges();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('opens the submenu once the pointer has rested on its entry, not as it arrives', () => {
    point('label');
    wait(SUBMENU_OPEN_DELAY_MS - 1);
    expect(isOpen()).toBe(false);

    // Moving inside the entry does not start the wait over.
    point('parent');
    wait(1);
    expect(isOpen()).toBe(true);
  });

  it('opens nothing for a pointer crossing the entry on its way elsewhere', () => {
    point('parent');
    point('copy');
    wait(SUBMENU_OPEN_DELAY_MS);
    expect(isOpen()).toBe(false);

    point('parent');
    element('panel').dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' }));
    wait(SUBMENU_OPEN_DELAY_MS);
    expect(isOpen()).toBe(false);
  });

  it('folds it once another entry is hovered past the grace, onto that entry', () => {
    fixture.componentInstance.open.set(true);
    fixture.detectChanges();

    point('pin');
    // What is no entry changes nothing.
    point('title');
    wait(SUBMENU_CLOSE_DELAY_MS - 1);
    expect(isOpen()).toBe(true);

    point('copy');
    wait(1);
    expect(isOpen()).toBe(false);
    expect(fixture.componentInstance.closedOnto).toEqual([element('copy')]);
  });

  /** The diagonal from the entry to the submenu's last level crosses the entries below it. */
  it('keeps it for a pointer that reaches the submenu, or comes back to its entry, within the grace', () => {
    fixture.componentInstance.open.set(true);
    fixture.detectChanges();

    point('copy');
    element('submenu').dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' }));
    wait(SUBMENU_CLOSE_DELAY_MS);
    expect(isOpen()).toBe(true);

    point('copy');
    point('parent');
    wait(SUBMENU_CLOSE_DELAY_MS);
    expect(isOpen()).toBe(true);
  });

  it('leaves a touch to the tap: it has no hover', () => {
    point('parent', 'touch');
    wait(SUBMENU_OPEN_DELAY_MS);
    expect(isOpen()).toBe(false);
  });

  it('acts on nothing once its panel is gone', () => {
    point('parent');
    fixture.destroy();
    vi.advanceTimersByTime(SUBMENU_OPEN_DELAY_MS);
    expect(fixture.componentInstance.open()).toBe(false);
  });
});
