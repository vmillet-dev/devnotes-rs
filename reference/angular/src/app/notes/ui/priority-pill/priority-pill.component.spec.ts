import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Priority } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { PriorityPillComponent } from './priority-pill.component';

describe('PriorityPillComponent', () => {
  let fixture: ComponentFixture<PriorityPillComponent>;

  function pill(): HTMLElement | null {
    return fixture.nativeElement.querySelector('[data-testid="priority-pill"]');
  }

  async function show(priority: Priority): Promise<void> {
    fixture.componentRef.setInput('priority', priority);
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [PriorityPillComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(PriorityPillComponent);
    fixture.componentRef.setInput('priority', 'none');
    fixture.autoDetectChanges();
  });

  it('draws nothing for a note without a priority', () => {
    expect(pill()).toBeNull();
  });

  /** The bars and the name say it; the hue only repeats them. */
  it('lights as many bars as the level says, and names it', async () => {
    const drawn: string[] = [];
    for (const level of ['low', 'medium', 'high', 'urgent'] as const) {
      await show(level);
      const lit = pill()?.querySelectorAll('.priority-bar.lit').length;
      drawn.push(`${pill()?.textContent?.trim()} ${lit}`);
    }

    expect(drawn).toEqual(['Basse 1', 'Moyenne 2', 'Haute 3', 'Urgente 3']);
    expect(pill()?.classList).toContain('is-urgent');
  });
});
