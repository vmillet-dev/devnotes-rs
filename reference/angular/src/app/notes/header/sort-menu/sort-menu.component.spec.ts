import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Arrangement, DEFAULT_ARRANGEMENT } from '@core/model/arrangement.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SortMenuComponent } from './sort-menu.component';

describe('SortMenuComponent', () => {
  let fixture: ComponentFixture<SortMenuComponent>;
  let emitted: string[];

  function item(testid: string, attribute: string, value: string): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"][data-${attribute}="${value}"]`);
  }

  function checked(testid: string, attribute: string): string[] {
    return [...fixture.nativeElement.querySelectorAll(`[data-testid="${testid}"][aria-checked="true"]`)].map(
      (button: HTMLElement) => button.getAttribute(`data-${attribute}`) ?? '',
    );
  }

  async function show(arrangement: Arrangement): Promise<void> {
    fixture.componentRef.setInput('arrangement', arrangement);
    await fixture.whenStable();
  }

  async function open(): Promise<void> {
    fixture.nativeElement.querySelector('[data-testid="sort-trigger"]').click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [SortMenuComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(SortMenuComponent);
    fixture.componentRef.setInput('arrangement', DEFAULT_ARRANGEMENT);
    emitted = [];
    fixture.componentInstance.orderChosen.subscribe(({ key, direction }) =>
      emitted.push(`${key} ${direction}`),
    );
    fixture.componentInstance.groupingChosen.subscribe((grouping) => emitted.push(`group ${grouping}`));
    fixture.componentInstance.pinnedFirstChosen.subscribe((pinned) => emitted.push(`pinned ${pinned}`));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** The mockup's "Priorité ↓": the button names the order the canvas is in. */
  it('names the current order on its button, and in its tooltip', async () => {
    await show({ ...DEFAULT_ARRANGEMENT, order: { key: 'priority', direction: 'descending' } });
    const trigger: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="sort-trigger"]');

    expect(fixture.nativeElement.querySelector('[data-testid="sort-now"]').textContent.trim()).toBe(
      'Priorité ↓',
    );
    expect(trigger.title).toBe('Trier et regrouper · Priorité ↓');
  });

  it('checks the current key, direction and grouping', async () => {
    await show({ order: { key: 'title', direction: 'ascending' }, grouping: 'format', pinnedFirst: false });
    await open();

    expect(checked('sort-key', 'key')).toEqual(['title']);
    expect(checked('sort-direction', 'direction')).toEqual(['ascending']);
    expect(checked('sort-grouping', 'grouping')).toEqual(['format']);
    expect(
      fixture.nativeElement.querySelector('[data-testid="sort-pinned-first"]').getAttribute('aria-checked'),
    ).toBe('false');
  });

  /** A new key reads the way it reads first; the checked one asks for nothing. */
  it('emits a key with its natural direction, a direction on the same key, and the rest as asked', async () => {
    await open();

    item('sort-key', 'key', 'modified').click();
    item('sort-key', 'key', 'title').click();
    item('sort-direction', 'direction', 'ascending').click();
    item('sort-grouping', 'grouping', 'priority').click();
    fixture.nativeElement.querySelector('[data-testid="sort-pinned-first"]').click();

    expect(emitted).toEqual(['title ascending', 'modified ascending', 'group priority', 'pinned false']);
    expect(fixture.nativeElement.querySelector('[data-testid="sort-menu"]')).not.toBeNull();
  });
});
