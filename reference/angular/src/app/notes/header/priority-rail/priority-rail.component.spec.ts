import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FacetCount, Priority } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { PriorityRailComponent } from './priority-rail.component';

const COUNTS: FacetCount<Priority>[] = [
  { value: 'none', count: 5 },
  { value: 'low', count: 6 },
  { value: 'medium', count: 0 },
  { value: 'high', count: 3 },
  { value: 'urgent', count: 2 },
];

describe('PriorityRailComponent', () => {
  let fixture: ComponentFixture<PriorityRailComponent>;

  function chip(level: Priority | 'all'): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="priority-chip-${level}"]`);
  }

  function rail(): HTMLElement | null {
    return fixture.nativeElement.querySelector('[data-testid="priority-rail"]');
  }

  async function show(counts: FacetCount<Priority>[], active: Priority[] = []): Promise<void> {
    fixture.componentRef.setInput('counts', counts);
    fixture.componentRef.setInput('activePriorities', new Set(active));
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [PriorityRailComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(PriorityRailComponent);
    fixture.componentRef.setInput('counts', []);
    fixture.componentRef.setInput('activePriorities', new Set());
    fixture.autoDetectChanges();
  });

  it('stays away from a space where nothing has a priority', async () => {
    await show([{ value: 'none', count: 12 }]);

    expect(rail()).toBeNull();
  });

  /** The mockup's row: "Toutes 16 · Urgente 2 · Haute 3 · Moyenne 0 · Basse 6". */
  it('counts every level from the most pressing down, and all the notes first', async () => {
    await show(COUNTS);

    const read = (button: HTMLButtonElement): string =>
      [...button.querySelectorAll('span')].map((span) => span.textContent?.trim()).join(' ');
    const chips: HTMLButtonElement[] = [...fixture.nativeElement.querySelectorAll('.priority-chip')];

    expect(chips.map(read)).toEqual(['Toutes 16', 'Urgente 2', 'Haute 3', 'Moyenne 0', 'Basse 6']);
    expect(chip('medium').disabled).toBe(true);
    expect(chip('all').getAttribute('aria-pressed')).toBe('true');
  });

  it('presses the levels chosen, and emits the one a click landed on', async () => {
    await show(COUNTS, ['urgent']);
    const toggled: Priority[] = [];
    let all = 0;
    fixture.componentInstance.priorityToggled.subscribe((level) => toggled.push(level));
    fixture.componentInstance.allChosen.subscribe(() => (all += 1));

    chip('high').click();
    chip('all').click();

    expect(chip('urgent').getAttribute('aria-pressed')).toBe('true');
    expect(chip('all').getAttribute('aria-pressed')).toBe('false');
    expect(toggled).toEqual(['high']);
    expect(all).toBe(1);
  });
});
