import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DialogComponent } from './dialog.component';

/** Two dialogs on two rungs: only one of them answers Escape. */
@Component({
  selector: 'app-dialog-host',
  imports: [DialogComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (backOpen()) {
      <app-dialog
        [layer]="'editor'"
        [dismissible]="dismissible()"
        [fullscreen]="fullscreen()"
        labelledBy="back-title"
        (dismissedAt)="clickedAt = $event"
        (closed)="backClosed = backClosed + 1"
      >
        <h2 id="back-title">back</h2>
        <button type="button" id="back-button">back</button>
      </app-dialog>
    }
    @if (frontOpen()) {
      <app-dialog [layer]="'zoom'" [variant]="'bare'" label="front" (closed)="frontClosed = frontClosed + 1">
        <button type="button" id="front-button">front</button>
      </app-dialog>
    }
  `,
})
class DialogHostComponent {
  readonly backOpen = signal(true);
  readonly frontOpen = signal(false);
  readonly dismissible = signal(true);
  readonly fullscreen = signal(false);

  backClosed = 0;
  frontClosed = 0;
  clickedAt: MouseEvent | null = null;
}

describe('DialogComponent', () => {
  let fixture: ComponentFixture<DialogHostComponent>;
  let host: DialogHostComponent;

  function root(): HTMLElement {
    return fixture.nativeElement as HTMLElement;
  }

  function panels(): HTMLElement[] {
    return Array.from(root().querySelectorAll<HTMLElement>('.dialog-panel'));
  }

  function backdrops(): HTMLElement[] {
    return Array.from(root().querySelectorAll<HTMLElement>('.dialog-backdrop'));
  }

  async function pressEscape(): Promise<void> {
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [DialogHostComponent] });
    fixture = TestBed.createComponent(DialogHostComponent);
    host = fixture.componentInstance;
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('frames its content as a modal dialog', () => {
    const [panel] = panels();

    expect(panel?.getAttribute('role')).toBe('dialog');
    expect(panel?.getAttribute('aria-modal')).toBe('true');
    expect(panel?.getAttribute('aria-labelledby')).toBe('back-title');
  });

  // The backdrop needs the class too: it carries the offset a full-height panel cannot
  // afford to have taken off its height.
  it('marks the panel and the backdrop when it fills the window', async () => {
    expect(panels()[0]?.classList.contains('fullscreen')).toBe(false);
    expect(backdrops()[0]?.classList.contains('fullscreen')).toBe(false);

    host.fullscreen.set(true);
    await fixture.whenStable();

    expect(panels()[0]?.classList.contains('fullscreen')).toBe(true);
    expect(backdrops()[0]?.classList.contains('fullscreen')).toBe(true);
  });

  it('takes focus, so the keyboard cannot walk behind the modal', () => {
    expect(document.activeElement?.id).toBe('back-button');
  });

  it('draws each rung above the one before it', async () => {
    host.frontOpen.set(true);
    await fixture.whenStable();

    const [back, front] = backdrops().map((backdrop) => Number(backdrop.style.zIndex));

    expect(front).toBeGreaterThan(back ?? 0);
  });

  it('closes on Escape', async () => {
    await pressEscape();

    expect(host.backClosed).toBe(1);
  });

  it('closes on a click that lands on the backdrop and not on the panel, and says where', async () => {
    backdrops()[0]?.dispatchEvent(new MouseEvent('click', { clientX: 40, clientY: 50, bubbles: true }));
    await fixture.whenStable();
    expect(host.backClosed).toBe(1);
    expect([host.clickedAt?.clientX, host.clickedAt?.clientY]).toEqual([40, 50]);

    panels()[0]?.click();
    await fixture.whenStable();
    expect(host.backClosed).toBe(1);
  });

  it('gives Escape to the dialog in front, and to it alone', async () => {
    host.frontOpen.set(true);
    await fixture.whenStable();

    await pressEscape();

    expect(host.frontClosed).toBe(1);
    expect(host.backClosed).toBe(0);
  });

  it('hands Escape back when the dialog in front has gone', async () => {
    host.frontOpen.set(true);
    await fixture.whenStable();
    host.frontOpen.set(false);
    await fixture.whenStable();

    await pressEscape();

    expect(host.backClosed).toBe(1);
  });

  it('refuses both ways out while it is not dismissible', async () => {
    host.dismissible.set(false);
    await fixture.whenStable();

    await pressEscape();
    backdrops()[0]?.click();
    await fixture.whenStable();

    expect(host.backClosed).toBe(0);
  });
});
