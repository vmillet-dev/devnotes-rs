import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DialogBackdropDirective } from './dialog-backdrop.directive';

@Component({
  selector: 'app-backdrop-host',
  imports: [DialogBackdropDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div appDialogBackdrop id="backdrop" (dismissed)="dismissed.set(dismissed() + 1)">
      <div id="panel">
        <button type="button" id="inside">inside</button>
      </div>
    </div>
  `,
})
class BackdropHostComponent {
  readonly dismissed = signal(0);
}

describe('DialogBackdropDirective', () => {
  let fixture: ComponentFixture<BackdropHostComponent>;

  function element(id: string): HTMLElement {
    return fixture.nativeElement.querySelector(`#${id}`);
  }

  function dismissals(): number {
    return fixture.componentInstance.dismissed();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [BackdropHostComponent] });
    fixture = TestBed.createComponent(BackdropHostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('dismisses when the click lands on the backdrop itself', () => {
    element('backdrop').click();

    expect(dismissals()).toBe(1);
  });

  /**
   * The whole point: a click inside the panel bubbles up to the backdrop, and taking
   * it as a dismissal would close the dialog on every button in it.
   */
  it('stays put for a click that started inside the panel', () => {
    element('panel').click();
    element('inside').click();

    expect(dismissals()).toBe(0);
  });

  it('dismisses once per click on the backdrop, not once per listener', () => {
    element('backdrop').click();
    element('backdrop').click();

    expect(dismissals()).toBe(2);
  });
});
