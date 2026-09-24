import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { MenuPanelDirective } from './menu-panel.directive';

@Component({
  selector: 'app-menu-panel-host',
  imports: [MenuPanelDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (open()) {
      <div appMenuPanel id="panel">
        <button type="button" appMenuItem id="first">Import…</button>
        <button type="button" appMenuItem id="second">Export…</button>
        <button type="button" appMenuItem id="third">Quit</button>
      </div>
    }
  `,
})
class MenuPanelHostComponent {
  readonly open = signal(false);
}

describe('MenuPanelDirective', () => {
  let fixture: ComponentFixture<MenuPanelHostComponent>;

  function element(id: string): HTMLElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  function press(key: string): void {
    element('panel').dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [MenuPanelHostComponent] });
    fixture = TestBed.createComponent(MenuPanelHostComponent);
    fixture.autoDetectChanges();
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
  });

  it('declares itself a menu to assistive technology', () => {
    expect(element('panel').getAttribute('role')).toBe('menu');
  });

  /** Load-bearing: a menu opened without focus is unreachable from the keyboard. */
  it('focuses its first entry once the panel exists', () => {
    expect(document.activeElement).toBe(element('first'));
  });

  it('walks down the entries', () => {
    press('ArrowDown');
    expect(document.activeElement).toBe(element('second'));

    press('ArrowDown');
    expect(document.activeElement).toBe(element('third'));
  });

  it('wraps around at both ends', () => {
    press('ArrowUp');
    expect(document.activeElement).toBe(element('third'));

    press('ArrowDown');
    expect(document.activeElement).toBe(element('first'));
  });

  it('jumps to the ends', () => {
    press('End');
    expect(document.activeElement).toBe(element('third'));

    press('Home');
    expect(document.activeElement).toBe(element('first'));
  });

  it('leaves other keys to whoever else wants them', () => {
    const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
    element('panel').dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    expect(document.activeElement).toBe(element('first'));
  });

  it('claims the arrows, so the page does not scroll under the menu', () => {
    const event = new KeyboardEvent('keydown', {
      key: 'ArrowDown',
      bubbles: true,
      cancelable: true,
    });
    element('panel').dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
  });
});
