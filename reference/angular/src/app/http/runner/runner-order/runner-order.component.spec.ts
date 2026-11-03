import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { RunnerRow } from '@core/model/http-runner.model';
import { provideAppTesting } from '@testing/testing.providers';
import { RunnerOrderComponent } from './runner-order.component';

const row = (name: string, enabled = true, path: string[] = []): RunnerRow => ({
  id: name,
  name,
  path,
  kind: 'http',
  method: name === 'Payer' ? 'POST' : 'GET',
  enabled,
});

describe('RunnerOrderComponent', () => {
  let fixture: ComponentFixture<RunnerOrderComponent>;
  let moves: { from: number; to: number }[];
  let toggles: number[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [RunnerOrderComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(RunnerOrderComponent);
    fixture.componentRef.setInput('rows', [row('Login', true, ['Auth']), row('Payer'), row('Reçu', false)]);
    moves = [];
    toggles = [];
    fixture.componentInstance.moved.subscribe((move) => moves.push(move));
    fixture.componentInstance.toggled.subscribe((index) => toggles.push(index));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const rows = () => [
    ...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLElement>(
      '[data-testid="http-runner-row"]',
    ),
  ];
  const key = (index: number, init: KeyboardEventInit) =>
    rows()[index]!.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));

  it('says how many run, each request under its folders, one left out struck through', () => {
    expect((fixture.nativeElement as HTMLElement).querySelector('.caption')?.textContent?.trim()).toBe(
      'Ordre · 2 sur 3 requêtes',
    );
    expect(rows()[0]?.textContent).toContain('Auth ›');
    expect(rows()[2]?.classList.contains('off')).toBe(true);
  });

  it('moves a row with Alt and the arrows, and leaves one out with Space or its box', () => {
    key(1, { key: 'ArrowUp', altKey: true });
    key(1, { key: 'ArrowDown', altKey: true });
    key(0, { key: 'ArrowUp', altKey: true });
    key(2, { key: 'ArrowDown', altKey: true });
    key(1, { key: 'ArrowDown' });
    expect(moves).toEqual([
      { from: 1, to: 0 },
      { from: 1, to: 2 },
    ]);
    key(2, { key: ' ' });
    rows()[0]!.querySelector<HTMLInputElement>('[data-testid="http-runner-row-enabled"]')!.click();
    expect(toggles).toEqual([2, 0]);
  });

  it('moves a row dragged by its grip to where it is dropped', async () => {
    const grip = rows()[2]!.querySelector<HTMLElement>('[data-testid="http-runner-row-grip"]')!;
    grip.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, pointerId: 1 }));
    // jsdom lays nothing out: every row ends at 0, so the pointer is over the first.
    grip.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: -5, pointerId: 1 }));
    await fixture.whenStable();
    expect(rows()[0]?.classList.contains('landing')).toBe(true);
    grip.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 1 }));
    expect(moves).toEqual([{ from: 2, to: 0 }]);

    grip.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: 5, pointerId: 1 }));
    grip.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 2, pointerId: 1 }));
    grip.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 1 }));
    expect(moves).toHaveLength(1);
  });

  it('moves nothing while a run is under way', async () => {
    fixture.componentRef.setInput('disabled', true);
    await fixture.whenStable();
    key(1, { key: ' ' });
    const grip = rows()[1]!.querySelector<HTMLElement>('[data-testid="http-runner-row-grip"]')!;
    grip.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, pointerId: 1 }));
    expect(toggles).toEqual([]);
    expect(fixture.componentInstance['dragging']()).toBeNull();
  });
});
