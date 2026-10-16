import {
  ChangeDetectionStrategy,
  Component,
  Directive,
  ElementRef,
  afterNextRender,
  inject,
  signal,
} from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { FocusTrapDirective } from './focus-trap.directive';

/** Content that places its own focus once drawn, as a dialog selecting its name field does. */
@Directive({ selector: '[appPlacesFocus]' })
class PlacesFocusDirective {
  constructor() {
    const element = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    afterNextRender(() => element.focus());
  }
}

/** The trap in a frame of its own, around projected content — the shape of `DialogComponent`. */
@Component({
  selector: 'app-trap-frame',
  imports: [FocusTrapDirective],
  template: '<div appFocusTrap><ng-content /></div>',
})
class TrapFrameComponent {}

/** Fields a component of its own draws in a loop, which runs after the frame it sits in. */
@Component({
  selector: 'app-late-fields',
  template: '@for (name of names; track name) {<input [id]="name" />}',
})
class LateFieldsComponent {
  protected readonly names = ['field'];
}

@Component({
  selector: 'app-focus-trap-host',
  imports: [FocusTrapDirective, PlacesFocusDirective, TrapFrameComponent, LateFieldsComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <button type="button" id="outside">outside</button>
    @if (open()) {
      <div appFocusTrap>
        <button type="button" id="first">first</button>
        <button type="button" id="middle" disabled>disabled</button>
        <button type="button" id="last">last</button>
        <button type="button" id="untabbable" tabindex="-1">out of the order</button>
      </div>
    }
    @if (late()) {
      <app-trap-frame>
        <app-late-fields />
        <button type="button" id="button">button</button>
        @if (placed()) {
          <input id="placed" appPlacesFocus />
        }
      </app-trap-frame>
    }
  `,
})
class FocusTrapHostComponent {
  readonly open = signal(false);
  readonly late = signal(false);
  readonly placed = signal(false);
}

describe('FocusTrapDirective', () => {
  let fixture: ComponentFixture<FocusTrapHostComponent>;

  function element(id: string): HTMLElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  function pressTab(shift: boolean): KeyboardEvent {
    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey: shift,
      bubbles: true,
      cancelable: true,
    });
    (document.activeElement ?? document.body).dispatchEvent(event);
    return event;
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [FocusTrapHostComponent] });
    fixture = TestBed.createComponent(FocusTrapHostComponent);
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  it('moves focus into the trap when it appears', async () => {
    element('outside').focus();

    fixture.componentInstance.open.set(true);
    await fixture.whenStable();

    expect(document.activeElement).toBe(element('first'));
  });

  it('waits for content drawn by another component before choosing where focus goes', async () => {
    fixture.componentInstance.late.set(true);
    await fixture.whenStable();

    expect(document.activeElement).toBe(element('field'));
  });

  it('leaves alone a focus the content already placed inside', async () => {
    const host = fixture.componentInstance;
    host.placed.set(true);
    host.late.set(true);
    await fixture.whenStable();

    expect(document.activeElement).toBe(element('placed'));
  });

  it('wraps forward from the last focusable element to the first', async () => {
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    element('last').focus();

    const event = pressTab(false);

    expect(document.activeElement).toBe(element('first'));
    expect(event.defaultPrevented).toBe(true);
  });

  it('wraps backward from the first focusable element to the last', async () => {
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    element('first').focus();

    const event = pressTab(true);

    expect(document.activeElement).toBe(element('last'));
    expect(event.defaultPrevented).toBe(true);
  });

  it('leaves navigation alone in the middle of the trap', async () => {
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    element('first').focus();

    const event = pressTab(false);

    expect(event.defaultPrevented).toBe(false);
  });

  /** `tabindex="-1"` takes any element out of the tab order, a button included. */
  it('treats an untabbable control as out of the trap, button or not', async () => {
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    element('first').focus();

    pressTab(true);

    expect(document.activeElement).toBe(element('last'));
  });

  /** Something inside answered for the key; wrapping on top of that undoes it. */
  it('leaves a Tab a descendant has already handled alone', async () => {
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    element('last').focus();

    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });
    event.preventDefault();
    element('last').dispatchEvent(event);

    expect(document.activeElement).toBe(element('last'));
  });

  it('restores focus to the previously focused element when destroyed', async () => {
    element('outside').focus();
    fixture.componentInstance.open.set(true);
    await fixture.whenStable();
    expect(document.activeElement).toBe(element('first'));

    fixture.componentInstance.open.set(false);
    await fixture.whenStable();

    expect(document.activeElement).toBe(element('outside'));
  });
});
